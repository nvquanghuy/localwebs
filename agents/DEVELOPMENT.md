# LocalWebs Development Guide

## Getting Started

### Prerequisites

- Rust 1.70+ (2021 edition)
- `lsof` command (pre-installed on most Unix-like systems)
- Basic understanding of async Rust (tokio)

### Initial Setup

```bash
# Clone the repo
git clone https://github.com/nvquanghuy/localwebs.git
cd localwebs

# Build
cargo build

# Run
cargo run

# Run with custom port
cargo run -- --port 6543
```

### Development Tools

```bash
# Install cargo-watch for hot reload
cargo install cargo-watch

# Run with auto-reload
cargo watch -x run

# Run with specific features
cargo watch -x 'run -- --port 7777 --config dev-config.toml'
```

## Project Structure

```
localwebs/
├── src/
│   ├── main.rs           # Entry point, CLI setup
│   ├── config.rs         # Configuration management
│   ├── models.rs         # Data structures
│   ├── scanner.rs        # Port discovery (lsof)
│   ├── detector.rs       # Service identification
│   ├── web/
│   │   ├── mod.rs        # Web module export
│   │   └── routes.rs     # Axum routes and handlers
│   └── ui/
│       ├── index.html    # Main UI
│       ├── styles.css    # Styling
│       └── app.js        # Frontend logic
├── agents/               # Internal development docs
│   ├── ARCHITECTURE.md   # System architecture
│   ├── DEVELOPMENT.md    # This file
│   └── TESTING.md        # Testing guide
├── config.toml           # Default configuration
├── Cargo.toml            # Rust dependencies
└── README.md             # User-facing documentation
```

## Code Organization

### Module Dependencies

```
main.rs
  ├─> config.rs
  ├─> models.rs
  ├─> scanner.rs ──> models.rs
  ├─> detector.rs ──> models.rs, config.rs
  └─> web/
      └─> routes.rs ──> scanner.rs, detector.rs, config.rs, models.rs
```

### Data Flow

```
User Request (GET /api/services)
  ↓
routes.rs: list_services()
  ↓
routes.rs: scan_and_detect()
  ↓
scanner.rs: scan() → Vec<OpenPort>
  ↓
detector.rs: identify() → ServiceInfo (for each port)
  ↓
JSON Response
```

## Adding New Features

### Adding a New Detection Method

1. **Add to DetectionSource enum** (`models.rs`):
```rust
pub enum DetectionSource {
    Config,
    Process,
    Http,
    Pattern,
    Docker,  // New!
    Unknown,
}
```

2. **Implement detection logic** (`detector.rs`):
```rust
async fn detect_docker(&self, port: &OpenPort) -> Option<ServiceInfo> {
    // Query Docker API
    // Match container ports
    // Return service info
}
```

3. **Add to detection priority** (`detector.rs: identify()`):
```rust
pub async fn identify(&self, port: OpenPort) -> ServiceInfo {
    // ... existing checks ...
    
    // New docker check
    if let Some(info) = self.detect_docker(&port).await {
        return info;
    }
    
    // ... fallback logic ...
}
```

### Adding Configuration Options

1. **Update config structures** (`config.rs`):
```rust
pub struct PortalConfig {
    pub port: u16,
    pub scan_timeout_ms: u64,
    pub probe_timeout_ms: u64,
    pub enable_docker: bool,  // New!
}
```

2. **Add default value**:
```rust
impl Default for PortalConfig {
    fn default() -> Self {
        Self {
            port: 9999,
            scan_timeout_ms: 500,
            probe_timeout_ms: 2000,
            enable_docker: false,  // New!
        }
    }
}
```

3. **Update config.toml**:
```toml
[portal]
port = 9999
scan_timeout_ms = 500
probe_timeout_ms = 2000
enable_docker = false
```

### Adding API Endpoints

1. **Define handler** (`web/routes.rs`):
```rust
async fn service_history(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<HistoryEntry>> {
    // Implementation
    Json(history)
}
```

2. **Register route** (`web/routes.rs: create_router()`):
```rust
Router::new()
    .route("/", get(serve_index))
    .route("/api/services", get(list_services))
    .route("/api/history", get(service_history))  // New!
    // ...
```

### Adding Frontend Features

1. **Update HTML** (`ui/index.html`):
```html
<div id="history-view" style="display: none;">
    <!-- History UI -->
</div>
```

2. **Add JavaScript** (`ui/app.js`):
```javascript
async function fetchHistory() {
    const response = await fetch('/api/history');
    const history = await response.json();
    displayHistory(history);
}
```

3. **Update styles** (`ui/styles.css`):
```css
.history-card {
    background: white;
    padding: 1rem;
    /* ... */
}
```

## Common Development Tasks

### Testing lsof Parsing

