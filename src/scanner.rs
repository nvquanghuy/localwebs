use crate::models::OpenPort;
use anyhow::Result;
use regex::Regex;
use std::process::Command;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub struct PortScanner {
    timeout: Duration,
    fallback_ports: Vec<u16>,
}

impl PortScanner {
    pub fn new(timeout_ms: u64, fallback_ports: Vec<u16>) -> Self {
        Self {
            timeout: Duration::from_millis(timeout_ms),
            fallback_ports,
        }
    }

    pub async fn scan(&self) -> Vec<OpenPort> {
        // Try lsof first
        if let Ok(ports) = self.scan_with_lsof() {
            if !ports.is_empty() {
                return self.filter_localhost_http(ports).await;
            }
        }

        // Fallback to manual scanning
        self.scan_fallback().await
    }

    fn scan_with_lsof(&self) -> Result<Vec<OpenPort>> {
        let output = Command::new("lsof")
            .args(["-i", "-P", "-n"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("lsof command failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let ports = self.parse_lsof_output(&stdout);

        Ok(ports)
    }

    fn parse_lsof_output(&self, output: &str) -> Vec<OpenPort> {
        let mut ports = Vec::new();

        // Regex to match lsof output with flexible spacing
        // Format: COMMAND PID USER FD TYPE DEVICE SIZE NODE NAME STATE
        // Example: python3 12345 user 3u IPv4 0x123 0t0 TCP *:8000 (LISTEN)
        let re = Regex::new(r"^(\S+)\s+(\d+)\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+TCP\s+(.+?):(\d+)\s+\(LISTEN\)").unwrap();

        for line in output.lines() {
            if !line.contains("LISTEN") || !line.contains("TCP") {
                continue;
            }

            if let Some(caps) = re.captures(line) {
                let process_name = caps.get(1).map(|m| m.as_str().to_string());
                let pid = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
                let address = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let port = caps.get(4).and_then(|m| m.as_str().parse::<u16>().ok());

                // Only include localhost services or wildcard bindings
                if let Some(port) = port {
                    if address == "*" || address == "127.0.0.1" || address == "0.0.0.0"
                        || address == "localhost" || address == "[::]"
                        || address.starts_with("192.168.") || address.starts_with("10.") {

                        // Get process start time if we have a PID
                        let start_time = if let Some(pid_val) = pid {
                            Self::get_process_start_time(pid_val)
                        } else {
                            None
                        };

                        ports.push(OpenPort {
                            port,
                            process_name,
                            pid,
                            start_time,
                        });
                    }
                }
            }
        }

        ports
    }

    fn get_process_start_time(pid: u32) -> Option<i64> {
        // Use ps to get process start time in seconds since epoch
        // ps -o lstart= gives human-readable start time
        // ps -o etimes= gives elapsed time in seconds
        // We'll use etimes and calculate the start time

        let output = Command::new("ps")
            .args(["-o", "etimes=", "-p", &pid.to_string()])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let elapsed_str = String::from_utf8_lossy(&output.stdout);
        let elapsed_secs: i64 = elapsed_str.trim().parse().ok()?;

        // Calculate start time = current time - elapsed time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;

        Some(now - elapsed_secs)
    }

    async fn filter_localhost_http(&self, ports: Vec<OpenPort>) -> Vec<OpenPort> {
        // Only keep ports that respond to TCP connections (HTTP-like services)
        let mut http_ports = Vec::new();

        for port in ports {
            if self.is_port_open(port.port).await {
                http_ports.push(port);
            }
        }

        http_ports
    }

    async fn scan_fallback(&self) -> Vec<OpenPort> {
        let mut tasks = Vec::new();

        for &port in &self.fallback_ports {
            let timeout = self.timeout;
            tasks.push(tokio::spawn(async move {
                if Self::check_port(port, timeout).await {
                    Some(OpenPort::new(port))
                } else {
                    None
                }
            }));
        }

        let mut open_ports = Vec::new();
        for task in tasks {
            if let Ok(Some(port)) = task.await {
                open_ports.push(port);
            }
        }

        open_ports
    }

    async fn is_port_open(&self, port: u16) -> bool {
        Self::check_port(port, self.timeout).await
    }

    async fn check_port(port: u16, duration: Duration) -> bool {
        let addr = format!("127.0.0.1:{}", port);
        timeout(duration, TcpStream::connect(addr))
            .await
            .is_ok()
    }
}
