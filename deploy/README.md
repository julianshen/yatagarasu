# Yatagarasu Observability Stack

This directory contains Docker Compose configuration and provisioning files for the Yatagarasu observability stack.

## Components

- **Redis** (port 6379): Cache backend for testing
- **Prometheus** (port 9090): Metrics collection and storage
- **Grafana** (port 3000): Metrics visualization and dashboards

## Quick Start

### Start the observability stack

```bash
# From the project root
docker-compose -f docker/docker-compose.observability.yml up -d
```

### Access the services

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (login: admin/admin)
- **Redis**: localhost:6379

### Run Yatagarasu with metrics

```bash
# Build and run the proxy (metrics exposed on :8080/metrics)
cargo run -- --config config.yaml
```

### View metrics in Grafana

1. Open http://localhost:3000
2. Login with admin/admin
3. Navigate to Dashboards
4. Select "Yatagarasu Cache Metrics" (auto-provisioned)

### Stop the stack

```bash
docker-compose -f docker/docker-compose.observability.yml down
```

### Clean up volumes (removes all data)

```bash
docker-compose -f docker/docker-compose.observability.yml down -v
```

## Metrics Exposed

The Yatagarasu proxy exposes the following metrics at `/metrics`:

### Cache Metrics

- `yatagarasu_cache_hits_total` - Total number of cache hits
- `yatagarasu_cache_misses_total` - Total number of cache misses
- `yatagarasu_cache_sets_total` - Total number of cache sets
- `yatagarasu_cache_evictions_total` - Total number of cache evictions
- `yatagarasu_cache_errors_total` - Total number of cache errors

### Operation Latency

- `yatagarasu_cache_operation_duration_seconds` - Histogram of operation latencies
  - Labels: `operation` (get/set/delete/clear)

### Connection Pool

- `yatagarasu_redis_pool_connections` - Current number of active connections
- `yatagarasu_redis_pool_idle_connections` - Current number of idle connections

## Configuration

### Prometheus

Edit `deploy/prometheus.yml` to:
- Change scrape intervals
- Add alerting rules
- Configure additional targets

### Grafana

- **Datasources**: `deploy/grafana/provisioning/datasources/`
- **Dashboards**: `deploy/grafana/provisioning/dashboards/`

Dashboards are automatically loaded from the dashboards directory.

## Development

### Testing with observability stack

```bash
# Start the stack
docker-compose -f docker/docker-compose.observability.yml up -d

# Run integration tests with real Redis
TEST_S3_ENDPOINT=http://localhost:9000 \
REDIS_URL=redis://localhost:6379 \
cargo test --test redis_cache_integration_test

# View metrics in Prometheus
open http://localhost:9090/graph

# View dashboards in Grafana
open http://localhost:3000
```

### Adding custom dashboards

1. Create a new `.json` file in `deploy/grafana/provisioning/dashboards/`
2. Restart Grafana: `docker-compose -f docker/docker-compose.observability.yml restart grafana`
3. Dashboard will be automatically loaded

## Production Deployment

For production, consider:

1. **Security**
   - Change Grafana admin password
   - Enable authentication for Prometheus
   - Use TLS for all endpoints

2. **High Availability**
   - Deploy Prometheus with remote storage (e.g., Thanos, Cortex)
   - Use Grafana with HA setup
   - Deploy Redis in cluster mode

3. **Alerting**
   - Configure Alertmanager
   - Add alerting rules in `prometheus.yml`
   - Set up notification channels (Slack, PagerDuty, etc.)

4. **Retention**
   - Configure Prometheus retention period
   - Set up long-term storage for historical data

## Troubleshooting

### Prometheus can't scrape Yatagarasu metrics

- Ensure Yatagarasu is running and exposing metrics on `:8080/metrics`
- Check Prometheus targets: http://localhost:9090/targets
- Verify `host.docker.internal` resolves (on Linux, use `172.17.0.1` instead)

### Grafana shows no data

- Verify Prometheus datasource is configured: http://localhost:3000/datasources
- Check Prometheus is scraping: http://localhost:9090/targets
- Ensure metrics are being generated (run some cache operations)

### Redis connection refused

- Check Redis is running: `docker ps | grep redis`
- Verify port mapping: `docker port yatagarasu-redis`
- Test connection: `redis-cli -h localhost -p 6379 ping`
