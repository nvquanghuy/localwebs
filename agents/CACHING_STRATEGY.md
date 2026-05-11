# Caching Strategy: Balancing Speed vs Freshness

## Problem Statement

Current implementation scans on every API request:
- **Scan time**: 100-500ms per request
- **Wasteful**: Multiple users/tabs trigger duplicate scans
- **Slow UX**: Every refresh takes half a second
- **Resource intensive**: Spawns lsof process repeatedly

## Performance Analysis

### Current Bottlenecks

```
GET /api/services
  ↓
lsof execution         ~50-100ms
  ↓
lsof parsing          ~1-5ms
  ↓
HTTP probing (x20)    ~50-200ms (parallel)
  ↓
JSON serialization    ~1-5ms
  ↓
Total: ~150-500ms
```

### Scan Breakdown

- **Fast operations** (can repeat frequently):
  - lsof execution: 50-100ms
  - Port list generation
  
- **Slow operations** (should cache):
  - HTTP probing each service: 1-10ms × N services
  - Timeout handling: 2000ms per unhealthy service
  - HTML parsing

## Proposed Solutions

### Option 1: Simple Time-Based Cache ⭐ **Recommended**

**Implementation:**
```rust
struct ServiceCache {
    data: Arc<RwLock<CachedData>>,
    last_scan: Arc<RwLock<Instant>>,
    ttl: Duration,
}

struct CachedData {
    services: Vec<ServiceInfo>,
    timestamp: Instant,
}

impl ServiceCache {
    async fn get_services(&self, config: &Config) -> Vec<ServiceInfo> {
        let last = *self.last_scan.read().await;
        
        if last.elapsed() < self.ttl {
            // Serve from cache
            return self.data.read().await.services.clone();
        }
        
        // Cache expired, refresh
        let services = scan_and_detect(config).await;
        
        *self.data.write().await = CachedData {
            services: services.clone(),
            timestamp: Instant::now(),
        };
        *self.last_scan.write().await = Instant::now();
        
        services
    }
    
    async fn invalidate(&self) {
        // Force refresh on next request
        *self.last_scan.write().await = Instant::now() - self.ttl;
    }
}
```

**API Changes:**
```rust
// Fast: returns cached data
GET /api/services  → 1-5ms response

// Force refresh: triggers new scan
POST /api/scan  → 150-500ms response, invalidates cache
```

**Configuration:**
```toml
[portal]
cache_ttl_seconds = 10  # Default: 10 seconds
```

**Pros:**
- ✅ Simple to implement (~50 lines)
- ✅ Instant responses (cached)
- ✅ Manual refresh still available
- ✅ No background tasks
- ✅ Thread-safe with RwLock

**Cons:**
- ❌ First request after expiry is slow
- ❌ "Thundering herd" if multiple requests hit expired cache
- ❌ Can miss services for up to TTL duration

**Best for:**
- Simple deployments
- Single-user scenarios
- When 10s staleness is acceptable

---

### Option 2: Background Scanning Task ⭐ **Best UX**

**Implementation:**
```rust
struct ServiceState {
    services: Arc<RwLock<Vec<ServiceInfo>>>,
    last_update: Arc<RwLock<Instant>>,
}

async fn background_scanner(
    state: Arc<ServiceState>,
    config: Config,
    interval: Duration,
) {
    let mut interval = tokio::time::interval(interval);
    
    loop {
        interval.tick().await;
        
        // Scan in background
        let services = scan_and_detect(&config).await;
        
        // Update shared state
        *state.services.write().await = services;
        *state.last_update.write().await = Instant::now();
        
        println!("📡 Background scan complete: {} services", services.len());
    }
}

// In main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let state = Arc::new(ServiceState::default());
    
    // Spawn background scanner
    tokio::spawn(background_scanner(
        state.clone(),
        config.clone(),
        Duration::from_secs(10),
    ));
    
    // API always reads from shared state (instant)
    let app = create_router(state);
    // ...
}
```

**API Response Time:**
```
GET /api/services  → 1-5ms (always fast)
POST /api/scan     → 1-5ms + triggers immediate background scan
```

**Configuration:**
```toml
[portal]
scan_interval_seconds = 10  # Background scan frequency
```

**Pros:**
- ✅ Always fast responses (1-5ms)
- ✅ Continuous monitoring
- ✅ No "first request" penalty
- ✅ Predictable scan intervals
- ✅ Can add scan history tracking

**Cons:**
- ❌ Scans even when no one is watching
- ❌ Slightly more complex (background task management)
- ❌ Need to handle task cancellation on shutdown

**Best for:**
- Production deployments
- Multiple users
- Dashboard/monitoring scenarios
- When <10ms response time is critical

---

