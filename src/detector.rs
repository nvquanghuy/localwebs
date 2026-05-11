use crate::config::{Config, KnownService};
use crate::models::{DetectionSource, OpenPort, ServiceInfo};
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::{Duration, Instant};

pub struct ServiceDetector {
    http_client: Client,
    config: Config,
}

impl ServiceDetector {
    pub fn new(config: Config, timeout_ms: u64) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap();

        Self {
            http_client,
            config,
        }
    }

    pub async fn identify(&self, port: OpenPort) -> ServiceInfo {
        let url = format!("http://127.0.0.1:{}", port.port);

        // 1. Check config for known service
        if let Some(known) = self.config.get_known_service(port.port) {
            return self.create_service_from_config(&port, known, &url).await;
        }

        // 2. Try HTTP probe
        if let Some(info) = self.http_probe(&port, &url).await {
            return info;
        }

        // 3. Use process name if available
        if let Some(ref process_name) = port.process_name {
            return ServiceInfo {
                port: port.port,
                name: self.format_process_name(process_name),
                description: None,
                source: DetectionSource::Process,
                url: url.clone(),
                title: None,
                server_header: None,
                process_name: port.process_name.clone(),
                pid: port.pid,
                is_healthy: false,
                response_time_ms: None,
                start_time: port.start_time,
            };
        }

        // 4. Pattern matching
        let name = self.pattern_match(port.port);

        ServiceInfo {
            port: port.port,
            name,
            description: None,
            source: DetectionSource::Pattern,
            url: url.clone(),
            title: None,
            server_header: None,
            process_name: port.process_name.clone(),
            pid: port.pid,
            is_healthy: false,
            response_time_ms: None,
            start_time: port.start_time,
        }
    }

    async fn create_service_from_config(
        &self,
        port: &OpenPort,
        known: &KnownService,
        base_url: &str,
    ) -> ServiceInfo {
        let url = if known.url_path == "/" {
            base_url.to_string()
        } else {
            format!("{}{}", base_url, known.url_path)
        };

        let start = Instant::now();
        let is_healthy = self.check_health(&url).await;
        let response_time = if is_healthy {
            Some(start.elapsed().as_millis() as u64)
        } else {
            None
        };

        ServiceInfo {
            port: port.port,
            name: known.name.clone(),
            description: known.description.clone(),
            source: DetectionSource::Config,
            url: base_url.to_string(),
            title: None,
            server_header: None,
            process_name: port.process_name.clone(),
            pid: port.pid,
            is_healthy,
            response_time_ms: response_time,
            start_time: port.start_time,
        }
    }

    async fn http_probe(&self, port: &OpenPort, url: &str) -> Option<ServiceInfo> {
        let start = Instant::now();
        let response = self.http_client.get(url).send().await.ok()?;
        let response_time_ms = start.elapsed().as_millis() as u64;

        let server_header = response
            .headers()
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let powered_by = response
            .headers()
            .get("x-powered-by")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let body = response.text().await.ok()?;
        let title = self.extract_title(&body);

        let name = title
            .clone()
            .or_else(|| powered_by)
            .or_else(|| server_header.clone())
            .or_else(|| port.process_name.clone())
            .unwrap_or_else(|| "Unknown Service".to_string());

        Some(ServiceInfo {
            port: port.port,
            name,
            description: None,
            source: DetectionSource::Http,
            url: url.to_string(),
            title,
            server_header,
            process_name: port.process_name.clone(),
            pid: port.pid,
            is_healthy: true,
            response_time_ms: Some(response_time_ms),
            start_time: port.start_time,
        })
    }

    fn extract_title(&self, html: &str) -> Option<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("title").ok()?;
        document
            .select(&selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    }

    async fn check_health(&self, url: &str) -> bool {
        self.http_client.get(url).send().await.is_ok()
    }

    fn format_process_name(&self, process: &str) -> String {
        match process {
            "node" => "Node.js Service".to_string(),
            "python" | "python3" => "Python Service".to_string(),
            "ruby" => "Ruby Service".to_string(),
            "java" => "Java Service".to_string(),
            "nginx" => "Nginx Server".to_string(),
            "apache2" | "httpd" => "Apache Server".to_string(),
            _ => format!("{} Service", capitalize_first(process)),
        }
    }

    fn pattern_match(&self, port: u16) -> String {
        match port {
            3000..=3003 => "Node.js/React App".to_string(),
            4200 => "Angular CLI".to_string(),
            5000 | 5001 => "Flask/Dev Server".to_string(),
            5173 => "Vite Dev Server".to_string(),
            8000 => "Python HTTP Server".to_string(),
            8080 | 8081 => "API Server".to_string(),
            8888 => "Jupyter Notebook".to_string(),
            9000 | 9001 => "Backend Service".to_string(),
            _ => "Unknown HTTP Service".to_string(),
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
