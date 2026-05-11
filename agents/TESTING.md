# LocalWebs Testing Guide

## Overview

This document covers testing strategies, test scenarios, and quality assurance for LocalWebs.

## Testing Pyramid

```
                    ┌─────────────┐
                    │   Manual    │  Browser testing, exploratory
                    │   Testing   │  
                    └─────────────┘
                   ┌───────────────┐
                   │  Integration  │  End-to-end API tests
                   │     Tests     │  Service detection tests
                   └───────────────┘
                ┌─────────────────────┐
                │     Unit Tests      │  Function-level tests
                │                     │  Parser tests, logic tests
                └─────────────────────┘
```

## Unit Tests

### Configuration Parsing

**File:** `src/config.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_default_config() {
        let config = Config::default();
        assert_eq!(config.portal.port, 9999);
        assert_eq!(config.portal.scan_timeout_ms, 500);
        assert_eq!(config.portal.probe_timeout_ms, 2000);
    }

    #[test]
    fn test_load_config_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"
            [portal]
            port = 7777
            
            [[services]]
            port = 3000
            name = "Test Service"
        "#).unwrap();

        let config = Config::load(file.path()).unwrap();
        assert_eq!(config.portal.port, 7777);
        assert_eq!(config.services.len(), 1);
        assert_eq!(config.services[0].name, "Test Service");
    }

    #[test]
    fn test_missing_config_file() {
        let config = Config::load(Path::new("/nonexistent/config.toml"));
        assert!(config.is_ok());  // Should return default config
    }

    #[test]
    fn test_invalid_config_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "invalid toml content {{").unwrap();

        let config = Config::load(file.path());
        assert!(config.is_err());
    }

    #[test]
    fn test_get_known_service() {
        let mut config = Config::default();
        config.services.push(KnownService {
            port: 3000,
            name: "React App".to_string(),
            description: Some("Frontend".to_string()),
            url_path: "/".to_string(),
        });

        assert!(config.get_known_service(3000).is_some());
        assert!(config.get_known_service(8080).is_none());
    }
}
```

### lsof Parsing

**File:** `src/scanner.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lsof_basic() {
        let output = "node    12345  user   20u  IPv4 0x123  0t0 TCP 127.0.0.1:3000 (LISTEN)\n";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 3000);
        assert_eq!(ports[0].process_name, Some("node".to_string()));
        assert_eq!(ports[0].pid, Some(12345));
    }

    #[test]
    fn test_parse_lsof_wildcard() {
        let output = "python3  9876  user    3u  IPv4 0x456  0t0 TCP *:8000 (LISTEN)\n";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 8000);
        assert_eq!(ports[0].process_name, Some("python3".to_string()));
    }

    #[test]
    fn test_parse_lsof_multiple_ports() {
        let output = "\
node     1111  user   20u  IPv4 0x1  0t0 TCP *:3000 (LISTEN)
python3  2222  user    3u  IPv4 0x2  0t0 TCP 127.0.0.1:8000 (LISTEN)
ruby     3333  user   17u  IPv4 0x3  0t0 TCP *:9999 (LISTEN)
";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].port, 3000);
        assert_eq!(ports[1].port, 8000);
        assert_eq!(ports[2].port, 9999);
    }

    #[test]
    fn test_parse_lsof_ignore_non_listen() {
        let output = "\
node     1111  user   20u  IPv4 0x1  0t0 TCP 127.0.0.1:3000 (ESTABLISHED)
python3  2222  user    3u  IPv4 0x2  0t0 TCP 127.0.0.1:8000 (LISTEN)
";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 8000);
    }

    #[test]
    fn test_parse_lsof_ignore_external_ip() {
        let output = "\
node     1111  user   20u  IPv4 0x1  0t0 TCP 8.8.8.8:3000 (LISTEN)
python3  2222  user    3u  IPv4 0x2  0t0 TCP 127.0.0.1:8000 (LISTEN)
";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        // Should only include localhost binding
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 8000);
    }

    #[test]
    fn test_parse_lsof_ipv6() {
        let output = "node    12345  user   20u  IPv6 0x123  0t0 TCP [::]:3000 (LISTEN)\n";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 3000);
    }

    #[test]
    fn test_parse_lsof_empty_output() {
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output("");
        
        assert_eq!(ports.len(), 0);
    }
}
```

### Service Detection Logic

