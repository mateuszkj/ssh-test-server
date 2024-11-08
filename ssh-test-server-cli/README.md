# In memory ssh server cli

`ssh-test-server-cli` is a in memory ssh server to that can be used in integration testing.

## Installation

```shell
cargo install ssh-test-server-cli
```

## Usage

```shell
RUST_LOG=trace cargo run -p ssh-test-server-cli
```

## Contributions

Contributions are welcome! Please open an issue or submit a pull request on Gitlab.

Before sending pull request please run lints and tests first:

```shell
cargo install just
just install-dev
just lint
just test
```

## License

This project is licensed under the MIT OR Apache-2.0 License.