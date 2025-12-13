---
title: Troubleshooting
layout: default
parent: Operations
nav_order: 2
---

# Troubleshooting

Common issues and their solutions.
{: .fs-6 .fw-300 }

---

## Startup Issues

### "Configuration file not found"

```
Error: Configuration file not found at /etc/yatagarasu/config.yaml
```

**Solution:**
1. Verify file exists: `ls -la /etc/yatagarasu/config.yaml`
2. Check mount in Docker: `docker inspect yatagarasu | grep Mounts`
3. Specify path explicitly: `yatagarasu --config /path/to/config.yaml`

### "Invalid YAML syntax"

```
Error: Failed to parse configuration: expected ':', found newline at line 15
```

**Solution:**
1. Validate YAML: `yq eval config.yaml`
2. Check indentation (use spaces, not tabs)
3. Ensure all strings with special characters are quoted

### "Missing required field"

```
Error: Missing required field 'bucket' in s3 configuration
```

**Solution:**
Check configuration has all required fields:
```yaml
buckets:
  - name: "required"
    path_prefix: "/required"
    s3:
      bucket: "required"      # This was missing
      region: "required"
      access_key: "required"
      secret_key: "required"
```

### "Environment variable not set"

```
Error: Environment variable 'AWS_SECRET_ACCESS_KEY' not set
```

**Solution:**
1. Set the variable: `export AWS_SECRET_ACCESS_KEY=xxx`
2. Pass to Docker: `docker run -e AWS_SECRET_ACCESS_KEY=xxx ...`
3. Add to Kubernetes secret

---

## Connection Issues

### "Connection refused" on port 8080

**Symptoms:**
```bash
curl http://localhost:8080/health
# curl: (7) Failed to connect to localhost port 8080: Connection refused
```

**Checklist:**
1. Check process is running: `ps aux | grep yatagarasu`
2. Check port binding: `netstat -tlnp | grep 8080`
3. In Docker, check container: `docker ps | grep yatagarasu`
4. Verify address in config: `server.address: "0.0.0.0:8080"`

### "S3 Access Denied"

```
Error: Access Denied when accessing bucket 'my-bucket'
```

**Checklist:**
1. Verify credentials are correct
2. Check bucket name and region match
3. Ensure IAM policy allows `s3:GetObject`
4. Verify bucket exists: `aws s3 ls s3://my-bucket`

### "Connection timeout to S3"

```
Error: Connection timeout after 5000ms to s3.us-east-1.amazonaws.com
```

**Checklist:**
1. Check network connectivity: `curl -I https://s3.us-east-1.amazonaws.com`
2. Verify firewall rules allow outbound HTTPS
3. Check VPC/security group settings
4. Increase timeout in config:
   ```yaml
   replicas:
     - name: "primary"
       timeout_seconds: 10
   ```

### "Cannot resolve hostname"

```
Error: DNS resolution failed for 's3.us-east-1.amazonaws.com'
```

**Checklist:**
1. Check DNS: `nslookup s3.us-east-1.amazonaws.com`
2. In Docker, check DNS config: `docker exec yatagarasu cat /etc/resolv.conf`
3. Use custom DNS: `docker run --dns 8.8.8.8 ...`

---

## Authentication Issues

### "Missing authentication token"

```json
{"error": "Missing authentication token", "status": 401}
```

**Checklist:**
1. Send token in configured source:
   ```bash
   curl -H "Authorization: Bearer <token>" ...
   ```
2. Check token source config matches request
3. Verify auth is enabled for this bucket

### "Invalid JWT signature"

```json
{"error": "Invalid JWT signature", "status": 401}
```

**Checklist:**
1. Verify secret matches between issuer and proxy
2. Check algorithm matches (HS256 vs RS256)
3. Decode token at jwt.io to inspect
4. For RS256, ensure public key is correct

### "JWT token expired"

```json
{"error": "Token expired", "status": 401}
```

**Solution:**
1. Generate new token with future `exp` claim
2. Check server time is synchronized
3. Increase token expiry time

### "Claims verification failed"

```json
{"error": "Claim 'role' does not match required value", "status": 403}
```

**Checklist:**
1. Decode token and check claims
2. Verify claims match config requirements
3. Check operator (equals vs in vs contains)

---

## Performance Issues

### High Latency

**Symptoms:**
- P95 latency > 500ms
- Slow response times

**Checklist:**
1. Check cache hit rate in metrics
2. Verify S3 backend latency
3. Check for resource exhaustion:
   ```bash
   docker stats yatagarasu
   ```
4. Review slow requests in logs

**Solutions:**
- Increase cache size
- Enable Redis for distributed caching
- Add more replicas
- Tune S3 timeout settings

### Low Cache Hit Rate

**Symptoms:**
- Cache hit rate < 50%
- High S3 request volume

**Checklist:**
1. Verify cache is enabled and sized appropriately
2. Check TTL settings
3. Review access patterns (many unique files?)