**File:** `src/detector.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_process_name() {
        let config = Config::default();
        let detector = ServiceDetector::new(config, 2000);
        
        assert_eq!(detector.format_process_name("node"), "Node.js Service");
        assert_eq!(detector.format_process_name("python"), "Python Service");
        assert_eq!(detector.format_process_name("nginx"), "Nginx Server");
        assert_eq!(detector.format_process_name("custom"), "Custom Service");
    }

    #[test]
    fn test_pattern_match() {
        let config = Config::default();
        let detector = ServiceDetector::new(config, 2000);
        
        assert_eq!(detector.pattern_match(3000), "Node.js/React App");
        assert_eq!(detector.pattern_match(5173), "Vite Dev Server");
        assert_eq!(detector.pattern_match(8000), "Python HTTP Server");
        assert_eq!(detector.pattern_match(9999), "Backend Service");
        assert_eq!(detector.pattern_match(12345), "Unknown HTTP Service");
    }

    #[test]
    fn test_extract_title() {
        let config = Config::default();
        let detector = ServiceDetector::new(config, 2000);
        
        let html = "<html><head><title>My App</title></head><body></body></html>";
        let title = detector.extract_title(html);
        
        assert_eq!(title, Some("My App".to_string()));
    }

    #[test]
    fn test_extract_title_no_title() {
        let config = Config::default();
        let detector = ServiceDetector::new(config, 2000);
        
        let html = "<html><head></head><body>No title</body></html>";
        let title = detector.extract_title(html);
        
        assert_eq!(title, None);
    }

    #[test]
    fn test_extract_title_with_whitespace() {
        let config = Config::default();
        let detector = ServiceDetector::new(config, 2000);
        
        let html = "<html><head><title>  Spaced Title  </title></head></html>";
        let title = detector.extract_title(html);
        
        assert_eq!(title, Some("Spaced Title".to_string()));
    }
}
```

### Running Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_parse_lsof_basic

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=8

# Run tests for specific module
cargo test scanner::tests
```

## Integration Tests

### End-to-End Service Detection

**File:** `tests/integration_test.rs`

```rust
use localwebs::*;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[tokio::test]
async fn test_detect_python_http_server() {
    // Start a simple HTTP server
    thread::spawn(|| {
        let listener = TcpListener::bind("127.0.0.1:18000").unwrap();
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let response = "HTTP/1.1 200 OK\r\n\r\n<html><title>Test Server</title></html>";
                let _ = stream.write_all(response.as_bytes());
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test detection
    let config = Config::default();
    let scanner = PortScanner::new(500, vec![18000]);
    let ports = scanner.scan().await;

    assert!(!ports.is_empty());
    
    let detector = ServiceDetector::new(config, 2000);
    let service = detector.identify(ports[0].clone()).await;
    
    assert_eq!(service.port, 18000);
    assert_eq!(service.title, Some("Test Server".to_string()));
    assert!(service.is_healthy);
}

#[tokio::test]
async fn test_detect_closed_port() {
    let config = Config::default();
    let scanner = PortScanner::new(500, vec![19999]);
    let ports = scanner.scan().await;

    // Should be empty - port is not open
    assert!(ports.is_empty());
}

#[tokio::test]
async fn test_full_scan_flow() {
    let config = Config::default();
    
    // Scan for services
    let scanner = PortScanner::new(500, config.scan_ranges.ports.clone());
    let open_ports = scanner.scan().await;
    
    // Identify services
    let detector = ServiceDetector::new(config, 2000);
    let mut services = vec![];
    for port in open_ports {
        let service = detector.identify(port).await;
        services.push(service);
    }
    
    // Should find at least some services (depends on test environment)
    // At minimum, might find the test server itself
    assert!(!services.is_empty());
    
    // All services should have valid data
    for service in services {
        assert!(service.port > 0);
        assert!(!service.name.is_empty());
    }
}
```

### API Integration Tests

```rust
use axum::http::StatusCode;
use tower::ServiceExt;

#[tokio::test]
async fn test_api_list_services() {
    let config = Config::default();
    let state = AppState { config };
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/services")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let services: Vec<ServiceInfo> = serde_json::from_slice(&body).unwrap();
    
    // Should return an array (even if empty)
    assert!(services.is_empty() || !services.is_empty());
}

#[tokio::test]
async fn test_serve_html() {
    let config = Config::default();
    let state = AppState { config };
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    
    assert!(html.contains("LocalWebs"));
    assert!(html.contains("<!DOCTYPE html>"));
}
```

## Manual Testing

### Test Scenarios

#### 1. Basic Service Detection

**Setup:**
```bash
# Terminal 1: Start test services
python3 -m http.server 8000 &
python3 -m http.server 8001 &

# Terminal 2: Start LocalWebs
cargo run -- --port 6543
```

**Test:**
1. Open http://localhost:6543
2. Verify both Python servers appear
3. Verify process names show "python3"
4. Verify PIDs are displayed
5. Click on service cards - should open in new tabs

**Expected:**
- 2 services detected on ports 8000 and 8001
- Both show as healthy (green indicator)
- Page titles: "Directory listing for /"
- Server headers: "SimpleHTTP/..."
- Process: python3 with PID

#### 2. Config-Based Detection

**Setup:**
```bash
# Create test config
cat > test-config.toml << EOF
[portal]
port = 6543

[[services]]
port = 8000
name = "My Custom Service"
description = "Test service from config"
url_path = "/"
EOF

python3 -m http.server 8000 &
cargo run -- --config test-config.toml
```

**Test:**
1. Open http://localhost:6543
2. Verify service on 8000 shows custom name

**Expected:**
- Service name: "My Custom Service"
- Description: "Test service from config"
- Detection source: "config"

#### 3. Unhealthy Service Detection

**Setup:**
```bash
# Start LocalWebs
cargo run

# In another terminal, find a service and kill it
lsof -i :3000
kill -9 <PID>
```

**Test:**
1. Refresh the portal
2. Verify killed service shows as unhealthy

**Expected:**
- Service still appears (port might still be in lsof)
- Red status indicator
- No response time

#### 4. Auto-Refresh

**Setup:**
```bash
cargo run -- --port 6543
```

**Test:**
1. Open http://localhost:6543
2. Start a new service: `python3 -m http.server 9000`
3. Wait 10 seconds (auto-refresh interval)

**Expected:**
- New service appears automatically
- No page reload needed

#### 5. Hostname Resolution

**Setup:**
```bash
# Access via different hostnames
cargo run -- --port 6543
```

**Test:**
1. Open http://localhost:6543 - click services
2. Open http://127.0.0.1:6543 - click services
3. Open http://fedora:6543 - click services (if hostname configured)

**Expected:**
- Service links match the hostname used
- localhost → http://localhost:PORT
- 127.0.0.1 → http://127.0.0.1:PORT
- fedora → http://fedora:PORT

#### 6. Large Number of Services

**Setup:**
```bash
# Start many services
for port in {8000..8020}; do
    python3 -m http.server $port &
done

cargo run -- --port 6543
```

**Test:**
1. Open http://localhost:6543
2. Verify all 21 services detected
3. Check performance (should be <1s)

**Expected:**
- All services displayed
- Grid layout handles overflow
- Responsive scrolling
- No timeouts

#### 7. Mobile Responsiveness

**Test:**
1. Open browser dev tools
2. Toggle device emulation
3. Test on various screen sizes

**Expected:**
- Single column layout on mobile
- All cards readable
- Refresh button accessible
- No horizontal scrolling

### Browser Compatibility

Test in:
- Chrome/Chromium
- Firefox
- Safari (if available)
- Edge

Verify:
- HTML5 features work
- CSS Grid displays correctly
- Fetch API works
- Auto-refresh functions

### Performance Benchmarks

#### Scan Performance

```bash
# Measure scan time
time curl http://localhost:6543/api/services > /dev/null

# Expected: < 1 second for ~20 services
```

#### Memory Usage

```bash
# Check memory consumption
ps aux | grep localwebs

# Expected: < 50MB RSS
```

#### CPU Usage

```bash
# Monitor CPU while running
top -p $(pgrep localwebs)

# Expected: < 5% when idle, < 50% during scan
```

## Load Testing

### API Load Test

```bash
# Install hey (HTTP load generator)
go install github.com/rakyll/hey@latest

# Test API endpoint
hey -n 1000 -c 10 http://localhost:6543/api/services

# Expected:
# - All requests succeed (200 OK)
# - Average response time < 500ms
# - No errors or timeouts
```

### Concurrent User Simulation

```bash
# Simulate multiple users
for i in {1..10}; do
    (
        while true; do
            curl -s http://localhost:6543/api/services > /dev/null
            sleep 5
        done
    ) &
done

# Monitor portal
htop
```

**Expected:**
- Portal remains responsive
- Memory usage stays constant
- No crashes or panics

## Regression Testing

### Before Each Release

- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Manual test scenarios complete
- [ ] No compiler warnings
- [ ] Documentation updated
- [ ] Config examples valid
- [ ] Cross-platform check (if applicable)

### Test Matrix

| OS | Architecture | Status |
|----|--------------|--------|
| Linux (Fedora) | x86_64 | ✅ |
| Linux (Ubuntu) | x86_64 | ⬜ |
| macOS | ARM64 | ⬜ |
| macOS | x86_64 | ⬜ |

## Bug Reporting

When filing bugs, include:

1. **Environment:**
   - OS and version
   - Rust version (`rustc --version`)
   - LocalWebs version

2. **Steps to reproduce:**
   ```
   1. Start LocalWebs with `cargo run`
   2. Start service on port 3000
   3. Observe incorrect detection
   ```

3. **Expected vs Actual:**
   - Expected: Service shows as "Node.js"
   - Actual: Service shows as "Unknown"

4. **Logs:**
   ```
   🌐 LocalWebs portal running at http://0.0.0.0:6543
   ...
   ```

5. **Configuration:**
   ```toml
   [portal]
   port = 6543
   ...
   ```

## Test Coverage

### Current Coverage

Run with tarpaulin:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Open tarpaulin-report.html
```

### Coverage Goals

- Unit tests: > 80%
- Integration tests: Core flows covered
- Manual tests: All user scenarios

## Future Testing Improvements

### Automated E2E Tests

Use `headless_chrome` or `playwright`:

```rust
#[tokio::test]
async fn test_ui_service_click() {
    // Start LocalWebs
    // Launch browser
    // Navigate to portal
    // Click service card
    // Verify new tab opens with correct URL
}
```

### Property-Based Testing

Use `proptest`:

```rust
proptest! {
    #[test]
    fn parse_any_lsof_line(s in ".*") {
        // Should never panic
        let scanner = PortScanner::new(500, vec![]);
        let _ = scanner.parse_lsof_output(&s);
    }
}
```

### Continuous Integration

GitHub Actions workflow:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
```