### Option 3: Lazy Background Refresh (Hybrid)

**Concept:**
- Cache serves fast responses
- When cache is "getting stale" (e.g., 70% of TTL), trigger background refresh
- Next request gets fresh data without waiting

**Implementation:**
```rust
impl ServiceCache {
    async fn get_services(&self, config: &Config) -> Vec<ServiceInfo> {
        let age = self.last_scan.read().await.elapsed();
        let stale_threshold = self.ttl * 7 / 10;  // 70%
        
        // If cache is getting stale, refresh in background
        if age > stale_threshold && age < self.ttl {
            let config = config.clone();
            let cache = self.clone();
            
            tokio::spawn(async move {
                let services = scan_and_detect(&config).await;
                cache.update(services).await;
            });
        }
        
        // Always return current cache (might be slightly stale)
        self.data.read().await.services.clone()
    }
}
```

**Pros:**
- ✅ Fast responses
- ✅ No wasted scans when idle
- ✅ Proactive refresh before expiry
- ✅ Best of both worlds

**Cons:**
- ❌ More complex logic
- ❌ Can still have stale data (up to TTL)
- ❌ Background task spawning overhead

**Best for:**
- Variable load scenarios
- When you want both performance and efficiency

---

### Option 4: Incremental Scanning

**Concept:**
- Track known ports
- Quick lsof scan to detect changes (50ms)
- Only probe new/changed ports (saves time)
- Full re-probe periodically

**Implementation:**
```rust
struct IncrementalScanner {
    known_ports: Arc<RwLock<HashMap<u16, ServiceInfo>>>,
    last_full_scan: Arc<RwLock<Instant>>,
}

impl IncrementalScanner {
    async fn scan(&self, config: &Config) -> Vec<ServiceInfo> {
        // Quick lsof scan
        let current_ports = quick_port_scan().await;  // Just ports, no HTTP
        let mut known = self.known_ports.write().await;
        
        // Detect changes
        let new_ports: Vec<_> = current_ports
            .iter()
            .filter(|p| !known.contains_key(&p.port))
            .collect();
        
        let removed_ports: Vec<_> = known
            .keys()
            .filter(|p| !current_ports.iter().any(|cp| cp.port == **p))
            .copied()
            .collect();
        
        // Only probe new ports
        if !new_ports.is_empty() {
            let detector = ServiceDetector::new(config.clone(), 2000);
            for port in new_ports {
                let service = detector.identify(port.clone()).await;
                known.insert(port.port, service);
            }
        }
        
        // Remove disappeared services
        for port in removed_ports {
            known.remove(&port);
        }
        
        // Full re-probe every 5 minutes
        if self.last_full_scan.read().await.elapsed() > Duration::from_secs(300) {
            // Re-probe all existing services
            // (Updates health status, response times, etc.)
        }
        
        known.values().cloned().collect()
    }
}
```

**Pros:**
- ✅ Fast incremental updates (50-100ms)
- ✅ Only slow when new services appear
- ✅ Detects changes quickly
- ✅ Efficient resource usage

**Cons:**
- ❌ Complex state management
- ❌ Need to handle port reuse
- ❌ Stale service info between full scans

**Best for:**
- Environments with frequent port changes
- When probe time dominates (many services)
- Development machines with constantly changing services

---

### Option 5: Two-Phase Loading (Progressive Enhancement)

**Concept:**
- Phase 1: Return basic port list immediately (fast)
- Phase 2: Stream detailed service info as probing completes

**Implementation:**

**API Design:**
```javascript
// Client-side
async function loadServices() {
    // Phase 1: Get port list instantly
    const response = await fetch('/api/ports');  // 50ms
    const ports = await response.json();
    
    // Render placeholder cards immediately
    renderPlaceholders(ports);
    
    // Phase 2: Fetch full details
    for (const port of ports) {
        const details = await fetch(`/api/services/${port}`);
        const service = await details.json();
        updateServiceCard(port, service);
    }
}
```

**Or with WebSocket:**
```rust
// Server streams updates
ws.send({"type": "port_discovered", "port": 3000})
ws.send({"type": "service_identified", "port": 3000, "data": {...}})
```

**Pros:**
- ✅ Instant initial render
- ✅ Progressive enhancement
- ✅ Feels very fast
- ✅ Can show scanning progress

**Cons:**
- ❌ Requires frontend changes
- ❌ More complex protocol (WebSocket)
- ❌ Still does full scan work

**Best for:**
- Large number of services (20+)
- When perceived performance matters
- Interactive dashboards

---

## Recommendation Matrix

