## Docker Usage

1. Create the persistent directory:

```bash
mkdir -p ~/.bench/root
```

2. Run the CLI using Docker Compose:

```bash
docker-compose run --rm bench
```

Or directly with Docker:

```bash
docker run -it --rm -v ~/.bench/root:/home/bench chatbench/cli:latest
```

The entire home directory of the bench user inside the container will be persisted to `~/.bench/root` on your host system. This includes:

- Configuration files
- Cache
- History
- Any other user-specific data
