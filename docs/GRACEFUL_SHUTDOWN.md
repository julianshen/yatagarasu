# Graceful Shutdown in Yatagarasu

## Overview

Yatagarasu leverages **Pingora's built-in graceful shutdown** capabilities. The `Server::run_forever()` method (see [src/main.rs:83](../src/main.rs#L83)) provides production-grade lifecycle management without requiring custom implementation.

## How It Works

### Signal Handling

Pingora automatically handles these signals:
- **SIGTERM** - Graceful shutdown (default for `kill`, Docker, Kubernetes)
- **SIGINT** - Graceful shutdown (Ctrl+C)
- **SIGQUIT** - Immediate shutdown (use sparingly)

### Shutdown Sequence

When Pingora receives a shutdown signal:

1. **Stop Accepting New Connections**
   - Listening sockets closed immediately
   - New connection attempts receive connection refused

2. **Wait for In-Flight Requests**
   - All active HTTP requests allowed to complete
   - Timeout controlled by `graceful_shutdown_timeout_s` (default: 30s)
   - If timeout reached, remaining connections force-closed

3. **Worker Shutdown**
   - Workers receive shutdown notification
   - Workers complete their cleanup
   - Process exits with status code 0

4. **Resource Cleanup**
   - Connection pools drained
   - S3 client connections closed gracefully
   - File descriptors released

## Configuration

Pingora's shutdown timeout can be configured in Pingora's `ServerConf`:

```rust
use pingora_core::server::configuration::ServerConf;

let mut server_conf = ServerConf::default();
server_conf.graceful_shutdown_timeout_s = 60; // 60 seconds
```

**Default**: 30 seconds

## Usage in Production

### Docker

Docker sends SIGTERM by default with `docker stop`:

```bash
# Docker waits 10 seconds then sends SIGKILL
docker stop yatagarasu

# Custom grace period (60 seconds)
docker stop --time=60 yatagarasu
```

**Dockerfile Example**:
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/yatagarasu /usr/local/bin/
# Pingora handles SIGTERM automatically
CMD ["/usr/local/bin/yatagarasu", "--config", "/etc/yatagarasu/config.yaml"]
```

### Kubernetes

Kubernetes sends SIGTERM with configurable grace period:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: yatagarasu
spec:
  containers:
  - name: yatagarasu
    image: yatagarasu:latest
    ports:
    - containerPort: 8080
  # Default: 30 seconds (matches Pingora default)
  terminationGracePeriodSeconds: 30
```

**Recommended Setup**:
- Set `terminationGracePeriodSeconds` ≥ Pingora's `graceful_shutdown_timeout_s`
- Add preStop hook if custom cleanup needed:
  ```yaml
  lifecycle:
    preStop:
      exec:
        command: ["/bin/sh", "-c", "sleep 5"]  # Brief delay for load balancer deregistration
  ```

### systemd

systemd sends SIGTERM followed by SIGKILL:

```ini
[Unit]
Description=Yatagarasu S3 Proxy
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/yatagarasu --config /etc/yatagarasu/config.yaml
Restart=on-failure

# Shutdown configuration
TimeoutStopSec=60       # Wait 60s before SIGKILL
KillMode=mixed          # SIGTERM to main process, SIGKILL to remaining after timeout
KillSignal=SIGTERM      # Use SIGTERM for graceful shutdown

[Install]
WantedBy=multi-user.target
```

## Verification

### Manual Testing

```bash
# Start proxy
./yatagarasu --config config.yaml &
PID=$!

# Send test request (simulate in-flight request)
curl http://localhost:8080/products/test.jpg &

# Send SIGTERM
kill -TERM $PID

# Proxy will:
# 1. Stop accepting new connections
# 2. Wait for curl request to complete
# 3. Shutdown gracefully
# 4. Exit with code 0
```

### Observability

Check Pingora's logs for shutdown events:
```
INFO  Received signal: SIGTERM
INFO  Graceful shutdown initiated
INFO  Waiting for 3 in-flight requests to complete
INFO  All requests completed
INFO  Shutdown complete
```

## Limitations & Future Enhancements

### Current Limitations
- Shutdown logging is minimal (Pingora's default)
- No custom cleanup hooks (not needed currently)
- /ready endpoint doesn't reflect shutdown state

### Planned Enhancements (v1.1+)
- [ ] Enhanced shutdown logging (reason, duration, in-flight count)
- [ ] Shutdown state in /ready endpoint ("shutting-down" status)
- [ ] Custom cleanup hooks for future features (cache flush, etc.)
- [ ] Graceful reload support (SIGHUP with zero-downtime)

## References

- [Pingora Documentation](https://github.com/cloudflare/pingora)
- [Kubernetes Pod Lifecycle](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/)
- [Docker Stop Reference](https://docs.docker.com/engine/reference/commandline/stop/)
- [systemd KillMode](https://www.freedesktop.org/software/systemd/man/systemd.kill.html)

## See Also

- [RETRY_INTEGRATION.md](RETRY_INTEGRATION.md) - Another example of Pingora built-in functionality
- [Phase 22: Graceful Shutdown](../plan.md) - Implementation plan
