# Bench CLI

A command-line interface tool for Bench Computing.

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/bench-chat/bench-cli/releases):

- Linux (x64): `bench-linux-amd64.tar.gz`
- Linux (ARM64): `bench-linux-arm64.tar.gz`
- macOS (Universal): `bench-macos-universal.tar.gz`
- Windows: `bench-windows-amd64.exe.zip`

### Building from Source

1. Install Rust using [rustup](https://rustup.rs/)

2. Clone the repository:
```bash
git clone https://github.com/chatbench/bench
cd bench
```

3. Build the project:
```bash
cargo build --release
```

The binary will be available at `target/release/bench`

## Docker Usage

1. Create the persistent directory:
```bash
mkdir -p ~/.bench/root
```

2. Run using Docker Compose:
```bash
docker-compose run --rm bench
```

Or directly with Docker:
```bash
docker run -it --rm -v ~/.bench/root:/home/bench chatbench/cli:latest
```

The entire home directory of the bench user inside the container will be persisted to `~/.bench/root` on your host system, including:
- Configuration files
- Cache
- History
- Any other user-specific data

## Dependencies

- Rust 1.x
- For Linux ARM64 builds: `gcc-aarch64-linux-gnu`, `g++-aarch64-linux-gnu`

## Development

### Building for Different Platforms

The project supports building for multiple platforms:
- Linux (x86_64, aarch64)
- macOS (Universal binary supporting both Intel and Apple Silicon)
- Windows (x86_64)

### Running Tests

```bash
cargo test
```

## License

[License information not provided in excerpts]

## Authors

Bench Computing <admin@bench.chat>