```rust
// In scanner.rs, add temporary test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lsof() {
        let output = "node    12345  user   20u  IPv4 0x123  0t0 TCP *:3000 (LISTEN)\n";
        let scanner = PortScanner::new(500, vec![]);
        let ports = scanner.parse_lsof_output(output);
        
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 3000);
        assert_eq!(ports[0].process_name, Some("node".to_string()));
    }
}
```

```bash
cargo test
```

### Debugging HTTP Probes

Add debug printing in `detector.rs`:

```rust
pub async fn http_probe(&self, port: &OpenPort, url: &str) -> Option<ServiceInfo> {
    println!("🔍 Probing {}", url);
    
    let response = self.http_client.get(url).send().await.ok()?;
    println!("  ✓ Status: {}", response.status());
    
    let headers = response.headers();
    println!("  Headers: {:?}", headers);
    
    // ... rest of logic
}
```

### Testing Service Detection

Start test services on different ports:

```bash
# Terminal 1: Python HTTP server
python3 -m http.server 8000

# Terminal 2: Node server
npx serve -p 3000

# Terminal 3: Simple HTTP echo
nc -l 5000

# Terminal 4: Run LocalWebs
cargo run -- --port 6543
```

Then check http://localhost:6543/api/services to see detection results.

### Frontend Development

Since assets are embedded via `include_str!`, you need to rebuild on every change:

```bash
# Watch mode - rebuilds on file changes
cargo watch -x run

# Or use a more targeted approach:
cargo watch -w src/ui -x run
```

**Tip**: For faster frontend iteration, temporarily serve files from disk:

```rust
// In routes.rs, comment out include_str! and use:
async fn serve_index() -> Html<String> {
    let html = tokio::fs::read_to_string("src/ui/index.html")
        .await
        .unwrap_or_default();
    Html(html)
}
```

Remember to revert before committing!

## Performance Profiling

### Measuring Scan Time

Add timing in `routes.rs`:

```rust
async fn scan_and_detect(config: &Config) -> Vec<ServiceInfo> {
    use std::time::Instant;
    
    let start = Instant::now();
    
    let scanner = PortScanner::new(...);
    let open_ports = scanner.scan().await;
    println!("🕐 Port scan: {:?}", start.elapsed());
    
    let detect_start = Instant::now();
    let detector = ServiceDetector::new(...);
    // ... detection logic
    println!("🕐 Service detection: {:?}", detect_start.elapsed());
    
    println!("🕐 Total: {:?}", start.elapsed());
    
    services
}
```

### Memory Profiling

```bash
# Install heaptrack
sudo dnf install heaptrack  # Fedora
sudo apt install heaptrack  # Ubuntu

# Profile
heaptrack ./target/release/localwebs

# Analyze
heaptrack_gui heaptrack.localwebs.*.gz
```

## Common Issues and Solutions

### Issue: lsof Not Found

**Symptom:** Empty service list, fallback to port scanning.

**Solution:**
```bash
# Install lsof
sudo dnf install lsof      # Fedora/RHEL
sudo apt install lsof      # Ubuntu/Debian
brew install lsof          # macOS
```

### Issue: Port Already in Use

**Symptom:** `Error: Address already in use (os error 98)`

**Solution:**
```bash
# Find what's using the port
lsof -i :9999

# Kill it or use a different port
cargo run -- --port 6543
```

### Issue: HTTP Probes Timing Out

**Symptom:** All services show as unhealthy.

**Solution:**
```toml
# Increase timeout in config.toml
[portal]
probe_timeout_ms = 5000  # Increase from 2000
```

### Issue: CORS Errors in Browser

**Symptom:** Console errors about CORS when accessing API.

**Solution:** Already handled in `routes.rs`:
```rust
.layer(CorsLayer::permissive())
```

If issues persist, verify tower-http version and features in Cargo.toml.

### Issue: Service Links Don't Work

**Symptom:** Clicking services opens 127.0.0.1 instead of current hostname.

**Solution:** Already fixed in `app.js` via `buildServiceUrl()`. Verify:
```javascript
function buildServiceUrl(port) {
    const hostname = window.location.hostname;
    return `http://${hostname}:${port}`;
}
```

## Code Style Guidelines

### Rust Style

Follow standard Rust conventions:
- `snake_case` for functions and variables
- `PascalCase` for types
- `SCREAMING_SNAKE_CASE` for constants
- Prefer explicit types in function signatures
- Use `?` for error propagation
- Add doc comments for public APIs

```rust
/// Scans for open ports on localhost
/// 
/// # Returns
/// A vector of discovered ports with process metadata
pub async fn scan(&self) -> Vec<OpenPort> {
    // Implementation
}
```

### Async Patterns

**Prefer:**
```rust
async fn process_ports(ports: Vec<Port>) -> Vec<Result> {
    let tasks: Vec<_> = ports
        .into_iter()
        .map(|p| tokio::spawn(process(p)))
        .collect();
    
    join_all(tasks).await
}
```

**Avoid:**
```rust
async fn process_ports(ports: Vec<Port>) -> Vec<Result> {
    let mut results = vec![];
    for port in ports {
        results.push(process(port).await);  // Sequential!
    }
    results
}
```

### Error Handling

**Prefer Result<T> for recoverable errors:**
```rust
pub fn load_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .context("Failed to read config")?;
    
    toml::from_str(&contents)
        .context("Failed to parse config")
}
```

**Use Option<T> for missing data:**
```rust
pub fn find_service(&self, port: u16) -> Option<&Service> {
    self.services.iter().find(|s| s.port == port)
}
```

### Frontend Style

- Use `const` for values that don't change
- Prefer `async/await` over `.then()`
- Use template literals for strings
- Add comments for non-obvious logic

```javascript
// Good
const hostname = window.location.hostname;
const url = `http://${hostname}:${port}`;

