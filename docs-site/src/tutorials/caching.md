# Caching & Warming

## Caching Strategy

Yatagarasu supports a multi-layer caching strategy:
1. **Memory**: Ultra-fast, local RAM cache (Moka).
2. **Disk**: Local SSD/HDD storage for larger datasets.
3. **Redis**: Distributed cache for sharing data across replicas.

## Configuration

```yaml
cache:
  enabled: true
  # Define order of checking
  cache_layers: ["memory", "redis"]
  
  memory:
    max_cache_size_mb: 512
    default_ttl_seconds: 3600
    
  redis:
    enabled: true
    redis_url: "redis://localhost:6379"
    redis_ttl_seconds: 86400
```

## Cache Warming (New in v1.3.0)

Cache warming allows you to pre-populate the cache before users request the data. This is useful for:
- New deployments (checking cache before traffic shifts).
- Popular content (daily reports, viral media).

### Configuration

Add the `warming` section to your `cache` config:

```yaml
cache:
  warming:
    concurrency: 20
    rate_limit: 500 # requests per second
```

### Triggering a Warm-up

Use the Admin API to start a job:

```bash
curl -X POST http://localhost:8080/admin/cache/prewarm \
  -H "Authorization: Bearer <ADMIN_TOKEN>" \
  -d '{
    "bucket": "my-bucket",
    "path": "daily-reports/",
    "recursive": true
  }'
```

The system will verify the job, spawn a worker interaction, and download objects matching the prefix into the configured cache layers.
