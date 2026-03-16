BINARY      := ai-ranger
TARGETS     := x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
               x86_64-apple-darwin aarch64-apple-darwin \
               x86_64-pc-windows-msvc

.PHONY: build test lint clean release run

# Windows: run as Administrator (uses SIO_RCVALL via ws2_32.dll - no npcap needed).
# Linux/macOS: run with sudo (uses AF_PACKET / BPF - no libpcap needed).

build:            ## Build debug binary for current platform
	cargo build

run:              ## Run agent (requires administrator/root for packet capture)
	cargo run

test:             ## Run all tests
	cargo test

lint:             ## Run clippy and rustfmt check
	cargo clippy -- -D warnings
	cargo fmt --check

release:          ## Build release binaries for all targets
	@for target in $(TARGETS); do \
		echo "Building $$target..."; \
		cargo build --release --target $$target; \
	done

clean:            ## Remove build artifacts
	cargo clean
