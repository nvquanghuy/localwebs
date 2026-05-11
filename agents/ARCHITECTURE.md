# LocalWebs Architecture

## Overview

LocalWebs is a Rust-based service discovery portal that automatically detects and displays all HTTP services running on localhost. It uses an **incremental scanning architecture** that efficiently tracks service changes without repeatedly probing unchanged services. This approach combines OS-level port discovery (via `lsof`) with intelligent HTTP probing and state tracking for optimal performance in development environments.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         User Browser                         │
│                    (http://fedora:6543)                      │
└────────────────────────────┬────────────────────────────────┘
                             │
                             │ HTTP
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                       Axum Web Server                        │
│                        (0.0.0.0:6543)                        │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Routes                                                 │ │
│  │  • GET  /           → Serve HTML UI                    │ │
│  │  • GET  /api/services → Return cached services (2-5ms) │ │
│  │  • POST /api/scan    → Force full refresh (500ms)      │ │
│  │  • GET  /api/stats   → Scanner statistics              │ │
│  │  • GET  /assets/*    → Serve CSS/JS                    │ │
│  └────────────────────────────────────────────────────────┘ │
└────────────────────────────┬────────────────────────────────┘
                             │
                             │ Reads from
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              Incremental Scanner (Background Task)           │
│  • HashMap<port, ServiceInfo> (cached state)                 │
│  • Background scan every 5 seconds                           │
│  • Full re-probe every 5 minutes                             │
└────────────────────────────┬────────────────────────────────┘
                             │
                             │ Detects Changes
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    Change Detection Flow                     │
└─────────────────────────────────────────────────────────────┘
        │                    │                    │
        ▼                    ▼                    ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Scanner    │    │   Detector   │    │    Config    │
│              │    │              │    │              │
│ • lsof -i    │───▶│ Identifies   │◀───│ Known        │
│ • Parse      │    │ services:    │    │ services     │
│   output     │    │              │    │              │
│ • Extract    │    │ 1. Config    │    │ Port         │
│   ports,     │    │ 2. Process   │    │ mappings     │
│   PIDs,      │    │ 3. HTTP      │    │              │
│   process    │    │ 4. Pattern   │    │ Scan         │
│   names      │    │              │    │ ranges       │
└──────────────┘    └──────────────┘    └──────────────┘
        │                    │
        │                    │ HTTP Probing
        │                    ▼
        │           ┌──────────────────┐
        │           │   HTTP Client    │
        │           │  (reqwest)       │
        │           │                  │
        │           │ • GET requests   │
        │           │ • Extract titles │
        │           │ • Read headers   │
        │           └──────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│                      Service Info                            │
│  • Port, Name, Description                                   │
│  • Detection Source (Config/Process/HTTP/Pattern)            │
│  • Process Name & PID                                        │
│  • Health Status & Response Time                             │
│  • Page Title, Server Headers                                │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Main Entry Point (`main.rs`)

**Responsibilities:**
- CLI argument parsing with `clap`
- Configuration loading
- Web server initialization
- Tokio runtime management

**Flow:**
1. Parse CLI args (port, config path)
2. Load configuration from TOML
3. Create IncrementalScanner
4. Perform initial scan
5. Start background scanner task (5s interval)
6. Create AppState with scanner
7. Build Axum router
8. Bind to 0.0.0.0:PORT
9. Start async server

### 2. Incremental Scanner (`incremental_scanner.rs`) ⭐ **Core Innovation**

**Purpose:** Efficiently tracks service changes without repeatedly scanning unchanged services.

**Architecture:**

```rust
pub struct IncrementalScanner {
    known_services: Arc<RwLock<HashMap<u16, ServiceInfo>>>,  // State cache
    last_full_scan: Arc<RwLock<Instant>>,                    // Timing
    config: Config,
}
```

**Scanning Strategy:**

```
Every 5 seconds (background task):
  ↓
1. Quick lsof scan (~50ms)
   • Get current port list
   • Extract process names/PIDs
   ↓
2. Compare with known_services HashMap
   • Detect NEW ports (not in HashMap)
   • Detect REMOVED ports (in HashMap but not in lsof)
   • Detect CHANGED processes (port exists but PID/name different)
   ↓
3. Selective HTTP Probing
   • NEW ports → Full HTTP probe + add to HashMap
   • CHANGED → Re-probe + update HashMap
   • REMOVED → Remove from HashMap
   • UNCHANGED → Skip (use cached data)
   ↓
4. Return cached services (~115ms total)

Every 5 minutes:
  ↓
Full re-probe all services
  • Updates health status
  • Refreshes response times
  • Validates all cached data
```

**Performance Benefits:**

| Scenario | Traditional | Incremental | Speedup |
|----------|------------|-------------|---------|
| No changes | 500ms (probe all) | 115ms (lsof only) | **4.3x faster** |
| 1 new service | 500ms | 137ms (probe 1) | **3.6x faster** |
| 1 removed | 500ms | 114ms (cleanup) | **4.4x faster** |
| API request | 500ms | 2-5ms (cached) | **100-250x faster** |

**Key Methods:**

```rust
impl IncrementalScanner {
    // Background scan - detects and probes changes
    pub async fn scan(&self) -> Vec<ServiceInfo>
    
    // Force full refresh (for manual button)
    pub async fn force_full_scan(&self) -> Vec<ServiceInfo>
    
    // Read cached data (for API endpoint)
    pub async fn get_cached(&self) -> Vec<ServiceInfo>
    
    // Get scanner statistics
    pub async fn get_stats(&self) -> ScannerStats
}
```

**Change Detection Logic:**

```rust
// New services
let new_ports: Vec<OpenPort> = current_ports
    .iter()
    .filter(|p| !known.contains_key(&p.port))
    .cloned()
    .collect();

// Removed services
let removed_ports: Vec<u16> = known
    .keys()
    .filter(|p| !current_port_set.contains_key(p))
    .copied()
    .collect();

// Changed processes
let changed_ports: Vec<OpenPort> = current_ports
    .iter()
    .filter(|p| {
        if let Some(existing) = known.get(&p.port) {
            existing.process_name != p.process_name || existing.pid != p.pid
        } else {
            false
        }
    })
    .cloned()
    .collect();
```

**Logging:**

```
🆕 New services detected: 3000, 8080, 5555
🗑️  Services removed: 5555
🔄 Services changed: 3000
📡 Incremental scan complete: 23 services in 115ms
```

**Thread Safety:**

- Uses `Arc<RwLock<>>` for shared state
- Multiple readers (API requests) don't block each other
- Single writer (background scanner) has exclusive access
- No race conditions or deadlocks

### 3. Configuration System (`config.rs`)

**Data Structures:**
```rust
Config
├── portal: PortalConfig
│   ├── port: u16
│   ├── scan_timeout_ms: u64
│   └── probe_timeout_ms: u64
├── scan_ranges: ScanRanges
│   └── ports: Vec<u16>
└── services: Vec<KnownService>
    ├── port: u16
    ├── name: String
    ├── description: Option<String>
    └── url_path: String
```

**Features:**
- TOML parsing with serde
- Default values for all fields
- Graceful fallback if config file missing
- Port-based service lookup

### 4. Port Scanner (`scanner.rs`)

**Note:** Used by IncrementalScanner for the quick lsof scan phase.

**Primary Method: lsof-based Discovery**

```rust
lsof -i -P -n  // Get all internet connections
    ↓
Filter LISTEN + TCP  // Only listening TCP ports
    ↓
Parse with regex  // Extract: process, PID, address, port
    ↓
Filter localhost/wildcard bindings  // Only local services
    ↓
Return OpenPort[]  // port, process_name, pid
```

**Regex Pattern:**
```regex
^(\S+)\s+(\d+)\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+TCP\s+(.+?):(\d+)\s+\(LISTEN\)
 ─┬──  ─┬─                                        ──┬─  ─┬─
  │     │                                           │    │
  │     └─ PID                                      │    └─ Port
  └─ Process name                          Address ┘
```

**Supported Address Patterns:**
- `*` (all interfaces)
- `127.0.0.1` (localhost IPv4)
- `0.0.0.0` (all IPv4)
- `[::]` (all IPv6)
- `192.168.*` (local network)
- `10.*` (local network)

**Fallback Method: Port Scanning**

If lsof unavailable or fails:
1. Use configured port ranges from config
2. Async TCP connection attempts (tokio::spawn)
3. Timeout-based detection
4. Returns ports without process metadata

### 5. Service Detector (`detector.rs`)

**Note:** Used by IncrementalScanner only for new/changed services.

**Detection Strategy (Priority Order):**

```
Input: OpenPort (port, process_name?, pid?)
    ↓
┌───────────────────────────────────────┐
│ 1. Config Lookup                      │
│    Check known services in config     │
│    → Highest confidence               │
└───────────────────────────────────────┘
    ↓ (if not found)
┌───────────────────────────────────────┐
│ 2. HTTP Probe                         │
│    GET http://127.0.0.1:PORT/         │
│    Extract:                           │
│    • HTML <title> tag                 │
│    • Server header                    │
│    • X-Powered-By header              │
│    → High confidence                  │
└───────────────────────────────────────┘
    ↓ (if HTTP fails)
┌───────────────────────────────────────┐
│ 3. Process Name                       │
│    Format process name nicely:        │
│    • node → "Node.js Service"         │
│    • python → "Python Service"        │
│    → Medium confidence                │
└───────────────────────────────────────┘
    ↓ (if no process name)
┌───────────────────────────────────────┐
│ 4. Port Pattern Matching              │
│    Common port conventions:           │
│    • 3000-3003 → Node.js/React        │
│    • 5173 → Vite Dev Server           │
│    • 8080 → API Server                │
│    → Low confidence                   │
└───────────────────────────────────────┘
    ↓ (fallback)
┌───────────────────────────────────────┐
│ 5. Unknown Service                    │
│    Generic label with port number     │
└───────────────────────────────────────┘
```

**HTTP Probing Details:**
- Timeout: 2000ms (configurable)
- Follows redirects
- Parses HTML with `scraper` crate
- Measures response time
- Sets health status based on success

### 5. Web Server (`web/routes.rs`)

**State Management:**
```rust
AppState {
    config: Config  // Shared via Arc<>
}
```

**Route Handlers:**

1. **`GET /`** - Serve HTML UI
   - Returns embedded HTML from `include_str!`
   - Single-page application

2. **`GET /api/services`** - List Services
   - Triggers scan_and_detect()
   - Returns JSON array of ServiceInfo
   - Fresh scan on every request

3. **`POST /api/scan`** - Manual Refresh
   - Same as GET /api/services
   - Allows explicit refresh from UI

4. **`GET /assets/styles.css`** - CSS
   - Embedded stylesheet

5. **`GET /assets/app.js`** - JavaScript
   - Embedded application logic

**Scan and Detect Flow:**
```rust
async fn scan_and_detect(config: &Config) -> Vec<ServiceInfo> {
    // 1. Discover open ports
    let scanner = PortScanner::new(timeout, fallback_ports);
    let open_ports = scanner.scan().await;
    
    // 2. Identify each service
    let detector = ServiceDetector::new(config, probe_timeout);
    let services = open_ports
        .iter()
        .map(|port| detector.identify(port).await)
        .collect();
    
    // 3. Sort by port
    services.sort_by_key(|s| s.port);
    
    services
}
```

### 6. Data Models (`models.rs`)

**OpenPort**
```rust
struct OpenPort {
    port: u16,
    process_name: Option<String>,
    pid: Option<u32>,
}
```
- Represents a discovered listening port
- Optionally includes process metadata from lsof

**ServiceInfo**
```rust
struct ServiceInfo {
    port: u16,
    name: String,                    // Display name
    description: Option<String>,     // From config
    source: DetectionSource,         // How it was identified
    url: String,                     // http://127.0.0.1:PORT
    title: Option<String>,           // HTML <title>
    server_header: Option<String>,   // Server: header
    process_name: Option<String>,    // From lsof
    pid: Option<u32>,                // From lsof
    is_healthy: bool,                // HTTP probe succeeded
    response_time_ms: Option<u64>,   // HTTP response time
}
```
- Complete service information
- Serializes to JSON for API

**DetectionSource**
```rust
enum DetectionSource {
    Config,   // From config.toml
    Process,  // From process name
    Http,     // From HTTP probe
    Pattern,  // From port pattern matching
    Unknown,  // Fallback
}
```
- Indicates confidence level
- Helps debugging detection logic

### 7. Frontend (`ui/`)

**Architecture: Vanilla JavaScript SPA**

**HTML Structure (`index.html`):**
```html
<header>
  <h1>LocalWebs</h1>
  <button id="refresh-btn">Refresh</button>
</header>

<div id="loading">...</div>
<div id="services-grid"></div>
<div id="empty-state">...</div>
```

**JavaScript Flow (`app.js`):**
```javascript
// 1. Initial load
fetchServices()
  ↓
// 2. Display services
displayServices(services)
  ↓
// 3. Create service cards
createServiceCard(service) × N
  ↓
// 4. Add click handlers
card.onclick → window.open(buildServiceUrl(port))

// 5. Auto-refresh every 10s
setInterval(fetchServices, 10000)
```

**Dynamic URL Building:**
```javascript
function buildServiceUrl(port) {
    const hostname = window.location.hostname;
    return `http://${hostname}:${port}`;
}
```
- Uses current hostname from browser
- Ensures links work regardless of access method
- Supports: localhost, IP addresses, hostnames (e.g., fedora)

**Styling (`styles.css`):**
- Gradient background
- Card-based grid layout (CSS Grid)
- Responsive design (mobile-friendly)
- Status indicators (green/red dots)
- Hover effects
- Loading spinner animation

## Async/Concurrency Design

### Port Scanning Concurrency

```rust
// Fallback scanning: parallel TCP checks
for &port in &ports {
    tasks.push(tokio::spawn(async move {
        // Each port checked independently
        TcpStream::connect(addr).await
    }));
}

// All ports scanned simultaneously
for task in tasks {
    results.push(task.await);
}
```

**Benefits:**
- ~100ms to scan 20 ports (vs 2000ms+ serial)
- Non-blocking I/O with tokio
- Efficient resource usage

### Service Detection Concurrency

Currently sequential (could be parallelized):
```rust
for port in open_ports {
    let service = detector.identify(port).await;  // Sequential
    services.push(service);
}
```

**Potential optimization:**
```rust
// Parallel HTTP probing
let tasks: Vec<_> = open_ports
    .iter()
    .map(|port| tokio::spawn(detector.identify(port)))
    .collect();

let services = join_all(tasks).await;
```

## Performance Characteristics

### Scan Performance

**Typical Timings:**
- lsof execution: ~50-100ms
- lsof parsing: <1ms
- HTTP probe per service: 1-10ms (healthy)
- HTTP probe timeout: 2000ms (unhealthy)
- Total scan time: ~100-500ms (varies with service count)

**Scalability:**
- Tested with 23+ services
- Memory usage: ~10-20MB
- CPU: Minimal (I/O bound)

### HTTP Probing

**Optimization techniques:**
- Connection pooling (reqwest)
- Configurable timeouts
- Fail-fast on connection errors
- No retries (single attempt)

## Error Handling Strategy

### Scanner Errors
```rust
lsof fails
  ↓
Log warning
  ↓
Fall back to port scanning
  ↓
If that fails, return empty list (don't crash)
```

### Detector Errors
```rust
HTTP probe fails
  ↓
Try next detection method
  ↓
Always return a ServiceInfo (mark as unhealthy)
```

### Server Errors
```rust
All errors return 200 OK with JSON
Never return 500 errors
Client handles empty service lists gracefully
```

## Security Considerations

### Current Security Posture

**Binds to 0.0.0.0:**
- Accessible from all interfaces
- Suitable for trusted networks (Tailscale)
- NOT suitable for public internet without auth

**No Authentication:**
- Anyone with network access can view services
- Read-only (no mutations possible)
- Service URLs exposed

**Input Validation:**
- Port numbers validated (u16)
- Config file parsing safe (TOML)
- No user-generated content in HTML

### Recommended for Production

1. **Add authentication layer:**
   - Basic auth via reverse proxy
   - OAuth integration
   - API tokens

2. **HTTPS support:**
   - TLS termination
   - Certificate management

3. **Access control:**
   - IP allowlist
   - Firewall rules
   - VPN/Tailscale only

4. **Rate limiting:**
   - Prevent scan DoS
   - Per-IP limits

## Testing Strategy

### Unit Tests (TODO)
- Config parsing
- lsof output parsing
- Service detection logic
- URL building

### Integration Tests (TODO)
- Start test HTTP servers
- Verify detection
- Test all detection methods

### Manual Testing
- Multiple service types
- Various port ranges
- Different access methods (IP, hostname)
- Auto-refresh behavior

## Future Enhancements

### Planned Features

1. **Persistent storage:**
   - SQLite database
   - Service history
   - Uptime tracking

2. **Enhanced detection:**
   - Docker container integration
   - Custom regex patterns
   - User-defined detection rules

3. **Notifications:**
   - Service down alerts
   - New service discovery
   - Email/Slack integration

4. **Service management:**
   - Start/stop services
   - Favorite services
   - Service groups/tags

5. **Performance monitoring:**
   - Response time graphs
   - Historical data
   - Alerting thresholds

## Development Workflow

### Building
```bash
cargo build          # Debug build
cargo build --release  # Optimized build
```

### Running
```bash
cargo run -- --port 6543
```

### Hot Reload
```bash
cargo watch -x run
```

### Deployment
```bash
cargo build --release
./target/release/localwebs --port 6543
```

## Dependencies

### Core Dependencies
- **tokio** (1.x) - Async runtime
- **axum** (0.7) - Web framework
- **tower-http** (0.5) - Middleware
- **serde** (1.x) - Serialization
- **reqwest** (0.12) - HTTP client
- **scraper** (0.20) - HTML parsing
- **regex** (1.x) - Pattern matching
- **clap** (4.x) - CLI parsing
- **toml** (0.8) - Config parsing
- **anyhow** (1.x) - Error handling

### Why These Choices?

**Axum over Actix/Rocket:**
- Better async ergonomics
- Tower middleware ecosystem
- Excellent documentation
- Active development

**Reqwest over hyper:**
- Higher-level API
- Connection pooling built-in
- Easier timeout handling

**Scraper over html5ever:**
- CSS selector support
- jQuery-like API
- Sufficient for simple parsing

## Monitoring and Debugging

### Logs
Currently uses `println!` for startup message.

**Future: Add tracing:**
```rust
use tracing::{info, debug, warn, error};

info!("Portal starting on {}", addr);
debug!("Discovered {} ports", ports.len());
warn!("HTTP probe timeout for port {}", port);
error!("Failed to bind to address: {}", err);
```

### Metrics
Potential metrics to track:
- Scan duration
- Service count
- HTTP probe success rate
- Response times
- Error rates

### Health Endpoint
```rust
GET /health
→ { "status": "ok", "services": 23, "last_scan": "2026-05-11T00:15:00Z" }
```
