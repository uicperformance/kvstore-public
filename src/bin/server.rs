// server.rs
// TCP server that loads TreeMap from disk, persists after mutations.
use clap::{ArgAction, Parser};
use kvstore::TreeMap;
use serde::*;
use std::fs::*;
use std::io::Seek;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};

// the kvstore can be configured to use different key types: String, FixedSize, or Integer below
type KeyType = String; // String, FixedSize, Integer
type ValueType = String; // String, FixedSize (max 32-byte values only)

const FIXED_LEN: usize = 32;
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
struct FixedSize([u8; FIXED_LEN]);
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
struct Integer(usize);

impl From<&str> for FixedSize {
    fn from(s: &str) -> Self {
        assert!(s.len() <= FIXED_LEN);
        let mut ret = FixedSize([0u8; FIXED_LEN]);
        ret.0[0..std::cmp::min(FIXED_LEN, s.len())]
            .copy_from_slice(&s.as_bytes()[0..std::cmp::min(FIXED_LEN, s.len())]);
        ret
    }
}
impl std::fmt::Display for FixedSize {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}", String::from_utf8(self.0.to_vec()).unwrap())
    }
}

impl From<&str> for Integer {
    fn from(s: &str) -> Self {
        // skip the "key" part
        Integer(usize::from_str_radix(&s[3..], 10).unwrap())
    }
}
impl std::fmt::Display for Integer {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}", self.0)
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address and port to bind the server socket to
    #[arg(short, long, default_value = "127.0.0.1:4000")]
    addr: String,

    /// Do not persist the database contents to disk
    #[arg(short, long, default_value = "true")]
    memonly: bool,

    /// Run the server single-threaded
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    singlethread: bool,

    /// Location of the database file
    #[arg(short, long, default_value = "kvstore.db")]
    dbfile: String,

    /// Location of the database transaction log
    #[arg(short, long, default_value = "kvstore.log")]
    logfile: String,

    /// Snapshot interval
    #[arg(short, long, default_value = "100")]
    snapshot_interval: usize,

    /// Pass this code to the server EXIT command to have it exit
    #[arg(short, long, default_value = "")]
    exit_code: String,
}

fn handle_client(
    args: Arc<Args>,
    stream: TcpStream,
    logwriter: Arc<Mutex<File>>,
    map: Arc<RwLock<TreeMap<KeyType, ValueType>>>,
) {
    let mut writer = stream.try_clone().unwrap();
    let reader = BufReader::with_capacity(65536, &stream);
    let mut lines = reader.lines();
    let mut response = String::new();
    let mut log = String::new();
    let mut snapshot_count = 0;

    while let Some(Ok(line)) = lines.next() {
        let mut parts: [&str; 3] = [""; 3];
        for (partlen, part) in line.trim_end().splitn(3, ' ').enumerate() {
            parts[partlen] = part;
        }

        //        let parts: Vec<&str> = line.trim_end().splitn(3, ' ').collect();
        match parts[0] {
            "GET" if parts.len() == 2 => {
                let map = map.read().unwrap();
                response.push_str(
                    &match map.get(&Into::<KeyType>::into(parts[1]).to_owned()) {
                        Some(v) => format!("OK {}\r\n", v),
                        None => "ERR NotFound\r\n".into(),
                    },
                );
            }
            "SET" if parts.len() == 3 => {
                let mut map = map.write().unwrap();
                map.insert(
                    Into::<KeyType>::into(parts[1]),
                    Into::<ValueType>::into(parts[2]),
                );
                if !args.memonly {
                    log.push_str(line.as_str());
                    log.push_str("\n");
                }
                response.push_str("OK\r\n");
            }
            "REMOVE" if parts.len() == 2 => {
                let mut map = map.write().unwrap();
                response.push_str(match map.remove(&Into::<KeyType>::into(parts[1])) {
                    Some(_) => {
                        if !args.memonly {
                            log.push_str(line.as_str());
                            log.push_str("\n");
                        }
                        "OK\r\n".into()
                    }
                    None => "ERR NotFound\r\n".into(),
                });
            }
            "SEEK" if parts.len() == 2 => {
                let map = map.read().unwrap();
                response.push_str(&match map.seek_ge(&Into::<KeyType>::into(parts[1])) {
                    Some((k, v)) => format!("OK {} {}\r\n", k, v),
                    None => "ERR NotFound\r\n".into(),
                });
            }
            "ENDBATCH" => {
                writer.write_all(response.as_bytes()).unwrap();
                response = String::new();

                snapshot_count += 1;
                if args.memonly == false && snapshot_count == args.snapshot_interval {
                    print!("Snapshotting...");
                    map.read().unwrap().save_to_file(&args.dbfile).unwrap();
                    println!("done");
                    let mut log = logwriter.lock().unwrap();
                    log.rewind().unwrap();
                    log.set_len(0).unwrap();
                    snapshot_count = 0;
                }

                if args.memonly == false {
                    let mut logwriter = logwriter.lock().unwrap();
                    logwriter.write_all(log.as_bytes()).unwrap();
                    logwriter.flush().unwrap();

                    log = String::new();
                }
            }
            // This is a handy special command to help with profiling the server. Would
            // not recommend having a command like this in your typical key-value store!
            "EXIT" if parts.len() == 2 && parts[1] == args.exit_code => {
                eprintln!("Received EXIT command with correct exit code. Exiting.");
                std::process::exit(0);
            }
            "CLEAR" => {
                let _ = std::mem::replace(&mut *map.write().unwrap(), TreeMap::new());
                writer.write_all("OK\r\n".as_bytes()).unwrap();
                println!("Cleared map.");
            }
            _ => {
                response = "ERR UnknownCommand\r\n".into();
            }
        }
    }
}

