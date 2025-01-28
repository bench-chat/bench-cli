FROM ubuntu:22.04

# Install minimal dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create bench user
RUN useradd -m bench

# Copy bench binary
COPY --chmod=755 target/x86_64-unknown-linux-gnu/release/bench /usr/local/bin/bench

# Switch to bench user
USER bench
WORKDIR /home/bench

# Set the entrypoint to bench CLI
ENTRYPOINT ["bench"]
