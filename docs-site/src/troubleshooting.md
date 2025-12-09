# Troubleshooting

## Common Issues

### 1. Connection Refused
- **Symptom**: `curl` fails to connect to port 8080.
- **Cause**: Server not running or firewall blocking.
- **Fix**: Check `docker compose ps` or `kubectl get pods`. Ensure port mapping is correct.

### 2. AWS S3 Access Denied
- **Symptom**: Proxy returns 403 Forbidden.
- **Cause**: Incorrect Credentials or Policies.
- **Fix**: Check `access_key` and `secret_key` in `config.yaml`. Verify IAM user has `s3:GetObject` permission.

### 3. Cache Misses High
- **Symptom**: `x-cache-status` is mostly `MISS`.
- **Cause**: TTL too short or keys not matching.
- **Fix**: Increase `default_ttl_seconds`. Ensure client isn't sending `Cache-Control: no-cache`.

## Logs

Set `RUST_LOG=debug` to see detailed request processing logic.

```bash
RUST_LOG=debug ./yatagarasu
```
