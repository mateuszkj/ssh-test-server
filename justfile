default:
	@just --list --justfile {{justfile()}}

# Rust cargo check
check:
	cargo check --tests

# Run fmt and clippy
lint: check
	cargo fmt
	cargo clippy --tests -- -D warnings

# Run tests
test:
	cargo test

# Run ssh server
server:
	RUST_LOG=trace cargo run -p ssh-test-server-cli