**Solutions:**
```yaml
cache:
  memory:
    max_capacity: 1073741824  # Increase to 1GB
    ttl_seconds: 7200         # Increase TTL
```

### High Memory Usage

**Symptoms:**
- Memory growing over time
- OOM kills

**Checklist:**
1. Check cache configuration
2. Monitor connection count
3. Look for memory leaks in metrics

**Solutions:**
- Reduce cache size
- Set memory limits
- Restart periodically as workaround

---

## S3 Backend Issues

### "All replicas failed"

```
Error: All S3 replicas failed for bucket 'my-bucket'
```

**Checklist:**
1. Check replica health in metrics
2. Verify all replicas are reachable
3. Check circuit breaker state
4. Review recent errors in logs

**Solutions:**
- Fix underlying S3 connectivity
- Wait for circuit breaker recovery
- Manually test S3 access:
  ```bash
  aws s3 ls s3://my-bucket
  ```

### Circuit Breaker Stuck Open

**Symptoms:**
- Replica shows unhealthy
- No requests to that replica

**Checklist:**
1. Check circuit breaker state in metrics
2. Wait for timeout period
3. Verify underlying issue is resolved

**Manual Recovery:**
- Restart Yatagarasu to reset circuit breaker

---

## Cache Issues

### Redis Connection Failed

```
Error: Failed to connect to Redis at redis://localhost:6379
```

**Checklist:**
1. Verify Redis is running: `redis-cli ping`
2. Check network connectivity
3. Verify URL format
4. Check authentication if required

### Disk Cache Permission Denied

```
Error: Permission denied writing to /var/cache/yatagarasu
```

**Solution:**
```bash
# Fix permissions
mkdir -p /var/cache/yatagarasu
chown 65532:65532 /var/cache/yatagarasu

# Or in Docker
docker run -v cache:/var/cache/yatagarasu ...
```

---

## Docker Issues

### Container Exits Immediately

```bash
docker logs yatagarasu
# Check for error message
```

**Common causes:**
- Invalid configuration
- Missing environment variables
- Port already in use

### "Permission denied" in Container

**Solution:**
- Yatagarasu runs as non-root (UID 65532)
- Ensure mounted volumes are accessible
- Use Docker volumes instead of bind mounts

### Cannot Access Localhost Services

**Problem:** Container can't reach services on host

**Solutions:**
```bash
# Use host.docker.internal
docker run --add-host=host.docker.internal:host-gateway ...

# Or use host network
docker run --network host ...
```

---

## Kubernetes Issues

### Pod CrashLoopBackOff

```bash
kubectl describe pod yatagarasu-xxx
kubectl logs yatagarasu-xxx --previous
```

**Common causes:**
- ConfigMap not mounted
- Secret missing
- Invalid configuration

### Readiness Probe Failing

```bash
kubectl describe pod yatagarasu-xxx | grep -A5 Readiness
```

**Checklist:**
1. Check S3 backend connectivity
2. Verify probe configuration
3. Check pod logs for errors

### Service Not Accessible

```bash
kubectl get svc yatagarasu
kubectl get endpoints yatagarasu
```

**Checklist:**
1. Verify service selector matches pod labels
2. Check pods are ready
3. Test from within cluster:
   ```bash
   kubectl run test --rm -it --image=curlimages/curl -- curl http://yatagarasu:8080/health
   ```

---

## Debug Commands

### Check Configuration

```bash
# Validate configuration
yatagarasu --config config.yaml --validate

# In Docker
docker run --rm \
  -v ./config.yaml:/etc/yatagarasu/config.yaml \
  ghcr.io/julianshen/yatagarasu:latest \
  --config /etc/yatagarasu/config.yaml --validate
```

### Check Connectivity

```bash
# Test S3
curl -I https://s3.us-east-1.amazonaws.com

# Test Redis
redis-cli -h redis-host ping

# Test OPA
curl http://opa:8181/health
```

### Inspect Logs

```bash
# All logs
docker logs yatagarasu

# Filter by level
docker logs yatagarasu 2>&1 | jq 'select(.level == "error")'

# Follow logs
docker logs -f yatagarasu

# Last 100 lines
docker logs --tail 100 yatagarasu
```

### Check Metrics

```bash
# All metrics
curl http://localhost:9090/metrics

# Specific metric
curl -s http://localhost:9090/metrics | grep yatagarasu_cache

# Cache hit rate
curl -s http://localhost:9090/metrics | grep cache_hits
```

---

## Getting Help

If you can't resolve an issue:

1. **Search existing issues**: [GitHub Issues](https://github.com/julianshen/yatagarasu/issues)
2. **Create new issue** with:
   - Configuration (sanitized)
   - Error messages
   - Steps to reproduce
   - Environment details

---

## See Also

- [Operations Guide](/yatagarasu/operations/)
- [Configuration Reference](/yatagarasu/configuration/)
- [Monitoring Guide](/yatagarasu/operations/monitoring/)
