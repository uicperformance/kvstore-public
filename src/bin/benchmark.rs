use clap::{Parser};
use std::sync::Arc;
use kvstore::bench_saver::BenchmarkSaver;
use kvstore::bench::*;

fn main() {
    let mut args = Args::parse();

    if args.submission_benchmark {
        args = Args::default();
    }

    let args = Arc::new(args);

    // --- Pre-population Phase ---
    if args.prepopulate > 0 {
        eprintln!(
            "Pre-populating {} keys with {} byte values...",
            args.prepopulate, args.value_size
        );
        kvstore::bench::prepopulate_keys(&args).expect("Failed to pre-populate keys");
        eprintln!("Pre-population complete.");
    }

    if let Some(ref file) = args.sleeperfile {
        std::fs::remove_file(file).expect("std::fs::remove_file for sleeperfile failed");
    }

    let (total_ops,total_histo,duration) = bench(args.clone());
    let throughput = total_ops as f64 / duration.as_secs_f64();

      // --- Output Results ---
    println!("\n--- Benchmark Results ---");
    println!("Total Operations: {}", total_ops);
    println!("Key Range: {}",args.key_range);
    println!("Total Duration:   {:.4} s", duration.as_secs_f64());
    println!("Avg Operations/s: {:.2}", throughput);
 
    println!("Mean latency: {:.0} us",total_histo.mean());
    println!("99% tail latency: {:.0} us",total_histo.value_at_percentile(99.0));
    println!("99.9% tail latency: {:.0} us",total_histo.value_at_percentile(99.9));
    println!("99.99% tail latency: {:.0} us",total_histo.value_at_percentile(99.99));

    if args.submission_benchmark {
        // Save benchmarks for your grade
        BenchmarkSaver::new().total_ops(total_ops).duration(duration.as_secs_f64()).throughput(throughput).save();
    }
}