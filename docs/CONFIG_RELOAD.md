# Configuration Hot Reload

## Overview

Yatagarasu supports **hot reload** for most configuration changes without requiring a server restart. This enables zero-downtime configuration updates for bucket routing, credentials, and authentication.

## How to Reload Configuration

### Method 1: SIGHUP Signal (POSIX)
```bash
# Edit your config.yaml
vim config.yaml

# Send SIGHUP to running process
kill -HUP $(pgrep yatagarasu)

# Or use systemd
systemctl reload yatagarasu
```

### Method 2: Admin API Endpoint
```bash
# Requires admin JWT token
TOKEN="your-admin-jwt-token"

curl -X POST http://localhost:8080/admin/reload \
  -H "Authorization: Bearer $TOKEN"
```

**Response on success (200 OK):**
```json
{
  "status": "success",
  "message": "Configuration reloaded successfully",
  "config_generation": 42,
  "timestamp": 1699564800
}
```

**Response on failure (400 Bad Request):**
```json
{
  "status": "error",
  "message": "Configuration reload failed",
  "error": "Duplicate path_prefix '/api' found in buckets: bucket1, bucket2"
}
```

**Response without authentication (401 Unauthorized):**
```json
{
  "status": "error",
  "message": "Authentication required: Missing token"
}
```

---

## Configuration Changes: Hot Reload vs Restart

### ✅ **Hot Reload Supported** (Zero Downtime)

These configuration changes take effect immediately without restarting the server:

#### Bucket Configuration
- ✅ **Add new bucket** - New bucket immediately available for routing
- ✅ **Remove bucket** - Existing requests complete, new requests get 404
- ✅ **Update S3 credentials** (access_key, secret_key) - New requests use updated credentials
- ✅ **Change bucket path_prefix** - New routing immediately active
- ✅ **Change S3 endpoint** - New requests use new endpoint
- ✅ **Change S3 region** - New requests use new region
- ✅ **Enable/disable bucket-level authentication** - Takes effect immediately

#### JWT Authentication Configuration
- ✅ **Rotate JWT secret** - New requests validated with new secret
- ✅ **Change JWT algorithm** (HS256, HS384, HS512, RS256, etc.)
- ✅ **Add/remove custom claims validation** - New validation rules apply immediately
- ✅ **Change token sources** (header, query param, custom header)
- ✅ **Enable/disable JWT globally** - Authentication enforcement changes immediately

#### Example: Add New Bucket
```yaml
# config.yaml - Add new bucket
buckets:
  - name: existing_bucket
    path_prefix: "/existing"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"

  # Add new bucket - will be available immediately after reload
  - name: new_bucket
    path_prefix: "/new"
    s3:
      bucket: "new-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
```

```bash
# Reload without restart
kill -HUP $(pgrep yatagarasu)

# New bucket immediately accessible
curl http://localhost:8080/new/sample.txt
```

---

### ❌ **Restart Required** (Server Rebind Needed)

These configuration changes require stopping and restarting the server process:

#### Server Network Configuration
- ❌ **Server address** (`server.address`) - Requires rebinding socket
- ❌ **Server port** (`server.port`) - Requires rebinding socket
- ❌ **TLS configuration** (if implemented) - Requires new TLS context

#### Server Resource Configuration
- ❌ **Worker threads** (`server.threads`) - Thread pool size fixed at startup
- ❌ **Max connections** (if configured) - Resource limits set at startup

#### Why Restart is Required

**Socket Binding**: Changing the server address or port requires closing the existing socket and binding to a new address. This cannot be done atomically without dropping connections.

**Thread Pool**: Pingora's thread pool is initialized at server startup and cannot be dynamically resized.

**Example: Change Server Port (Requires Restart)**
```yaml
server:
  address: "0.0.0.0"
  port: 9090  # Changed from 8080 - REQUIRES RESTART
  threads: 4
```

```bash
# Stop server
systemctl stop yatagarasu

# Edit config.yaml (change port)

# Start server with new config
systemctl start yatagarasu
```

---

## Reload Behavior

### In-Flight Requests
- **In-flight requests continue** with the configuration that was active when they started
- No requests are dropped during reload
- Ensures zero downtime for ongoing operations

### New Requests
- **New requests use the new configuration** immediately after successful reload
- Routing, authentication, and S3 credentials all updated atomically

### Configuration Validation
- **Invalid configuration is rejected** without affecting the running service
- Validation errors returned via API or logged for SIGHUP
- Service continues with previous valid configuration on validation failure

### Example: Safe Credential Rotation
```bash
# 1. Update credentials in config.yaml
vim config.yaml

# 2. Reload - validates config before applying
curl -X POST http://localhost:8080/admin/reload \
  -H "Authorization: Bearer $TOKEN"

# If validation fails (400 response):
# - Service continues with old credentials
# - Fix config.yaml and retry

# If validation succeeds (200 response):
# - New requests use new credentials immediately
# - In-flight requests complete with old credentials
```

---

