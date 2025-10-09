// client.rs
// Synchronous client issuing workload against server.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn send_request(req: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect("127.0.0.1:4000")?;
    stream.write_all(req.as_bytes())?;
    let mut reader = BufReader::new(stream);
    let mut resp = String::new();
    reader.read_line(&mut resp)?;
    Ok(resp)
}

fn main() {
    let num_ops = 1000;
    for i in 0..num_ops {
        let req = format!("SET key{} value{}\n", i, i);
        match send_request(&req) {
            Ok(r) => println!("SET {}: {}", i, r.trim_end()),
            Err(e) => eprintln!("SET {} failed: {}", i, e),
        }
    }
    for i in 0..num_ops {
        let key = format!("key{}", i * 10);
        let req = format!("SEEK {}\n", key);
        match send_request(&req) {
            Ok(r) => println!("SEEK {}: {}", key, r.trim_end()),
            Err(e) => eprintln!("SEEK {} failed: {}", key, e),
        }
    }
}
