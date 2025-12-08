# Yatagarasu - Product Roadmap

**Current Version**: v1.2.0
**Project**: High-Performance S3 Proxy built with Rust and Pingora

---

## Released Versions

### v1.2.0 - Production Hardening ✅ **RELEASED** (December 2025)

- SIGHUP hot reload with ArcSwap (zero dropped requests)
- OpenFGA fine-grained authorization (ReBAC)
- Multi-architecture Docker images on GHCR
- CI pipeline stabilization
- Project structure reorganization

### v1.1.0 - Enhanced Features ✅ **RELEASED** (November 2025)

- Multi-tier caching (Memory/Disk/Redis) with 80%+ hit rates
- Advanced JWT (RS256/ES256, JWKS endpoints)
- OPA policy-based authorization
- Comprehensive audit logging
- OpenTelemetry distributed tracing
- IP allowlist/blocklist, per-user rate limiting

### v1.0.0 - Production Release ✅ **RELEASED** (November 2025)

- Core S3 proxy on Pingora framework
- Multi-bucket routing with credential isolation
- JWT authentication (HS256)
- HTTP Range requests, zero-copy streaming
- Health endpoints, Prometheus metrics
- HA bucket replication with failover
- Rate limiting, circuit breaker

---

## Planned Features

### v1.3.0 - Advanced Operations (Q1 2026)

**Focus**: Operational improvements and debugging

- [ ] Admin dashboard web UI
- [ ] Cache warming/preloading API
- [ ] Enhanced metrics (SLO tracking, percentile histograms)
- [ ] Request replay for debugging
- [ ] Configuration validation endpoint

### v2.0.0 - Extended Capabilities (Future)

**Focus**: New proxy capabilities

Ideas under consideration:
- WebSocket support for real-time S3 events
- GraphQL API for object queries
- Image/video transformation on-the-fly
- Optional write support (PUT/POST behind feature flag)
- Multi-region latency-based routing

---

## Development Principles

- **TDD**: Red → Green → Refactor
- **Quality**: All tests pass, no clippy warnings, >90% coverage
- **Architecture**: Separation of concerns, explicit dependencies, fail fast

---

## Contributing

1. Deploy and test in your environment
2. Report issues via GitHub Issues
3. Request features for future versions
4. Contribute via Pull Requests

**Docker**: `ghcr.io/julianshen/yatagarasu:1.2.0`

---

**Last Updated**: December 2025
**License**: MIT