## Monitoring Reload Operations

### Prometheus Metrics

Yatagarasu exposes reload metrics via `/metrics` endpoint:

```prometheus
# Successful configuration reloads
config_reload_success_total 5

# Failed configuration reload attempts
config_reload_failure_total 1

# Current configuration generation number
config_generation 42
```

### Use Cases

**Alerting on Reload Failures:**
```yaml
# Prometheus alert rule
- alert: ConfigReloadFailure
  expr: increase(config_reload_failure_total[5m]) > 0
  annotations:
    summary: "Configuration reload failed"
    description: "Yatagarasu failed to reload configuration. Check logs for validation errors."
```

**Tracking Configuration Changes:**
```yaml
# Grafana query - correlate performance changes with config updates
config_generation
```

**Audit Trail:**
```bash
# Query metrics to see reload history
curl http://localhost:8080/metrics | grep config_reload
```

---

## Best Practices

### 1. Validate Before Reload
Always test configuration changes in a staging environment before applying to production.

```bash
# Dry-run validation (future feature)
yatagarasu --config config.yaml --validate
```

### 2. Use Version Control
Keep configuration in version control (Git) to track changes and enable rollback:

```bash
git add config.yaml
git commit -m "Rotate S3 credentials for products bucket"
git push
```

### 3. Automate Reload with Configuration Management

**Ansible Example:**
```yaml
- name: Update Yatagarasu configuration
  copy:
    src: config.yaml
    dest: /etc/yatagarasu/config.yaml
  notify: reload yatagarasu

handlers:
  - name: reload yatagarasu
    shell: kill -HUP $(pgrep yatagarasu)
```

**systemd Example:**
```bash
# systemd will send SIGHUP on reload
systemctl reload yatagarasu
```

### 4. Monitor Reload Metrics
Set up alerts for reload failures to detect configuration errors immediately:

```yaml
# Alert on any reload failure
alert: ConfigReloadFailed
expr: config_reload_failure_total > 0
for: 1m
```

### 5. Document Changes
Use structured commit messages when updating configuration:

```bash
git commit -m "Add new 'analytics' bucket for data exports

- Bucket: analytics-data
- Region: us-west-2
- Path: /analytics/*
- Auth: Required (admin role)
"
```

---

## Troubleshooting

### Reload Fails with "Duplicate path_prefix"
**Error:**
```json
{
  "status": "error",
  "error": "Duplicate path_prefix '/api' found in buckets: bucket1, bucket2"
}
```

**Solution:** Ensure all bucket `path_prefix` values are unique.

### Reload Fails with "Invalid JWT secret"
**Error:**
```json
{
  "status": "error",
  "error": "JWT secret must be at least 32 characters"
}
```

**Solution:** Use a longer JWT secret (recommended: 64+ characters).

### Reload Succeeds but New Bucket Returns 404
**Cause:** Path prefix doesn't match request path.

**Solution:** Check bucket `path_prefix` matches the URL path:
```yaml
buckets:
  - name: mydata
    path_prefix: "/data"  # Matches: /data/file.txt
```

### SIGHUP Has No Effect
**Cause:** Process not receiving signal or signal handler not registered.

**Solution:**
1. Check process is running: `pgrep yatagarasu`
2. Check logs for signal handler registration
3. Use API endpoint instead: `POST /admin/reload`

---

## Security Considerations

### Admin Endpoint Authentication
The `/admin/reload` endpoint **requires JWT authentication** with admin privileges:

```yaml
jwt:
  enabled: true
  secret: "your-admin-secret-key"
  claims:
    - claim: "role"
      operator: "equals"
      value: "admin"  # Only admin role can reload
```

### Credential Rotation
When rotating S3 credentials:

1. Update credentials in S3/IAM
2. Update config.yaml with new credentials
3. Reload configuration
4. Verify new requests succeed
5. Revoke old credentials in S3/IAM

**Time-sensitive:** Complete rotation quickly to minimize window with both credentials active.

### JWT Secret Rotation
When rotating JWT secrets:

1. Issue new tokens with new secret before reload
2. Reload configuration with new secret
3. Revoke old tokens (set short expiration)

**No grace period:** Old tokens immediately invalid after reload.

---

## Summary

| Configuration Type | Hot Reload | Restart Required |
|--------------------|------------|------------------|
| Bucket credentials | ✅ | |
| Bucket path prefix | ✅ | |
| Add/remove bucket | ✅ | |
| JWT secret | ✅ | |
| JWT algorithm | ✅ | |
| Custom claims | ✅ | |
| Token sources | ✅ | |
| Server address | | ❌ |
| Server port | | ❌ |
| Worker threads | | ❌ |

**Reload Methods:**
- SIGHUP: `kill -HUP $(pgrep yatagarasu)`
- API: `POST /admin/reload` (requires admin JWT)

**Monitoring:**
- Metrics: `/metrics` endpoint
- Counters: `config_reload_success_total`, `config_reload_failure_total`
- Gauge: `config_generation`
