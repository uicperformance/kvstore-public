
use std::io;
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

/// A handle to a spawned child process.
pub struct ProcessHandle {
    child: Child,
}

/// Error types for the crate.
#[derive(Debug)]
pub enum Error {
    /// Failed to spawn the process.
    Spawn(io::Error),
    /// Failed to wait on the process.
    Wait(io::Error),
    /// Failed to kill the process.
    Kill(io::Error),
    /// Process timed out.    
    ServerStartTimeout,
    ServerExitTimeout,
}

/// Result type alias.
pub type Result<T> = std::result::Result<T, Error>;

impl ProcessHandle {
    /// Spawns an external executable with the given arguments.
    pub fn spawn(path: &str, args: &[&str]) -> Result<Self> {
        let child = Command::new(path)
            .args(args)
            .spawn()
            .map_err(Error::Spawn)?;
        Ok(ProcessHandle { child })
    }

    pub fn spawn_cmdline(cmd: &str) -> Result<Self> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err(Error::Spawn(
                io::Error::new(io::ErrorKind::InvalidInput, "Empty command line")
            ));
        }
        let path = parts[0];
        let args = &parts[1..];
        Self::spawn(path, args)
    }

    // repeatedly tries to connect 
    pub fn wait_for_server(&mut self, addr: &str, timeout: std::time::Duration) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if let Ok(_) = std::net::TcpStream::connect(addr) { return Ok(()) }
            else {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        return Err(Error::ServerStartTimeout)
    }

    /// Waits for the process to complete within the specified timeout.   
    pub fn wait_with_timeout(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let start = Instant::now();
        let sleep_duration = Duration::from_millis(100);
        loop {
            if start.elapsed() >= timeout {
                // Timeout reached, kill the process
                self.child
                    .kill()
                    .map_err(Error::Kill)?;
                
                // Wait for the kill to take effect and get the status
                let res = self.child
                    .wait()
                    .map_err(Error::Wait)
                    .map_err(|e| {
                        // If wait fails after kill, still consider it a timeout error
                        if let Error::Wait(_) = e {
                            Error::ServerExitTimeout
                        } else {
                            e
                        }
                    })
                    .or(Err(Error::ServerExitTimeout));

                if res.is_ok() { return Err(Error::ServerExitTimeout) }
                else { return res }
            }

            // Check if process has exited
            match self.child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {
                    // Still running, sleep a bit
                    thread::sleep(sleep_duration);
                }
                Err(e) => return Err(Error::Wait(e)),
            }
        }
    }
}