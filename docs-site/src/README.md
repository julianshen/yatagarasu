# Yatagarasu S3 Proxy

Yatagarasu is a high-performance, caching S3 reverse proxy built in Rust using the Pingora framework. It is designed to sit between your clients and S3-compatible storage (like AWS S3, MinIO, Ceph, etc.) to improve performance, reduce costs, and enforce security policies.

## Key Features

- **High Performance**: Built on Cloudflare's Pingora framework for asynchronous, non-blocking I/O.
- **Smart Caching**:
    - **Multi-Layer**: Memory (Moka), Disk, and Redis/DragonflyDB layers.
    - **Cache Warming**: Pre-fetch objects into cache to ensure high hit rates from the start.
    - **Configurable TTL**: Define Time-To-Live per bucket.
- **Security & Authorization**:
    - **OPA Integration**: Fine-grained access control using Open Policy Agent.
    - **OpenFGA Integration**: Relationship-based access control (ReBAC).
    - **JWT Authentication**: Validate JSON Web Tokens at the edge.
- **Deployment Ready**:
    - Helm charts and Kustomize overlays included.
    - Prometheus metrics export.
    - Structured logging.

## Why "Yatagarasu"?

Named after the three-legged crow from Japanese mythology, a divine messenger and guide. Just as the Yatagarasu guides lost souls, this proxy guides your S3 requests swiftly and securely to their destination.
