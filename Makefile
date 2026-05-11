.PHONY: build release install clean help

help:
	@echo "LocalWebs - Build & Release"
	@echo ""
	@echo "Usage:"
	@echo "  make build         Build debug binary"
	@echo "  make release       Build optimized release binary"
	@echo "  make install       Install to ~/.local/bin"
	@echo "  make cross         Build for all platforms (requires cross)"
	@echo "  make clean         Clean build artifacts"
	@echo ""

build:
	cargo build

release:
	cargo build --release --locked
	@echo ""
	@echo "✅ Release binary: target/release/localwebs"

install: release
	@mkdir -p ~/.local/bin
	@cp target/release/localwebs ~/.local/bin/
	@chmod +x ~/.local/bin/localwebs
	@echo ""
	@echo "✅ Installed to ~/.local/bin/localwebs"
	@echo "🚀 Run: localwebs --port 9999"

cross:
	@echo "Building for Linux x86_64..."
	cross build --release --target x86_64-unknown-linux-musl
	@echo "Building for Linux ARM64..."
	cross build --release --target aarch64-unknown-linux-musl
	@echo "Building for macOS x86_64..."
	cross build --release --target x86_64-apple-darwin
	@echo "Building for macOS ARM64..."
	cross build --release --target aarch64-apple-darwin
	@echo ""
	@echo "✅ Cross-compilation complete"
	@echo "📦 Binaries in target/<arch>/release/"

package: cross
	@mkdir -p dist
	@echo "Packaging releases..."
	@cd target/x86_64-unknown-linux-musl/release && tar czf ../../../dist/localwebs-x86_64-unknown-linux-musl.tar.gz localwebs
	@cd target/aarch64-unknown-linux-musl/release && tar czf ../../../dist/localwebs-aarch64-unknown-linux-musl.tar.gz localwebs
	@cd target/x86_64-apple-darwin/release && tar czf ../../../dist/localwebs-x86_64-apple-darwin.tar.gz localwebs
	@cd target/aarch64-apple-darwin/release && tar czf ../../../dist/localwebs-aarch64-apple-darwin.tar.gz localwebs
	@echo ""
	@echo "✅ Packages created in dist/"
	@ls -lh dist/

clean:
	cargo clean
	rm -rf dist

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings
