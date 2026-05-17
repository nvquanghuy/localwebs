use crate::config::Config;
use crate::detector::ServiceDetector;
use crate::models::{OpenPort, ServiceInfo};
use crate::scanner::PortScanner;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct IncrementalScanner {
    known_services: Arc<RwLock<HashMap<u16, ServiceInfo>>>,
    last_full_scan: Arc<RwLock<Instant>>,
    config: Config,
}

impl IncrementalScanner {
    pub fn new(config: Config) -> Self {
        Self {
            known_services: Arc::new(RwLock::new(HashMap::new())),
            last_full_scan: Arc::new(RwLock::new(Instant::now())),
            config,
        }
    }

    /// Quick scan: only probes new/changed services
    pub async fn scan(&self) -> Vec<ServiceInfo> {
        // Quick port scan (just lsof, no HTTP probing)
        let scanner = PortScanner::new(
            self.config.portal.scan_timeout_ms,
            self.config.scan_ranges.ports.clone(),
        );
        let current_ports = scanner.scan().await;

        let mut known = self.known_services.write().await;

        // Build current port set for fast lookup
        let current_port_set: HashMap<u16, &OpenPort> =
            current_ports.iter().map(|p| (p.port, p)).collect();

        // Detect new ports
        let new_ports: Vec<OpenPort> = current_ports
            .iter()
            .filter(|p| !known.contains_key(&p.port))
            .cloned()
            .collect();

        // Detect removed ports
        let removed_ports: Vec<u16> = known
            .keys()
            .filter(|p| !current_port_set.contains_key(p))
            .copied()
            .collect();

        // Detect process changes (port still exists but process changed)
        let changed_ports: Vec<OpenPort> = current_ports
            .iter()
            .filter(|p| {
                if let Some(existing) = known.get(&p.port) {
                    // Check if process name or PID changed
                    existing.process_name != p.process_name || existing.pid != p.pid
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        // Log changes
        if !new_ports.is_empty() {
            println!(
                "🆕 New services detected: {}",
                new_ports
                    .iter()
                    .map(|p| p.port.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if !removed_ports.is_empty() {
            println!(
                "🗑️  Services removed: {}",
                removed_ports
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if !changed_ports.is_empty() {
            println!(
                "🔄 Services changed: {}",
                changed_ports
                    .iter()
                    .map(|p| p.port.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        // Probe new services
        if !new_ports.is_empty() {
            let detector =
                ServiceDetector::new(self.config.clone(), self.config.portal.probe_timeout_ms);

            for port in new_ports {
                let service = detector.identify(port.clone()).await;
                known.insert(port.port, service);
            }
        }

        // Re-probe changed services
        if !changed_ports.is_empty() {
            let detector =
                ServiceDetector::new(self.config.clone(), self.config.portal.probe_timeout_ms);

            for port in changed_ports {
                let service = detector.identify(port.clone()).await;
                known.insert(port.port, service);
            }
        }

        // Remove disappeared services
        for port in removed_ports {
            known.remove(&port);
        }

        // Check if we need a full re-probe
        let last_full = *self.last_full_scan.read().await;
        if last_full.elapsed() > Duration::from_secs(300) {
            // Full re-probe every 5 minutes
            println!("🔄 Running full re-probe (updating health status)...");
            self.full_reprobe(&mut known).await;
            *self.last_full_scan.write().await = Instant::now();
        }

        // Return sorted list
        let mut services: Vec<ServiceInfo> = known.values().cloned().collect();
        services.sort_by_key(|s| s.port);
        services
    }

    /// Full re-probe: updates health status and metadata for all existing services
    async fn full_reprobe(&self, known: &mut HashMap<u16, ServiceInfo>) {
        let detector =
            ServiceDetector::new(self.config.clone(), self.config.portal.probe_timeout_ms);

        // Re-probe all existing services to update health status
        let ports_to_probe: Vec<u16> = known.keys().copied().collect();

        for port in ports_to_probe {
            if let Some(existing) = known.get(&port) {
                // Create OpenPort from existing service info
                let open_port = OpenPort {
                    port,
                    process_name: existing.process_name.clone(),
                    pid: existing.pid,
                    start_time: existing.start_time,
                };

                let updated_service = detector.identify(open_port).await;
                known.insert(port, updated_service);
            }
        }

        println!("✅ Full re-probe complete");
    }

    /// Force a full refresh (useful for manual refresh button)
    pub async fn force_full_scan(&self) -> Vec<ServiceInfo> {
        println!("🔄 Force full scan triggered");

        // Clear known services
        self.known_services.write().await.clear();

        // Reset last full scan time
        *self.last_full_scan.write().await = Instant::now();

        // Perform full scan
        self.scan().await
    }

    /// Get current services without scanning
    pub async fn get_cached(&self) -> Vec<ServiceInfo> {
        let known = self.known_services.read().await;
        let mut services: Vec<ServiceInfo> = known.values().cloned().collect();
        services.sort_by_key(|s| s.port);
        services
    }

    /// Get stats about the scanner state
    pub async fn get_stats(&self) -> ScannerStats {
        let known = self.known_services.read().await;
        let last_full = *self.last_full_scan.read().await;

        ScannerStats {
            service_count: known.len(),
            last_full_scan: last_full,
            time_until_full_scan: Duration::from_secs(300)
                .checked_sub(last_full.elapsed())
                .unwrap_or(Duration::ZERO),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannerStats {
    pub service_count: usize,
    pub last_full_scan: Instant,
    pub time_until_full_scan: Duration,
}

impl Clone for IncrementalScanner {
    fn clone(&self) -> Self {
        Self {
            known_services: Arc::clone(&self.known_services),
            last_full_scan: Arc::clone(&self.last_full_scan),
            config: self.config.clone(),
        }
    }
}