| Scenario | Best Option | TTL/Interval | Why |
|----------|-------------|--------------|-----|
| **Single user, simple** | Option 1: Simple Cache | 10-30s | Easy to implement, good enough |
| **Production, multi-user** | Option 2: Background Task | 10-15s | Always fast, predictable |
| **Development machine** | Option 4: Incremental | N/A | Adapts to frequent changes |
| **20+ services** | Option 5: Two-Phase | N/A | Progressive loading feels faster |
| **Dashboard/monitoring** | Option 2: Background Task | 5-10s | Continuous updates |

## Implementation Plan: Option 2 (Background Task) ⭐

### Phase 1: Core Implementation

1. **Add shared state** (`main.rs`):
```rust
pub struct AppState {
    pub config: Config,
    pub services: Arc<RwLock<Vec<ServiceInfo>>>,
    pub last_update: Arc<RwLock<Instant>>,
}
```

2. **Background scanner** (`scanner.rs`):
```rust
pub async fn start_background_scanner(
    state: Arc<AppState>,
    interval_secs: u64,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            Duration::from_secs(interval_secs)
        );
        
        loop {
            interval.tick().await;
            
            let services = scan_and_detect(&state.config).await;
            *state.services.write().await = services;
            *state.last_update.write().await = Instant::now();
        }
    })
}
```

3. **Update routes** (`web/routes.rs`):
```rust
async fn list_services(State(state): State<Arc<AppState>>) -> Json<Vec<ServiceInfo>> {
    // Instant response from cache
    let services = state.services.read().await.clone();
    Json(services)
}

async fn trigger_scan(State(state): State<Arc<AppState>>) -> Json<ScanResponse> {
    // Trigger immediate scan
    let services = scan_and_detect(&state.config).await;
    *state.services.write().await = services.clone();
    *state.last_update.write().await = Instant::now();
    
    Json(ScanResponse {
        services,
        scanned_at: Instant::now(),
    })
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    Json(StatusResponse {
        service_count: state.services.read().await.len(),
        last_update: *state.last_update.read().await,
        uptime: /* ... */,
    })
}
```

4. **Configuration** (`config.toml`):
```toml
[portal]
port = 9999
scan_interval_seconds = 10  # Background scan frequency
initial_scan_on_startup = true
```

### Phase 2: Enhanced Features

1. **Scan history tracking**:
```rust
pub struct ScanHistory {
    timestamp: Instant,
    duration_ms: u64,
    service_count: usize,
    new_services: Vec<u16>,
    removed_services: Vec<u16>,
}
```

2. **Adaptive intervals**:
```rust
// Scan more frequently when changes detected
if services_changed {
    interval = Duration::from_secs(5);
} else {
    interval = Duration::from_secs(30);
}
```

3. **Health monitoring**:
```rust
// Track service uptime
// Alert on service down
// Historical response time graphs
```

### Phase 3: Performance Metrics

Add endpoint to monitor scanner performance:

```rust
GET /api/metrics
{
    "last_scan_duration_ms": 156,
    "average_scan_duration_ms": 142,
    "service_count": 23,
    "uptime_seconds": 3600,
    "total_scans": 360,
    "cache_hit_rate": 0.98
}
```

## Testing Strategy

### Benchmark Current vs Cached

```bash
# Current (no cache)
time curl http://localhost:6543/api/services
# Expected: 150-500ms

# With cache
time curl http://localhost:6543/api/services
# Expected: 1-5ms (95% improvement)
```

### Load Test

```bash
# Concurrent requests
hey -n 1000 -c 50 http://localhost:6543/api/services

# Without cache: Many timeouts, slow
# With cache: All succeed, <10ms avg
```

## Migration Path

1. **Implement Option 1 first** (simple cache)
   - Quick win, immediate improvement
   - 1-2 hours work
   
2. **Upgrade to Option 2** (background task)
   - Better UX, production-ready
   - 2-3 hours work
   
3. **Add Option 4** (incremental) if needed
   - Optimization for high-churn environments
   - 4-6 hours work

## Configuration Examples

### Development (fast updates):
```toml
[portal]
scan_interval_seconds = 5
```

### Production (balanced):
```toml
[portal]
scan_interval_seconds = 15
```

### Low-resource (infrequent):
```toml
[portal]
scan_interval_seconds = 60
```

## Open Questions

1. **What happens on startup?**
   - Option A: Wait for first scan (slow startup)
   - Option B: Start with empty list, scan in background (fast startup)
   - **Recommendation**: Option A for small scans, Option B for large

2. **How to handle scan failures?**
   - Keep serving stale data
   - Show error state in UI
   - Retry with exponential backoff

3. **Should scan interval be dynamic?**
   - Yes, adapt based on change frequency
   - Faster when changes detected
   - Slower when stable

4. **How to signal freshness to users?**
   - Add "Last updated: 5s ago" in UI
   - Spinner during active scan
   - Toast notification on new services
