---
title: Server Configuration
layout: default
parent: Configuration
nav_order: 1
---

# Server Configuration

Configure the HTTP server settings.
{: .fs-6 .fw-300 }

---

## Basic Configuration

```yaml
server:
  address: "0.0.0.0:8080"
  threads: 4
```

---

## Options

### address

The address and port to listen on.

| | |
|:--|:--|
| **Type** | `string` |
| **Default** | `"0.0.0.0:8080"` |
| **Required** | No |
| **Hot Reload** | No (requires restart) |

**Examples:**

```yaml
# Listen on all interfaces, port 8080
address: "0.0.0.0:8080"

# Listen only on localhost
address: "127.0.0.1:8080"

# Custom port
address: "0.0.0.0:3000"

# IPv6
address: "[::]:8080"
```

---

### threads

Number of worker threads for handling requests.

| | |
|:--|:--|
| **Type** | `integer` |
| **Default** | Number of CPU cores |
| **Required** | No |
| **Hot Reload** | No (requires restart) |

**Examples:**

```yaml
# Fixed number of threads
threads: 4

# Match CPU cores (default behavior if omitted)
# threads: auto  # implied default
```

**Guidelines:**

| CPU Cores | Recommended Threads |
|:----------|:--------------------|
| 1-2 | 2 |
| 4 | 4 |
| 8 | 8 |
| 16+ | 8-16 |

For I/O-bound workloads (typical for a proxy), more threads than CPU cores can help.

---

## Full Example

```yaml
server:
  # Listen on all interfaces
  address: "0.0.0.0:8080"

  # Use 8 worker threads
  threads: 8
```

---

## Environment Variables

Server settings can use environment variables:

```yaml
server:
  address: "${LISTEN_ADDRESS:-0.0.0.0:8080}"
  threads: "${WORKER_THREADS:-4}"
```

---

## Docker Considerations

When running in Docker:

```yaml
server:
  # Always use 0.0.0.0 in containers
  address: "0.0.0.0:8080"
```

Then map ports in Docker:

```bash
docker run -p 8080:8080 yatagarasu
```

---

## Kubernetes Considerations

For Kubernetes deployments:

```yaml
server:
  address: "0.0.0.0:8080"
  # Let K8s handle scaling, use moderate thread count
  threads: 4
```

Use Horizontal Pod Autoscaler for scaling instead of many threads per pod.

---

## Performance Notes

- Each thread handles multiple connections via async I/O
- Memory usage scales with `threads * connections_per_thread * ~64KB`
- More threads help with CPU-bound operations (JWT validation, S3 signing)
- For pure proxy workloads, 4-8 threads is usually sufficient

---

## See Also

- [Metrics Configuration](/yatagarasu/configuration/metrics/) - Separate port for metrics
- [Performance Guide](/yatagarasu/operations/performance/) - Tuning recommendations
