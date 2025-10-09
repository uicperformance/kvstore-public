use clap::Parser;
use hdrhistogram::*;
use rand::{Rng, distributions::Alphanumeric};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

/// A high-performance benchmarking client for a key-value store.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// IP address and port of the kvstore server
    #[arg(short, long, default_value = "127.0.0.1:4000")]
    pub addr: String,

    /// Number of client threads to spawn
    #[arg(long, default_value_t = 1)]
    pub threads: usize,

    /// Number of TCP connections per client thread
    #[arg(long, default_value_t = 1)]
    pub connections: usize,

    /// Number of operations to run per thread
    #[arg(long, default_value_t = 5000000)]
    pub ops: usize,

    /// Max duration in seconds
    #[arg(long, default_value = None)]
    pub max_duration: Option<usize>,

    /// Batch size: max number of outstanding requests per connection (pipelining)
    #[arg(long, default_value_t = 1000)]
    pub batch_size: usize,

    /// Number of keys to pre-populate randomly before the benchmark
    #[arg(long, default_value_t = 10000)]
    pub prepopulate: usize,

    /// The size of the key range for the workload (keys will be `key[0..key_range]`)
    #[arg(long, default_value_t = 1000)]
    pub key_range: usize,

    /// The size of the value in bytes for SET operations
    #[arg(long, default_value_t = 128)]
    pub value_size: usize,

    /// Read/Write ratio (e.g., 90 means 90% GET, 10% SET)
    #[arg(long, default_value_t = 50)]
    pub rw_ratio: u8,

    /// Pass this code to the server EXIT command to have the server exit
    #[arg(short, long, default_value = "")]
    pub exit_code: String,

    /// Delete this file before running benchmark workload. This may be used to synchronize profiling tools.
    #[arg(short, long, default_value = None)]
    pub sleeperfile: Option<String>,

    /// Fixes all other arguments to defaults. Should be run directly before submission!
    #[arg(long)]
    pub submission_benchmark: bool,
}

// Default impl just chooses all clap default argument values with the exception of `submission_benchmark`
impl Default for Args {
    fn default() -> Self {
        let mut ret = <Self as Parser>::parse_from(["benchmark"]);
        ret.submission_benchmark = true;
        ret
    }
}

/// Represents a single connection with its buffered reader.
struct Connection {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Connection {
    /// Establishes a new connection to the server.
    fn new(addr: &str) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Connection { stream, reader })
    }

    fn send(&mut self, request: String) -> Result<(), std::io::Error> {
        self.stream.write_all(request.as_bytes())?;
        let mut resp_buf = String::new();
        self.reader.read_line(&mut resp_buf)?;
        Ok(())
    }

    /// Sends a batch of requests and reads back the responses.
    fn send_batch(&mut self, requests: &[String]) -> Result<usize, std::io::Error> {
        // Write all requests to the stream (pipelining)
        let mut buf = vec![];
        for req in requests {
            buf.extend_from_slice(req.as_bytes());
        }
        self.stream.write_all(&buf[..])?;
        self.stream.write_all("ENDBATCH\r\n".as_bytes())?;

        // Read all responses from the stream
        let mut resp_buf = String::new();
        for i in 0..requests.len() {
            if self.reader.read_line(&mut resp_buf)? > 0 {
                resp_buf.clear(); // Clear buffer for next read
            } else {
                return Ok(i);
            }
        }
        Ok(requests.len())
    }

    fn exit_server(addr: &str, code: &str) {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .write_all(format!("EXIT {code}\r\n").as_bytes())
            .unwrap();
    }
}

