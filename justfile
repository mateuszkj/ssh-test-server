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
