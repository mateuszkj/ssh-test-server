default:
	@just --list --justfile {{justfile()}}

# Install development dependences.
install-dev:
    cargo install --locked cargo-llvm-cov cargo-mutants cargo-deny cargo-edit cargo-sort cargo-sort-derives typos-cli

# Run formatter.
fmt:
    cargo fmt
    cargo sort -w
    cargo sort-derives


# Run fmt and clippy
lint: fmt
    cargo check --tests
    typos
    cargo clippy -- -D warnings
    cargo clippy --tests -- -D warnings
    cargo deny check


# Run tests
test:
	cargo test

# Run ssh server
server:
	RUST_LOG=trace cargo run -p ssh-test-server-cli