// Avoid
var hostname = window.location.hostname;
var url = 'http://' + hostname + ':' + port;
```

## Git Workflow

### Branch Naming

- `feature/service-groups` - New features
- `fix/port-parsing` - Bug fixes
- `refactor/async-scanning` - Code refactoring
- `docs/api-reference` - Documentation

### Commit Messages

Follow conventional commits:

```
feat: add Docker container detection
fix: correct lsof regex for IPv6 addresses
refactor: extract HTTP probing to separate module
docs: add API endpoint documentation
chore: update dependencies
```

### Pull Request Process

1. Create feature branch
2. Make changes
3. Test locally
4. Update documentation if needed
5. Create PR with description
6. Wait for review
7. Address feedback
8. Merge when approved

## Building for Production

### Release Build

```bash
cargo build --release
```

Binary size optimization:

```toml
# Add to Cargo.toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

### Binary Size Comparison

```bash
# Debug build
ls -lh target/debug/localwebs
# ~50MB

# Release build (default)
ls -lh target/release/localwebs
# ~10MB

# Release build (optimized)
ls -lh target/release/localwebs
# ~5MB
```

### Cross-Compilation

For different platforms:

```bash
# Install cross
cargo install cross

# Build for different targets
cross build --release --target x86_64-unknown-linux-musl
cross build --release --target aarch64-unknown-linux-gnu
```

## Deployment Options

### Systemd Service

```ini
# /etc/systemd/system/localwebs.service
[Unit]
Description=LocalWebs Service Portal
After=network.target

[Service]
Type=simple
User=localwebs
WorkingDirectory=/opt/localwebs
ExecStart=/opt/localwebs/localwebs --port 9999
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable localwebs
sudo systemctl start localwebs
```

### Docker Container

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y lsof && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/localwebs /usr/local/bin/
EXPOSE 9999
CMD ["localwebs", "--port", "9999"]
```

```bash
docker build -t localwebs .
docker run -p 9999:9999 localwebs
```

### Reverse Proxy (nginx)

```nginx
server {
    listen 80;
    server_name localwebs.example.com;

    location / {
        proxy_pass http://127.0.0.1:9999;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Resources

### Documentation
- [Tokio Docs](https://docs.rs/tokio)
- [Axum Guide](https://docs.rs/axum)
- [Rust Async Book](https://rust-lang.github.io/async-book/)

### Similar Projects
- [Traefik](https://traefik.io/) - Service discovery for containers
- [Consul](https://www.consul.io/) - Service mesh
- [Portainer](https://www.portainer.io/) - Container management

### Useful Tools
- `cargo-watch` - Auto-rebuild on changes
- `cargo-audit` - Security vulnerability scanning
- `cargo-outdated` - Check for outdated dependencies
- `cargo-tree` - Visualize dependency tree

## Getting Help

### Internal Resources
- Architecture docs: `agents/ARCHITECTURE.md`
- Testing guide: `agents/TESTING.md`
- README: User-facing documentation

### External Help
- Rust Discord: https://discord.gg/rust-lang
- Tokio Discord: https://discord.gg/tokio
- Stack Overflow: Tag `rust` + `axum`

## Contributing Guidelines

When contributing to LocalWebs:

1. **Check existing issues** before starting work
2. **Open an issue** for discussion on major changes
3. **Write tests** for new features
4. **Update documentation** for API changes
5. **Follow code style** guidelines
6. **Keep PRs focused** on a single feature/fix
7. **Add comments** for complex logic

### Code Review Checklist

- [ ] Code compiles without warnings
- [ ] Tests pass (when added)
- [ ] Documentation updated
- [ ] No hardcoded values (use config)
- [ ] Error handling in place
- [ ] No panics in production code
- [ ] Async code uses proper patterns
- [ ] Frontend changes tested in browser

## Version History

See [GitHub Releases](https://github.com/nvquanghuy/localwebs/releases) for changelog.
