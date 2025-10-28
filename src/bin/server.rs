#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

// server.rs
// TCP server that loads TreeMap from disk, persists after mutations.
use clap::{ArgAction, Parser};
use std::net::{TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Write};
use std::fs::*;
use std::sync::{Arc, RwLock, Mutex};

#[cfg(not(feature="btree"))]
use kvstore::TreeMap;
//use std::io::Seek;
use serde::*;
#[cfg(feature="btree")]
use kvstore::btree::*;

// the kvstore can be configured to use different key types: String, FixedSize, or Integer below
type KeyType=String; // String, FixedSize, HeapSlice
type ValueType=String; // String, FixedSize (max 32-byte values only)

#[cfg(feature="btree")]
const FANOUT: usize = 16;
#[cfg(feature="btree")]
type MapType=BTree<KeyType, ValueType, FANOUT>;

#[cfg(not(feature="btree"))]
type MapType=TreeMap<KeyType, ValueType>;

const FIXED_LEN: usize = 32;

#[derive(PartialOrd,Ord,PartialEq,Eq,Copy,Clone,Debug,Serialize,Deserialize)]
struct FixedSize([u8;FIXED_LEN]);

impl From<&str> for FixedSize {
    fn from(s: &str) -> Self {
        assert!(s.len() <= FIXED_LEN);
        let mut ret = FixedSize([0u8;FIXED_LEN]);
        ret.0[0..std::cmp::min(FIXED_LEN,s.len())].copy_from_slice(&s.as_bytes()[0..std::cmp::min(FIXED_LEN,s.len())]);
        ret
    }
}

impl AsRef<str> for FixedSize {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

impl std::borrow::Borrow<str> for FixedSize {
    fn borrow(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.0[..]) }
    }
}

/*
#[derive(PartialOrd,Ord,PartialEq,Eq,Clone,Debug,Serialize,Deserialize)]
struct HeapSlice(Box<[u8]>); 

impl From<&str> for HeapSlice {
    fn from(s: &str) -> Self {
        HeapSlice(s.as_bytes().into())
    }
}

impl AsRef<str> for HeapSlice {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&*self.0) }
    }
}

impl std::borrow::Borrow<str> for HeapSlice {
    fn borrow(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.0[..]) }
    }
}
*/

