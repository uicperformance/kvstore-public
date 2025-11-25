use std::{fs::File, io::Write, num::NonZeroUsize};

use sha2::{Digest, Sha256};

#[derive(Default)]
pub struct BenchmarkSaver {
    total_ops: Option<NonZeroUsize>,
    duration: Option<f64>,
    throughput: Option<f64>,
    /// ASCII hex characters representing the bytes in the 32-byte SHA256 digest
    checksum: String,
}

impl BenchmarkSaver {
    pub fn new() -> Self {
        /* CARGO_MANIFEST_DIR will be set on compilation with `cargo` as documented below:
         * https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
         */
        let source_file_contents = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            '/',
            "src/bin/server.rs"
        ));

        // Initially checksum just the source file
        let checksum = Sha256::digest(source_file_contents);

        Self {
            checksum: format!("{:x}", checksum),
            ..Default::default()
        }
    }

    pub fn total_ops(mut self, total_ops: usize) -> Self {
        // Panic on failure to avoid error handling in caller
        self.total_ops = Some(
            total_ops
                .try_into()
                .expect("usize passed to BenchmarkSaver::total_ops must be non-zero"),
        );

        self
    }

    pub fn duration(mut self, duration: f64) -> Self {
        self.duration = Some(duration);

        self
    }

    pub fn throughput(mut self, throughput: f64) -> Self {
        self.throughput = Some(throughput);

        self
    }

    pub fn save(self) {
        // Extract all benchmark numbers (panicking if any are missing)
        let (total_ops, duration, throughput) = self
            .total_ops
            .zip(self.duration)
            .zip(self.throughput)
            .map(|((total_ops, duration), throughput)| (total_ops.get(), duration, throughput))
            .expect("All benchmarks must be set before calling BenchmarkSaver::save");

        let line_part_one = format!("{} {:.4} {:.2}", total_ops, duration, throughput);

        let digest = Sha256::digest(format!("{}{}", &self.checksum, line_part_one));

        // Save to file
        let mut file = File::create(SAVE_TO_PATH)
            .expect("Creation of .benchmarks std::fs::File must not fail");
        writeln!(file, "{} {:x}", line_part_one, digest)
            .expect("Writing to .benchmarks std::fs::File must not fail");
    }
}

const SAVE_TO_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), '/', ".benchmarks");
