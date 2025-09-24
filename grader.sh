#!/usr/bin/env bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
HOME_DIR_DB_FILE=$(mktemp -p "$HOME")
HOME_DIR_LOG_FILE=$(mktemp -p "$HOME")
EXIT_CODE=$(< /dev/urandom tr -dc '[:alpha:]' | head -c10)
LOG_READ_P=0
DB_FILE_BYTES_P=0
ANSI_BRED='\033[1;31m'
ANSI_RES='\033[0m'


early_exit() {
	echo "Error. Cannot grade your assignment" >&2

	rm -f "$HOME_DIR_DB_FILE" "$HOME_DIR_LOG_FILE"
}
trap 'early_exit' EXIT

echo -e "\033[0;32mPlease wait patiently while the autograder runs. It may take some time and not print anything.\033[0m"

SERVER_PORT=$(comm -23 <(seq 1024 65535 | sort) <(ss -Htan | awk '{print $4}' | sed 's/.*://;s/\[.*\]://' | sort -u) | shuf -n 1)

cd "$SCRIPT_DIR" >/dev/null


rm -f "$HOME_DIR_DB_FILE"
TEST_KEY="$RANDOM$RANDOM"
TEST_VAL="$RANDOM$RANDOM"
echo "SET $TEST_KEY $TEST_VAL" > "$HOME_DIR_LOG_FILE"
cargo r -r --bin server -- -a "127.0.0.1:$SERVER_PORT" -d "$HOME_DIR_DB_FILE" -l "$HOME_DIR_LOG_FILE" -e "$EXIT_CODE" > /dev/null 2>&1 &
sleep 0.5
SERVER_RESPONSE=$(nc -N 127.0.0.1 "$SERVER_PORT" <<EOF | awk '{print $2}' | tr -d '\r'
GET $TEST_KEY
ENDBATCH
EOF
)

if [ "$SERVER_RESPONSE" = "$TEST_VAL" ]; then
	LOG_READ_P=1
fi

nc -N 127.0.0.1 "$SERVER_PORT" <<EOF
EXIT $EXIT_CODE
EOF

rm -f "$HOME_DIR_DB_FILE"
cargo r -r --bin server -- -a "127.0.0.1:$SERVER_PORT" -d "$HOME_DIR_DB_FILE" -e "$EXIT_CODE" --snapshot-interval 100 > /dev/null 2>&1 &
SERVER_PID=$!

sleep 0.5

cargo r -r --bin benchmark -- -a "127.0.0.1:$SERVER_PORT" -e "$EXIT_CODE" --submission-benchmark --ops 10000 --batch-size 10 --threads 5 > /dev/null 2>&1

DB_FILE_BYTES=$(wc -c "$HOME_DIR_DB_FILE" | awk '{print $1}')
if [ -n "$DB_FILE_BYTES" ] && [ "$DB_FILE_BYTES" -ge 400000 ]; then
	DB_FILE_BYTES_P=1
fi

P_BENCHMARK_VAL="$LOG_READ_P$DB_FILE_BYTES_P$(sha256sum <<< "$LOG_READ_P$DB_FILE_BYTES_P" | awk '{print $1}')"
echo "$P_BENCHMARK_VAL" >> .benchmarks


EXPECTED_DURATION="1.77015" 
EXPECTED_THROUGHPUT="27965.0"

server_rs_checksum=$(sha256sum src/bin/server.rs | awk '{print $1}')
benchmarks_numbers=$(awk 'NR==1{print $1, $2, $3}' .benchmarks)
full_checksum=$(echo -n "$server_rs_checksum$benchmarks_numbers" | sha256sum | awk '{print $1}')
expected_checksum=$(awk 'NR==1{print $4}' .benchmarks)
[ "$full_checksum" = "$expected_checksum" ] && CHECKSUM_GATE1_PASSED=1

duration_score_num=25
PRODUCED_DURATION=$(awk 'NR==1{print $2}' .benchmarks)
if [ $( echo "$PRODUCED_DURATION < $EXPECTED_DURATION" | bc ) -ne 1 ]; then
	duration_score_num=0
	duration_explain="$(printf "\tYour benchmark program produced a duration of $PRODUCED_DURATION. ${ANSI_BRED}Expected a value less than${ANSI_RES}: $EXPECTED_DURATION")\n"
fi

throughput_score_num=25
PRODUCED_THROUGHPUT=$(awk 'NR==1{print $3}' .benchmarks)
if [ $( echo "$PRODUCED_THROUGHPUT > $EXPECTED_THROUGHPUT" | bc ) -ne 1 ]; then
	throughput_score_num=0
	throughput_explain="$(printf "\tYour benchmark program produced a throughput of $PRODUCED_THROUGHPUT. ${ANSI_BRED}Expected a value greater than${ANSI_RES}: $EXPECTED_THROUGHPUT")\n"
fi

read_score_num=25
if [ -z "$LOG_READ_P" ] || [ "$LOG_READ_P" -ne 1 ]; then
	read_score_num=0
	read_explain="$(printf "\t${ANSI_BRED}Your server program should read from the write-ahead log on startup${ANSI_RES}")\n"
fi

produced_db_num=25
if [ -z "$DB_FILE_BYTES_P" ] || [ "$DB_FILE_BYTES_P" -ne 1 ]; then
	produced_db_num=0
	produced_db_explain="$(printf "\t${ANSI_BRED}Your server program should write to the db file every snapshot interval${ANSI_RES}")\n"
fi

if [ -z "$CHECKSUM_GATE1_PASSED" ] || [ "$CHECKSUM_GATE1_PASSED" -ne 1 ]; then
	duration_score_num=0
	throughput_score_num=0
	read_score_num=0
	produced_db_num=0
fi

total_score=$[ $duration_score_num + $throughput_score_num + $read_score_num + $produced_db_num ]

echo "Duration score: $duration_score_num"
echo -ne "$duration_explain"
echo "Throughput score: $throughput_score_num"
echo -ne "$throughput_explain"
echo "Read from log score: $read_score_num"
echo -ne "$read_explain"
echo "Produced Valid DB score: $produced_db_num"
echo -ne "$produced_db_explain"
echo -e "\nTotal score: $total_score/100"

rm -f "$HOME_DIR_DB_FILE" "$HOME_DIR_LOG_FILE"

trap - EXIT

