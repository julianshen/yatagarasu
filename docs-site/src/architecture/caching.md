# Caching Architecture

## Layered Design

Yatagarasu treats caching as a fallback chain.

### Layer 1: Memory (Moka)
- **Tech**: `moka` Rust crate.
- **Characteristics**: Extremely fast, low latency, bounded by RAM.
- **Use Case**: Hot objects (index.html, common images).

### Layer 2: Disk (Local)
- **Tech**: Custom implementation.
- **Characteristics**: High capacity, durable across restarts, bounded by disk I/O.
- **Use Case**: Large objects that don't fit in RAM.

### Layer 3: Redis (Distributed)
- **Tech**: Redis or DragonflyDB.
- **Characteristics**: Shared state across multiple Yatagarasu replicas.
- **Use Case**: Reducing origin fetch rate in HA deployments.

## Cache Warming

The **Prewarm Manager** (v1.3.0) is a background system that actively populates the cache.
It iterates over S3 bucket prefixes and downloads objects into the configured cache layers, ensuring high availability before user traffic hits.
