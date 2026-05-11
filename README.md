# LocalWebs - Local Service Portal

A Rust-based service discovery portal that helps you track all HTTP services running on your local machine. Never forget which port your services are running on again!

## Features

- **Automatic Discovery**: Uses `lsof` to find all listening ports on your system
- **Smart Identification**: Identifies services through multiple methods:
  - HTTP probing (page titles, server headers)
  - Process names and PIDs from lsof
  - Port-based pattern matching
  - User-defined configuration
- **Live Dashboard**: Clean web interface showing all discovered services
- **Auto-refresh**: Updates every 10 seconds
- **One-click Access**: Click any service card to open it in a new tab

## Installation

```bash
# Build the project
cargo build --release

# Binary will be available at
./target/release/localwebs
```

## Usage

### Basic Usage

```bash
# Run with default settings (port 9999)
./target/release/localwebs

# Or just
cargo run
```

Then open http://127.0.0.1:9999 in your browser (or whichever port you configured).

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
port = 9999                # Port for the portal itself
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

1. **Port Discovery**: Runs `lsof -i -P -n` to find all listening TCP ports
2. **Process Metadata**: Extracts process names and PIDs from lsof output
3. **HTTP Probing**: Sends GET requests to discovered ports to:
   - Extract HTML page titles
   - Read server headers
   - Detect frameworks and technologies
4. **Smart Fallback**: If a service doesn't respond to HTTP, uses process name or port patterns
5. **Display**: Shows all discovered services in a clean, card-based interface

## Service Detection Priority

1. **Config Lookup** - Known services defined in `config.toml`
2. **Process Name** - From lsof (e.g., "node", "python", "nginx")
3. **HTTP Probe** - Page title, server headers, framework detection
4. **Port Pattern** - Common development ports (3000 → Node.js, 5173 → Vite, etc.)
5. **Fallback** - "Unknown Service"

## API Endpoints

- `GET /` - Web interface
- `GET /api/services` - JSON list of all discovered services
- `POST /api/scan` - Trigger a fresh scan

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
