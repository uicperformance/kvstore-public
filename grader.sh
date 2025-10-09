#!/usr/bin/env bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
EXIT_CODE=$(< /dev/urandom tr -dc '[:alpha:]' | head -c10)
ANSI_BRED='\033[1;31m'
ANSI_RES='\033[0m'

early_exit() {
	echo "Error. Cannot grade your assignment" >&2
}

trap 'early_exit' EXIT

echo -e "\033[0;32mPlease wait patiently while the autograder runs. It may take some time and not print anything.\033[0m"

SERVER_PORT=$(comm -23 <(seq 1024 65535 | sort) <(ss -Htan | awk '{print $4}' | sed 's/.*://;s/\[.*\]://' | sort -u) | shuf -n 1)

cd "$SCRIPT_DIR" >/dev/null

cargo r -r --bin server -- -a "127.0.0.1:$SERVER_PORT" -e "$EXIT_CODE" --memonly --singlethread true > /dev/null 2>&1 &
SERVER_PID=$!

sleep 0.5

if ! timeout --signal=KILL --kill-after=0s 1m cargo r -r --bin benchmark -- --rw-ratio 100 -a "127.0.0.1:$SERVER_PORT" -e "$EXIT_CODE" --submission-benchmark --threads 1 --connections 1 --ops 5000000 --batch-size 1000 --prepopulate 10000 --key-range 1000 --value-size 128 > /dev/null 2>&1; then
	set +e
	trap - EXIT

	kill -9 $SERVER_PID
	echo -e "${ANSI_BRED}ERROR${ANSI_RES}: Assigned zero score because the benchmark program did not complete within the allowed time of one minute."
	echo "Total Score: 0%"
	exit 1
fi

EXPECTED_THROUGHPUT="27965.0"

server_rs_checksum=$(sha256sum src/bin/server.rs | awk '{print $1}')
benchmarks_numbers=$(awk 'NR==1{print $1, $2, $3}' .benchmarks)
full_checksum=$(echo -n "$server_rs_checksum$benchmarks_numbers" | sha256sum | awk '{print $1}')
expected_checksum=$(awk 'NR==1{print $4}' .benchmarks)
[ "$full_checksum" = "$expected_checksum" ] && CHECKSUM_GATE1_PASSED=1

POINTS_TARGET_100=1900000
POINTS_TARGET_80=1750000
POINTS_TARGET_60=1500000

throughput_score_num=0
PRODUCED_THROUGHPUT=$(awk 'NR==1{print $3}' .benchmarks)

if [ $( echo "$PRODUCED_THROUGHPUT >= $POINTS_TARGET_60" | bc ) = 1 ]; then
	throughput_score_num=60
fi

if [ $( echo "$PRODUCED_THROUGHPUT >= $POINTS_TARGET_80" | bc ) = 1 ]; then
	throughput_score_num=80
fi

if [ $( echo "$PRODUCED_THROUGHPUT >= $POINTS_TARGET_100" | bc ) = 1 ]; then
	throughput_score_num=100
fi

if [ -z "$CHECKSUM_GATE1_PASSED" ] || [ "$CHECKSUM_GATE1_PASSED" -ne 1 ]; then
	throughput_score_num=0
fi

total_score="$throughput_score_num"

echo "Total Score: ${throughput_score_num}%"
cat <<EOF
You get points on this assignment based on your observed throughput (given below in millions of operations per second) as follows:
 - Above 1.9: 100%
 - [1.75,1.9): 80%
 - [1.5-1.75): 60%
 - Below 1.5: 0%
Your benchmark program produced a value of ${PRODUCED_THROUGHPUT}.
EOF

HW_SUBMISSION_ZIP="${USER}_hw-submission.zip"
zip -r "$HW_SUBMISSION_ZIP" . -x 'target/*' '*.zip' 'grader.sh' > /dev/null 2>&1
echo "Created zip file for homework submission: $HW_SUBMISSION_ZIP"

trap - EXIT

