# LocalWebs - Local Service Portal

A Rust-based service discovery portal that helps you track all HTTP services running on your local machine. Never forget which port your services are running on again!

## Features

- **Automatic Discovery**: Uses `lsof` to find all listening ports on your system
- **Incremental Scanning**: Fast, efficient detection of service changes
  - Only probes new/changed services (not everything)
  - Detects new services within 5 seconds
  - 95% faster than traditional full scans (~115ms vs 500ms)
  - Perfect for development environments with frequent changes
- **Smart Identification**: Identifies services through multiple methods:
  - HTTP probing (page titles, server headers)
  - Process names and PIDs from lsof
  - Port-based pattern matching
  - User-defined configuration
- **Live Dashboard**: Clean web interface showing all discovered services
- **Instant Responses**: API responds in 2-5ms (cached results)
- **One-click Access**: Click any service card to open it in a new tab
- **Dynamic URLs**: Service links match your access method (hostname/IP)

## Installation

### Quick Install (Recommended)

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/nvquanghuy/localwebs/master/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/nvquanghuy/localwebs/master/install.ps1 | iex
```

### Alternative Methods

**From Cargo:**
```bash
cargo install --git https://github.com/nvquanghuy/localwebs.git
```

**From Source:**
```bash
git clone https://github.com/nvquanghuy/localwebs.git
cd localwebs
make install
# Or: cargo build --release && cp target/release/localwebs ~/.local/bin/
```

**Download Binary:**
- Go to [Releases](https://github.com/nvquanghuy/localwebs/releases/latest)
- Download for your platform
- Extract and run

## Usage

### Basic Usage

```bash
# Run with default settings (port 4444)
./target/release/localwebs

# Or just
cargo run
```

Then open http://127.0.0.1:4444 in your browser (or whichever port you configured).

### Custom Configuration

```bash
# Use a different port
./localwebs --port 7777

# Use a custom config file
./localwebs --config my-config.toml
```

## Configuration

Create a `config.toml` file to customize the portal:

```toml
[portal]
port = 4444                # Port for the portal itself
scan_timeout_ms = 500      # Timeout for port scanning
probe_timeout_ms = 2000    # Timeout for HTTP probes

[scan_ranges]
# Fallback ports to scan if lsof is unavailable
ports = [3000, 3001, 4200, 5000, 5173, 8000, 8080, 8081, 8888, 9000]

# Define known services for better identification
[[services]]
port = 3000
name = "React App"
description = "Main frontend application"
url_path = "/"

[[services]]
port = 8080
name = "API Server"
description = "Backend REST API"
url_path = "/api/health"
```

## How It Works

### Incremental Scanning Architecture

LocalWebs uses an efficient incremental scanning approach optimized for development environments:

1. **Initial Scan**: On startup, discovers all current services
2. **Background Monitoring**: Scans every 5 seconds for changes
3. **Smart Detection**: 
   - Quick lsof scan (~50ms) to get current port list
   - Compares with known services
   - Only probes NEW or CHANGED services with HTTP
   - Removes disappeared services from cache
4. **Periodic Full Refresh**: Every 5 minutes, re-probes all services to update health status
5. **Instant API Responses**: Serves from cache (2-5ms response time)

### Service Identification

1. **Port Discovery**: Runs `lsof -i -P -n` to find all listening TCP ports
2. **Process Metadata**: Extracts process names and PIDs from lsof output
3. **HTTP Probing**: (Only for new/changed services)
   - Extract HTML page titles
   - Read server headers
   - Detect frameworks and technologies
4. **Smart Fallback**: If a service doesn't respond to HTTP, uses process name or port patterns
5. **Change Detection**:
   - New service? → Probe it immediately
   - Removed service? → Clean from cache
   - Process changed? → Re-probe to update info

## Service Detection Priority

1. **Config Lookup** - Known services defined in `config.toml`
2. **Process Name** - From lsof (e.g., "node", "python", "nginx")
3. **HTTP Probe** - Page title, server headers, framework detection
4. **Port Pattern** - Common development ports (3000 → Node.js, 5173 → Vite, etc.)
5. **Fallback** - "Unknown Service"

## API Endpoints

- `GET /` - Web interface
- `GET /api/services` - JSON list of all discovered services (cached, ~2-5ms)
- `POST /api/scan` - Force full refresh scan (~500ms)
- `GET /api/stats` - Scanner statistics (service count, last scan time)

### Example API Response

```json
{
  "port": 3000,
  "name": "Holistics Docs",
  "description": null,
  "source": "http",
  "url": "http://127.0.0.1:3000",
  "title": "Holistics Docs (4.0)",
  "server_header": null,
  "process_name": "node",
  "pid": 1771402,
  "is_healthy": true,
  "response_time_ms": 8
}
```

## Performance

| Metric | Value | Notes |
|--------|-------|-------|
| API Response Time | 2-5ms | Cached results |
| Incremental Scan | ~115ms | No changes detected |
| New Service Scan | ~130ms | Includes HTTP probe |
| Full Scan | ~500ms | Manual refresh only |
| Detection Latency | <5 seconds | New/removed services |

Perfect for development environments with frequently changing services!

## Requirements

- Rust 1.70+
- `lsof` command (pre-installed on most Unix-like systems)
- Modern web browser

## Development

```bash
# Run in development mode
cargo run

# Build for release
cargo build --release

# Run tests (when added)
cargo test
```

## Technology Stack

- **tokio** - Async runtime
- **axum** - Web framework
- **reqwest** - HTTP client
- **scraper** - HTML parsing
- **serde** - Serialization
- **clap** - CLI arguments

## License

MIT

## Contributing

Contributions welcome! Feel free to open issues or submit pull requests.
