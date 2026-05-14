use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub portal: PortalConfig,
    #[serde(default)]
    pub scan_ranges: ScanRanges,
    #[serde(default)]
    pub services: Vec<KnownService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalConfig {
    #[serde(default = "default_portal_port")]
    pub port: u16,
    #[serde(default = "default_scan_timeout")]
    pub scan_timeout_ms: u64,
    #[serde(default = "default_probe_timeout")]
    pub probe_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRanges {
    #[serde(default = "default_ports")]
    pub ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownService {
    pub port: u16,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_url_path")]
    pub url_path: String,
}

impl Default for PortalConfig {
    fn default() -> Self {
        Self {
            port: default_portal_port(),
            scan_timeout_ms: default_scan_timeout(),
            probe_timeout_ms: default_probe_timeout(),
        }
    }
}

impl Default for ScanRanges {
    fn default() -> Self {
        Self {
            ports: default_ports(),
        }
    }
}

fn default_portal_port() -> u16 {
    4444
}

fn default_scan_timeout() -> u64 {
    500
}

fn default_probe_timeout() -> u64 {
    2000
}

fn default_ports() -> Vec<u16> {
    vec![
        3000, 3001, 3002, 3003, 4200, 4300, 5000, 5001, 5173, 8000, 8001, 8080, 8081, 8082, 8888,
        9000, 9001,
    ]
}

fn default_url_path() -> String {
    "/".to_string()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", path))
    }

    pub fn get_known_service(&self, port: u16) -> Option<&KnownService> {
        self.services.iter().find(|s| s.port == port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            portal: PortalConfig::default(),
            scan_ranges: ScanRanges::default(),
            services: Vec::new(),
        }
    }
}
