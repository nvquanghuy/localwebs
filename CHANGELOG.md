# Changelog

All notable changes to LocalWebs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-05-17

### Added
- `localwebs --version` now reports the packaged version for release verification.

## [0.1.1] - 2026-05-17

### Changed
- Default portal port is now 4444.
- Configured fallback ports are probed even when `lsof` is unavailable or incomplete.
- HTTP probing no longer depends on OpenSSL/TLS, improving release build reliability for musl targets.

## [0.1.0] - 2025-05-11

### Added
- **Incremental scanning** for fast service detection (95% faster than full scans)
  - Detects new/removed/changed services automatically
  - Background scanning every 5 seconds
  - Full re-probe every 5 minutes
  - ~115ms scan time vs 500ms traditional
- **Process start time tracking**
  - Sort by "Recently Started" to see newest services first
  - Display "Started Xm/Xh/Xd ago" in list view
  - Perfect for dev: new service appears at top immediately
- **Dual view modes**
  - Grid view: Card-based layout for detailed service info
  - List view: Compact table for viewing many services
  - Toggle between views with button
- **Flexible sorting**
  - Sort by: Recently Started, Port, Name, Status, Response Time
  - Toggle ascending/descending order
  - Preferences persist in localStorage
- **Service discovery**
  - Uses `lsof` to detect listening ports
  - HTTP probing for service identification
  - Process name and PID tracking
  - Multi-layered detection: config → process → HTTP → pattern
- **Smart identification**
  - Page titles from HTML
  - Server headers
  - Process names
  - Port pattern matching
  - User-defined config
- **Web UI**
  - Real-time service grid/list
  - Auto-refresh every 10 seconds
  - Click to open services
  - Responsive design
  - Dark gradient theme
- **Dynamic URLs**
  - Service links match your access method
  - Works with localhost, IPs, hostnames
  - Perfect for VPS/Tailscale networks
- **Installation**
  - One-liner install scripts (Linux/macOS/Windows)
  - Pre-built binaries for 5 platforms
  - GitHub Actions automated releases

### Performance
- API responses: 2-5ms (cached)
- Incremental scans: ~115ms
- New service detection: <5 seconds
- Memory usage: ~10-20MB

### Platforms
- Linux x86_64 (musl static)
- Linux ARM64 (musl static)
- macOS Intel (x86_64)
- macOS Apple Silicon (ARM64)
- Windows x86_64

[0.1.2]: https://github.com/nvquanghuy/localwebs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/nvquanghuy/localwebs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/nvquanghuy/localwebs/releases/tag/v0.1.0