fn recover_from_log(map: &mut TreeMap<KeyType, ValueType>, log: File) {
    let mut lines = BufReader::new(log).lines();
    println!("Recovering from log...");
    let mut count = 0;
    while let Some(Ok(line)) = lines.next() {
        count += 1;
        let parts: Vec<&str> = line.trim_end().splitn(3, ' ').collect();
        match parts[0] {
            "SET" if parts.len() == 3 => {
                map.insert(parts[1].into(), parts[2].into());
            }
            "REMOVE" if parts.len() == 2 => {
                map.remove(&parts[1].into());
            }
            _ => {
                panic!("Bad log entry.");
            }
        }
    }
    println!("Recovered {count} updates from log.\n");
}
fn main() -> std::io::Result<()> {
    let args = Arc::new(Args::parse());

    {
        // Create or open pidfile and write PID to it
        let mut file = std::fs::File::create("server_pid.txt").unwrap();
        writeln!(file, "{}", std::process::id())?;
    }

    let mut map = match TreeMap::load_from_file(&args.dbfile) {
        Ok(m) => m,
        Err(_) => TreeMap::new(),
    };

    if let Ok(true) = std::fs::exists(args.logfile.as_str()) {
        recover_from_log(&mut map, File::open(args.logfile.as_str()).unwrap());
    }

    let listener = TcpListener::bind(&args.addr)?;
    println!(
        "Server listening on {}, using keys of type {}",
        args.addr,
        std::any::type_name::<KeyType>()
    );

    let map = Arc::new(RwLock::new(map));
    let logfile = Arc::new(Mutex::new(File::create(args.logfile.as_str()).unwrap()));

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                // Nagle's algorithm waits a little bit before acknowledging a received packet.
                // This is usually a good idea, but not if your program sends small packets and cares about low latency.
                s.set_nodelay(true)?;
                let args = args.clone();
                let map = map.clone();
                let logfile = logfile.clone();

                // use the new --singlethread command line argument to set this
                if args.singlethread {
                    handle_client(args, s, logfile, map);
                } else {
                    std::thread::spawn(move || {
                        handle_client(args, s, logfile, map);
                    });
                }
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
    Ok(())
}