impl std::fmt::Display for FixedSize {
    fn fmt(&self, out:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out,"{}",String::from_utf8(self.0.to_vec()).unwrap())
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

fn handle_client(args: Arc<Args>, stream: TcpStream, logwriter: Arc<Mutex<File>>, map: Arc<RwLock<MapType>>)
{
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::with_capacity(65536,&stream);
//    let mut lines = reader.lines();
    let mut response = String::with_capacity(4096);
    let mut log = String::new();
    let mut line = String::with_capacity(200);
//    let mut snapshot_count = 0;
    while let Ok(length) = reader.read_line(&mut line) {
        if length == 0 { break; }

        let mut parts = ["";3];
        let mut partlen = 0;
        for part in line.trim_end().splitn(3, ' ') {
            parts[partlen]=part;
            partlen+=1;
        } 

//        let parts: Vec<&str> = line.trim_end().splitn(3, ' ').collect();
        match parts[0] {
            "GET" if partlen == 2 => {
                let map = map.read().unwrap();                
                match map.get(parts[1]) {
                    Some(v) => {
                        response.push_str("OK ");
                        response.push_str(v);
                        response.push_str("\r\n");
                    },
                    None => {
                        response.push_str("ERR NotFound\r\n")
                    }
                }
                // response.push_str(&match map.get(parts[1]) {
                //     Some(v) => format!("OK {}\r\n", v),
                //     None    => "ERR NotFound\r\n".into(),
                // });
            }
            "SET" if partlen == 3 => {
                let mut map = map.write().unwrap();
                map.insert(Into::<KeyType>::into(parts[1]), Into::<ValueType>::into(parts[2]));
                if !args.memonly {
                    log.push_str(line.as_str());
                    log.push_str("\n");
                }
                response.push_str("OK\r\n");
            }
            // "REMOVE" if partlen == 2 => {
            //     let mut map = map.write().unwrap();
            //     response.push_str(match map.remove(&Into::<KeyType>::into(parts[1])) {
            //         Some(_) => {
            //             if !args.memonly {
            //                 log.push_str(line.as_str());
            //                 log.push_str("\n");
            //             }
            //             "OK\r\n".into()
            //         } 
            //         None => "ERR NotFound\r\n".into(),
            //     });
            // }
            // "SEEK" if partlen == 2 => {
            //     let map = map.read().unwrap();
            //     response.push_str(&match map.seek_ge(&Into::<KeyType>::into(parts[1])) {
            //         Some((k, v)) => format!("OK {} {}\r\n", k, v),
            //         None          => "ERR NotFound\r\n".into(),
            //     });
            // }
            "ENDBATCH" => {
                writer.write_all(response.as_bytes()).unwrap();
                response=String::new();

//                snapshot_count += 1;
                // if args.memonly==false && snapshot_count == args.snapshot_interval {
                //     print!("Snapshotting...");
                //     map.read().unwrap().save_to_file(&args.dbfile).unwrap();                    
                //     println!("done");
                //     let mut log = logwriter.lock().unwrap();
                //     log.rewind().unwrap();
                //     log.set_len(0).unwrap();
                //     snapshot_count = 0;
                // }

                if args.memonly==false {
                    let mut logwriter = logwriter.lock().unwrap();
                    logwriter.write_all(log.as_bytes()).unwrap();
                    logwriter.flush().unwrap();
                    
                    log=String::new();
                }

            }            
            // This is a handy special command to help with profiling the server. Would 
            // not recommend having a command like this in your typical key-value store!
            "EXIT" if partlen == 2 && parts[1] == args.exit_code  => {
                eprintln!("Received EXIT command with correct exit code. Exiting.");
                std::process::exit(0);
            },
            "STATS" => {                
                let s = map.read().unwrap().stats();
                println!("Stats: {:?}",s);
                writer.write_all(format!("{} {}\r\n",s.size,s.depth).as_bytes()).unwrap();
            },
            "CLEAR" => {
                let _ = std::mem::replace(&mut *map.write().unwrap(),MapType::new());
                writer.write_all("OK\r\n".as_bytes()).unwrap();
                println!("Cleared map.");
            },
            _ => { 
                response="ERR UnknownCommand\r\n".into();
            }
        };
        line.clear();
    }
}

// fn recover_from_log(map: &mut BTree<KeyType, ValueType, 16, 2>, log: File) { 
//     let mut lines = BufReader::new(log).lines();
//     println!("Recovering from log...");
//     let mut count = 0;
//     while let Some(Ok(line)) = lines.next() {
//         count+=1;
//         let parts: Vec<&str> = line.trim_end().splitn(3, ' ').collect();
//         match parts[0] {
//         "SET" if parts.len() == 3 => {
//             map.insert(parts[1].into(), parts[2].into());
//         },
//         "REMOVE" if parts.len() == 2 => {
//             map.remove(&parts[1].into());
//         },
//         _ => { panic!("Bad log entry."); }
//         }  
//     }
//     println!("Recovered {count} updates from log.\n");

// }
fn main() -> std::io::Result<()> {
    let args = Arc::new(Args::parse());
 
    {   // Create or open pidfile and write PID to it
        let mut file = std::fs::File::create("server_pid.txt").unwrap();
        writeln!(file, "{}", std::process::id())?;
    }


    let map = MapType::new(); /*match BTree::load_from_file(&args.dbfile) {
        Ok(m) => m,
        Err(_) => BTree::new(),
    };*/

//    if let Ok(true) = std::fs::exists(args.logfile.as_str()) {
//        recover_from_log(&mut map, File::open(args.logfile.as_str()).unwrap());
//    }

    let listener = TcpListener::bind(&args.addr)?;
    println!("Server listening on {}, using tree of type {}",args.addr,std::any::type_name::<MapType>());

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
                }
                else { 
                    std::thread::spawn(move || { handle_client(args, s, logfile, map); });
                }
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
    Ok(())
}