pub fn bench(args: Arc<Args>) -> (usize, Histogram<u64>, std::time::Duration) {
    // --- Benchmarking Phase ---
    eprintln!(
        "Starting benchmark with {} threads ({} ops/thread)...",
        args.threads, args.ops
    );
    let start = Instant::now();
    let mut handles = vec![];

    for _ in 0..args.threads {
        let args_clone = Arc::clone(&args);
        handles.push(thread::spawn(move || run_client_thread(args_clone)));
    }

    let mut total_ops = 0;
    let mut total_histo = Histogram::new(4).unwrap();

    for handle in handles {
        let (ops, histo) = handle.join().expect("Thread panicked");
        total_ops += ops;
        let _ = total_histo.add(histo);
    }

    let duration = start.elapsed();

    if !args.exit_code.is_empty() {
        Connection::exit_server(&args.addr, &args.exit_code);
    }

    (total_ops, total_histo, duration)
}

/// Logic for a single client thread.
fn run_client_thread(args: Arc<Args>) -> (usize, Histogram<u64>) {
    let mut rng = rand::thread_rng();
    let mut histo = Histogram::new(4).unwrap();

    if args.connections == 0 {
        return (0, histo); // Avoid division by zero if no connections are specified
    }

    let mut connections: Vec<_> = (0..args.connections)
        .map(|_| Connection::new(&args.addr).expect("Failed to connect"))
        .collect();

    let mut requests_batch = Vec::with_capacity(args.batch_size);
    let mut ops_done = 0usize;

    // We cannot use .cycle() on a mutable iterator.
    // Instead, we'll cycle through connections manually using an index.
    let mut conn_index = 0;
    let conn_count = connections.len();

    let value_template = "a".repeat(args.value_size);
    let start_time = std::time::Instant::now();
    while ops_done < args.ops {
        if let Some(duration) = args.max_duration {
            if start_time.elapsed().as_secs() as usize > duration {
                eprintln!("Thread reached maximum duration, exiting.\n");
                return (ops_done, histo);
            }
        }
        // Get the next connection mutably from the vector
        let connection = &mut connections[conn_index];
        conn_index = (conn_index + 1) % conn_count; // Manually cycle to the next index

        requests_batch.clear();

        // Create a batch of requests, ensuring we don't exceed the total ops target
        let remaining_ops = args.ops - ops_done;
        let current_batch_size = (args.batch_size as usize).min(remaining_ops) as usize;

        if current_batch_size == 0 {
            break; // No more operations to perform
        }

        for _ in 0..current_batch_size {
            let key = format!("key{}", rng.gen_range(0..args.key_range));
            let req = if rng.gen_range(0..100) < args.rw_ratio {
                // GET operation
                format!("GET {}\r\n", key)
            } else {
                // SET operation
                format!("SET {} {}\r\n", key, value_template)
            };
            requests_batch.push(req);
        }
        // Send the batch and read responses
        if !requests_batch.is_empty() {
            let batch_start = std::time::Instant::now();
            match connection.send_batch(&requests_batch) {
                Ok(did) => {
                    ops_done += did;
                    let _ = histo.record(batch_start.elapsed().as_micros() as u64);
                }
                Err(_e) => {
                    // Attempt to replace the failed connection
                    *connection = Connection::new(&args.addr).expect("Reconnect failed");
                }
            }
        }
    }

    println!("Thread finished.");
    (ops_done, histo)
}

/// Pre-populates the kvstore with random data.
pub fn prepopulate_keys(args: &Arc<Args>) -> Result<(), std::io::Error> {
    if args.batch_size == 0 {
        return Ok(());
    }
    let mut rng = rand::thread_rng();
    let mut requests_batch = Vec::with_capacity(args.batch_size);

    let mut conn = Connection::new(&args.addr)?;
    // make sure the database is empty before the next benchmark
    conn.send("CLEAR\r\n".to_owned())?;

    for i in 0..args.prepopulate {
        let key = format!("key{}", rng.gen_range(0..args.key_range));
        let value: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(args.value_size)
            .map(char::from)
            .collect();
        requests_batch.push(format!("SET {} {}\r\n", key, value));

        if requests_batch.len() >= args.batch_size || i == args.prepopulate - 1 {
            if !requests_batch.is_empty() {
                conn.send_batch(&requests_batch)?;
                requests_batch.clear();
            }
        }
    }
    Ok(())
}
