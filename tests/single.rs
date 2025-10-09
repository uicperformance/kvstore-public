use std::{sync::Arc, time::Duration};

use bench::Args;
use clap::Parser;
use kvstore::{process_handle::*, *};

#[test]
pub fn default() -> Result<()> {
    let addr = "localhost:6745";
    let mut server = ProcessHandle::spawn_cmdline(
        format!("cargo run --release --bin server -- --addr {addr} --exit-code=done").as_str(),
    )?;
    server.wait_for_server(addr, Duration::from_millis(2000))?;

    let args = Arc::new(Args {
        addr: addr.to_owned(),
        exit_code: "done".to_owned(),
        ..Default::default()
    });

    let (total_ops, _histo, duration) = bench::bench(args.clone());
    server.wait_with_timeout(Duration::from_millis(3000))?;

    assert_eq!(total_ops, args.ops * args.threads);
    assert!(duration.as_millis() < 10000);
    Ok(())
}

#[test]
pub fn memonly() -> Result<()> {
    let addr = "localhost:6746";
    let mut server = ProcessHandle::spawn_cmdline(
        format!("cargo run --release --bin server -- --memonly --addr {addr} --exit-code=done")
            .as_str(),
    )?;
    server.wait_for_server(addr, Duration::from_millis(2000))?;

    let args = Arc::new(bench::Args::parse_from(
        format!("bench --ops 10000 --exit-code done --addr {addr} --max-duration 10")
            .split_whitespace(),
    ));
    let (total_ops, _histo, duration) = bench::bench(args.clone());
    server.wait_with_timeout(Duration::from_millis(3000))?;

    assert_eq!(total_ops, args.ops * args.threads);
    assert!(total_ops as f64 / (duration.as_secs_f64()) > 1000.0);
    Ok(())
}
