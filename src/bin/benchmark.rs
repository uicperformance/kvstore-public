use clap::Parser;
use rand::{distributions::Alphanumeric, Rng};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use hdrhistogram::Histogram;

/// A high-performance benchmarking client for a key-value store.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address and port of the kvstore server
    #[arg(short, long, default_value = "127.0.0.1:4000")]
    addr: String,

    /// Number of client threads to spawn
    #[arg(long, default_value_t = 10)]
    threads: u32,

    /// Number of TCP connections per client thread
    #[arg(long, default_value_t = 1)]
    connections: u32,

    /// Number of operations to run per thread
    #[arg(long, default_value_t = 1000)]
    ops: usize,

    /// Batch size: max number of outstanding requests per connection (pipelining)
    #[arg(long, default_value_t = 100)]
    batch_size: usize,

    /// Number of keys to pre-populate randomly before the benchmark
    #[arg(long, default_value_t = 10)]
    prepopulate: usize,

    /// The size of the key range for the workload (keys will be `key[0..key_range]`)
    #[arg(long, default_value_t = 1000000000)]
    key_range: usize,

    /// The size of the value in bytes for SET operations
    #[arg(long, default_value_t = 128)]
    value_size: usize,

    /// Read/Write ratio (e.g., 90 means 90% GET, 10% SET)
    #[arg(long, default_value_t = 50)]
    rw_ratio: u8,

    // Pass this code to the server EXIT command to have the server exit
    #[arg(short, long, default_value = "")]
    exit_code: String,

    // Delete this file before running benchmark workload. This may be used to synchronize profiling tools.
    #[arg(short, long, default_value = None)]
    sleeperfile: Option<String>,

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

       /// Sends a batch of requests and reads back the responses.
    fn send_batch(&mut self, requests: &[String]) -> Result<usize, std::io::Error> {
        // Write all requests to the stream (pipelining)
        for req in requests {
            self.stream.write_all(req.as_bytes())?;            
        }
        self.stream.write_all("ENDBATCH\r\n".as_bytes())?;
        let _ = self.stream.flush(); 
        
        // Read all responses from the stream
        let mut resp_buf = String::new();
        for i in 0..requests.len() {
            if self.reader.read_line(&mut resp_buf)? > 0 {  
                resp_buf.clear(); // Clear buffer for next read
            }
            else {
                return Ok(i)
            }
        }        
        Ok(requests.len())
    }

    fn exit_server(addr: &str, code: &str) {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(format!("EXIT {code}\r\n").as_bytes()).unwrap();
    }
}

fn main() {
    let args = Args::parse();
    let args = Arc::new(args);

    // --- Pre-population Phase ---
    if args.prepopulate > 0 {
        eprintln!(
            "Pre-populating {} keys with {} byte values...",
            args.prepopulate, args.value_size
        );
        prepopulate_keys(&args).expect("Failed to pre-populate keys");
        eprintln!("Pre-population complete.");
    }

    if let Some(ref file) = args.sleeperfile {
        let _ = std::fs::remove_file(file);
    }

    // --- Benchmarking Phase ---
    eprintln!(
        "Starting benchmark with {} threads ({} ops/thread)...",
        args.threads, args.ops
    );
    let start = Instant::now();
    let mut handles = vec![];

    for _ in 0..args.threads {
        let args_clone = Arc::clone(&args);
        handles.push(thread::spawn(move || {
            run_client_thread(args_clone)
        }));
    }

    let mut total_ops = 0;
    let mut total_histo = Histogram::new(4).unwrap();
    for handle in handles {
        let (ops,histo) = handle.join().expect("Thread panicked");
        total_ops += ops;
        let _ = total_histo.add(histo);
    }

    let duration = start.elapsed();
    let throughput = total_ops as f64 / duration.as_secs_f64();

    if args.exit_code.len() > 0 {
        Connection::exit_server(&args.addr,&args.exit_code);
    }
    // --- Output Results ---
    println!("\n--- Benchmark Results ---");
    println!("Total Operations: {}", total_ops);
    println!("Total Duration:   {:.4} s", duration.as_secs_f64());
    println!("Avg Operations/s: {:.2}", throughput);
    println!("Mean latency: {:.0} ms",total_histo.mean()/1000.0);
    println!("99% tail latency: {:.0} ms",total_histo.value_at_percentile(99.0)/1000);
}

/// Logic for a single client thread.
fn run_client_thread(args: Arc<Args>) -> (usize,Histogram<u64>) {
    let mut rng = rand::thread_rng();
    let mut histo = Histogram::new(4).unwrap();
    if args.connections == 0 {
        return (0,histo); // Avoid division by zero if no connections are specified
    }
    let mut connections: Vec<_> = (0..args.connections)
        .map(|_| Connection::new(&args.addr).expect("Failed to connect"))
        .collect();

    let mut requests_batch = String::new();
    let mut ops_done = 0usize;
    
    // We cannot use .cycle() on a mutable iterator.
    // Instead, we'll cycle through connections manually using an index.
    let mut conn_index = 0;
    let conn_count = connections.len();

    let value_template = "a".repeat(args.value_size);

    while ops_done < args.ops {
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
            requests_batch.push_str(&req);
        }
        requests_batch.push_str("ENDBATCH\r\n");

        
        let start = std::time::Instant::now();
        let _ = connection.stream.write(requests_batch.as_bytes());

        // Read all responses from the stream
        let mut resp_buf = String::new();
        for _ in 0..current_batch_size {
            if connection.reader.read_line(&mut resp_buf).unwrap() > 0 {  
                resp_buf.clear(); // Clear buffer for next read
                let _ = histo.record(start.elapsed().as_micros() as u64);
                ops_done += 1; 
            }
            else {
                *connection = Connection::new(&args.addr).expect("Reconnect failed");
            }
        }
    }

    println!("Thread finished.");
    (ops_done,histo)
}

/// Pre-populates the kvstore with random data.
fn prepopulate_keys(args: &Arc<Args>) -> Result<(), std::io::Error> {
    if args.batch_size == 0 {
        return Ok(());
    }
    let mut rng = rand::thread_rng();
    let mut requests_batch = Vec::with_capacity(args.batch_size);

    for i in 0..args.prepopulate {
        let mut conn = Connection::new(&args.addr)?;
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