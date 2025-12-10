# Production Checklist

Before going live, ensure:

1. [ ] **Resource Limits**: CPU/Memory requests/limits are set (see `values.yaml`).
2. [ ] **Replicas**: Running at least 2-3 replicas for HA.
3. [ ] **Monitoring**: Prometheus scraping is enabled (`/metrics`).
4. [ ] **Logging**: Log level set to `INFO` (not `DEBUG`).
5. [ ] **Security**: TLS enabled at Ingress or LoadBalancer level.
6. [ ] **Cache**: Redis is configured for shared state.
7. [ ] **Warming**: Critical paths are warmed up using the Prewarm API.
