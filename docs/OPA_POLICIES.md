# OPA Integration Guide

This guide covers Open Policy Agent (OPA) integration with Yatagarasu for flexible, policy-based authorization.

## Overview

Yatagarasu integrates with OPA to provide fine-grained access control beyond what JWT claims validation can offer. With OPA, you can implement:

- Role-based access control (RBAC)
- Attribute-based access control (ABAC)
- Time-based access restrictions
- IP-based restrictions
- Complex conditional policies

## Configuration

### Enabling OPA Authorization

Add an `authorization` section to your bucket configuration:

```yaml
buckets:
  - name: products
    path_prefix: /products
    jwt:
      enabled: true
      algorithm: HS256
      secret: ${JWT_SECRET}
    authorization:
      type: opa
      url: http://localhost:8181
      policy_path: yatagarasu/authz/allow
      timeout_ms: 100          # Fast fail (default: 100ms)
      cache_ttl_seconds: 60    # Cache decisions for 60s
      fail_mode: closed        # Deny on OPA failure (default)
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `type` | Authorization type (currently only `opa`) | Required |
| `url` | OPA server base URL | Required |
| `policy_path` | Path to policy decision endpoint | Required |
| `timeout_ms` | Request timeout in milliseconds | `100` |
| `cache_ttl_seconds` | How long to cache decisions | `60` |
| `fail_mode` | Behavior on OPA error: `open` or `closed` | `closed` |

### Fail Modes

- **`closed`** (default): Deny requests when OPA is unreachable. More secure, recommended for production.
- **`open`**: Allow requests when OPA is unreachable. Higher availability, but less secure.

## OPA Input Format

Yatagarasu sends the following input to OPA for each authorization decision:

```json
{
  "input": {
    "jwt_claims": {
      "sub": "user123",
      "roles": ["admin", "developer"],
      "department": "engineering",
      "exp": 1700000000,
      "iat": 1699990000
    },
    "bucket": "products",
    "path": "/products/engineering/secret.pdf",
    "method": "GET",
    "client_ip": "192.168.1.100"
  }
}
```

### Input Fields

| Field | Description |
|-------|-------------|
| `jwt_claims` | All claims from the validated JWT token |
| `bucket` | Name of the bucket being accessed |
| `path` | Full request path |
| `method` | HTTP method (`GET` or `HEAD`) |
| `client_ip` | Client IP address (when available) |

## Example Rego Policies

> **Note:** These examples use modern Rego v1 syntax which requires the `if` keyword
> before rule bodies. Older Rego syntax without `if` is deprecated and will not work
> with recent OPA versions (0.55+). See the [OPA Rego v1 migration guide](https://www.openpolicyagent.org/docs/latest/opa-1/) for details.

### Basic Admin Role Policy

```rego
# policies/yatagarasu/authz/allow.rego
package yatagarasu.authz

default allow = false

# Allow admins to access everything
allow if {
    input.jwt_claims.roles[_] == "admin"
}
```

### Department-Based Access

```rego
package yatagarasu.authz

default allow = false

# Allow users to access their own department's files
allow if {
    input.jwt_claims.department == path_department
}

# Extract department from path: /products/{department}/file.txt
path_department := dept if {
    parts := split(input.path, "/")
    count(parts) > 2
    dept := parts[2]
}
```

### Combined Role and Department Policy

```rego
package yatagarasu.authz

default allow = false

# Admins can access everything
allow if {
    input.jwt_claims.roles[_] == "admin"
}

# Users can access their department's files
allow if {
    input.jwt_claims.department == path_department
}

# Managers can access any department
allow if {
    input.jwt_claims.roles[_] == "manager"
}

path_department := dept if {
    parts := split(input.path, "/")
    count(parts) > 2
    dept := parts[2]
}
```

### Time-Based Access (Business Hours Only)

```rego
package yatagarasu.authz

default allow = false

# Contractors can only access during business hours (9 AM - 5 PM)
allow if {
    input.jwt_claims.roles[_] == "contractor"
    is_business_hours
}

is_business_hours if {
    now := time.now_ns()
    [hour, _, _] := time.clock(now)
    hour >= 9
    hour < 17
}

# Full-time employees have 24/7 access
allow if {
    input.jwt_claims.employment_type == "full-time"
}
```

### IP-Based Restrictions

```rego
package yatagarasu.authz

default allow = false

# Internal users must access from internal network
allow if {
    input.jwt_claims.roles[_] == "internal"
    is_internal_ip
}

is_internal_ip if {
    net.cidr_contains("10.0.0.0/8", input.client_ip)
}

is_internal_ip if {
    net.cidr_contains("192.168.0.0/16", input.client_ip)
}

# External users can access from anywhere
allow if {
    input.jwt_claims.roles[_] == "external"
}
```

### Bucket-Specific Policies

```rego
package yatagarasu.authz

default allow = false

# Public bucket - allow all authenticated users
allow if {
    input.bucket == "public-assets"
}

# Private bucket - admins only
allow if {
    input.bucket == "private-data"
    input.jwt_claims.roles[_] == "admin"
}

# Products bucket - product team members
allow if {
    input.bucket == "products"
    input.jwt_claims.team == "product"
}
```

### Path Pattern Matching

```rego
package yatagarasu.authz

default allow = false

# Allow access to public directories
allow if {
    startswith(input.path, "/public/")
}

# Block access to hidden files
deny if {
    contains(input.path, "/.")
}

# Final decision
allow if {
    not deny
    some_positive_rule
}

some_positive_rule if {
    input.jwt_claims.authenticated == true
}
```

### Detailed Response with Reason

```rego
package yatagarasu.authz

default allow := {
    "allowed": false,
    "reason": "No matching policy rule"
}

# Admin access
allow := result if {
    input.jwt_claims.roles[_] == "admin"
    result := {
        "allowed": true,
        "reason": "Admin role grants full access"
    }
}

# Department access
allow := result if {
    input.jwt_claims.department == path_department
    result := {
        "allowed": true,
        "reason": sprintf("User belongs to department %s", [path_department])
    }
}

path_department := dept if {
    parts := split(input.path, "/")
    count(parts) > 2
    dept := parts[2]
}
```

## Deployment

### Docker Compose

```yaml
version: '3.8'

services:
  opa:
    image: openpolicyagent/opa:latest
    command:
      - "run"
      - "--server"
      - "--addr=0.0.0.0:8181"
      - "--log-level=info"
      - "/policies"
    volumes:
      - ./policies:/policies:ro
    ports:
      - "8181:8181"
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8181/health"]
      interval: 10s
      timeout: 5s
      retries: 3

  yatagarasu:
    image: yatagarasu:latest
    environment:
      - OPA_URL=http://opa:8181
      - JWT_SECRET=${JWT_SECRET}
    depends_on:
      opa:
        condition: service_healthy
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: opa
spec:
  replicas: 2
  selector:
    matchLabels:
      app: opa
  template:
    metadata:
      labels:
        app: opa
    spec:
      containers:
        - name: opa
          image: openpolicyagent/opa:latest
          args:
            - "run"
            - "--server"
            - "--addr=0.0.0.0:8181"
            - "--config-file=/config/config.yaml"
          ports:
            - containerPort: 8181
          volumeMounts:
            - name: policies
              mountPath: /policies
            - name: config
              mountPath: /config
          readinessProbe:
            httpGet:
              path: /health
              port: 8181
            initialDelaySeconds: 5
            periodSeconds: 10
      volumes:
        - name: policies
          configMap:
            name: opa-policies
        - name: config
          configMap:
            name: opa-config
---
apiVersion: v1
kind: Service
metadata:
  name: opa
spec:
  selector:
    app: opa
  ports:
    - port: 8181
      targetPort: 8181
```

## Policy Management

### Loading Policies

**Via Volume Mount** (recommended for production):
```bash
# Place .rego files in ./policies directory
docker run -v $(pwd)/policies:/policies openpolicyagent/opa:latest \
  run --server /policies
```

**Via OPA API** (for dynamic updates):
```bash
# Upload a policy
curl -X PUT http://localhost:8181/v1/policies/authz \
  -H "Content-Type: text/plain" \
  --data-binary @policies/authz.rego

# List policies
curl http://localhost:8181/v1/policies

# Delete a policy
curl -X DELETE http://localhost:8181/v1/policies/authz
```

### Testing Policies

**Using OPA REPL**:
```bash
# Start OPA with policies
opa run policies/

# In REPL, test a decision
> data.yatagarasu.authz.allow with input as {
    "jwt_claims": {"roles": ["admin"]},
    "bucket": "products",
    "path": "/products/file.txt",
    "method": "GET"
  }
true
```

**Using OPA API**:
```bash
# Test a policy decision
curl -X POST http://localhost:8181/v1/data/yatagarasu/authz/allow \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "jwt_claims": {"sub": "user1", "roles": ["admin"]},
      "bucket": "products",
      "path": "/products/file.txt",
      "method": "GET"
    }
  }'
# Response: {"result": true}
```

### Policy Unit Tests

Create `_test.rego` files alongside your policies:

```rego
# policies/authz_test.rego
package yatagarasu.authz

test_admin_allowed if {
    allow with input as {
        "jwt_claims": {"roles": ["admin"]},
        "bucket": "products",
        "path": "/products/file.txt",
        "method": "GET"
    }
}

test_non_admin_denied if {
    not allow with input as {
        "jwt_claims": {"roles": ["user"]},
        "bucket": "products",
        "path": "/products/file.txt",
        "method": "GET"
    }
}
```

Run tests:
```bash
opa test policies/ -v
```

## Performance Considerations

### Caching

Yatagarasu caches OPA decisions to minimize latency:

- **Cache Key**: SHA-256 hash of the full input (JWT claims, bucket, path, method, IP)
- **TTL**: Configurable via `cache_ttl_seconds` (default 60s)
- **Cache Size**: Up to 10,000 entries

For frequently accessed resources with the same user, caching significantly reduces OPA calls.

### Timeout

The default timeout is 100ms to ensure fast fail behavior. If OPA consistently times out:

1. Check OPA server health and resources
2. Simplify complex policy rules
3. Consider increasing `timeout_ms` (not recommended beyond 500ms)
4. Add more OPA replicas for load distribution

### Monitoring

Monitor these metrics for OPA integration health:

- `yatagarasu_opa_requests_total` - Total OPA evaluation requests
- `yatagarasu_opa_request_duration_seconds` - OPA request latency
- `yatagarasu_opa_cache_hits_total` - Cache hit count
- `yatagarasu_opa_cache_misses_total` - Cache miss count
- `yatagarasu_opa_errors_total` - OPA errors by type

## Troubleshooting

### Common Issues

**OPA returns undefined (empty result)**:
- Ensure your policy package matches the `policy_path` in config
- Check that the policy has a `default allow = false` rule
- Verify input field names match what the policy expects

**Connection refused**:
- Verify OPA is running and accessible from Yatagarasu
- Check firewall rules and network policies
- Ensure correct URL in configuration

**Timeout errors**:
- Check OPA server resources (CPU, memory)
- Simplify complex policy rules
- Increase timeout if policies are legitimately slow

**Cache not working**:
- Verify `cache_ttl_seconds` is set
- Check that identical requests have identical inputs
- Monitor cache hit/miss metrics

### Debug Mode

Enable debug logging to see OPA requests and responses:

```yaml
logging:
  level: debug
```

This will log:
- Full OPA input for each request
- OPA response status and result
- Cache hit/miss events
- Timing information

## Load Testing with OPA

Yatagarasu includes k6 load test scripts for testing OPA authorization performance.

### Prerequisites

1. **Start OPA server**:
```bash
docker run -d -p 8181:8181 --name opa \
  openpolicyagent/opa:latest run --server --addr=0.0.0.0:8181
```

2. **Load test policy**:
```bash
curl -X PUT http://localhost:8181/v1/policies/authz \
  -H "Content-Type: text/plain" \
  --data-binary @policies/loadtest-authz.rego
```

3. **Start MinIO** (or use existing S3):
```bash
docker run -d -p 9000:9000 --name minio \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data
```

4. **Create test buckets and files**:
```bash
# Install mc (MinIO client)
mc alias set local http://localhost:9000 minioadmin minioadmin
mc mb local/test-opa
mc cp /path/to/test-file.txt local/test-opa/
```

5. **Start proxy with OPA config**:
```bash
cargo run -- --config config.loadtest-opa.yaml
```

### Running Load Tests

**Basic OPA load test**:
```bash
k6 run k6-opa.js
```

**With custom options**:
```bash
k6 run k6-opa.js \
  --env BASE_URL=http://localhost:8080 \
  --env ADMIN_TOKEN="<your-admin-jwt>" \
  --env USER_TOKEN="<your-user-jwt>"
```

### Test Scenarios

The OPA load test (`k6-opa.js`) includes four scenarios:

| Scenario | Description | Target |
|----------|-------------|--------|
| `opa_constant_rate` | 500 req/s for 30s | Measure baseline throughput |
| `opa_ramping` | 10→100→50 VUs | Find saturation point |
| `opa_cache_hit` | 1000 req/s, same user | Test cache effectiveness |
| `opa_cache_miss` | 200 req/s, unique paths | Test OPA without cache |

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| P95 latency | <200ms | With OPA + S3 backend |
| Auth latency (P95) | <50ms | OPA evaluation only |
| Error rate | <1% | All responses |
| Throughput | >500 req/s | With OPA enabled |

### Comparing With and Without OPA

To measure OPA overhead:

```bash
# Test without OPA (JWT only)
k6 run k6-baseline.js

# Test with OPA
k6 run k6-opa.js

# Compare results
```

Expected overhead from OPA:
- First request: +10-50ms (OPA evaluation)
- Cached requests: +1-5ms (cache lookup)

### Generating Test JWTs

Use the following script to generate valid test JWTs:

```bash
# Generate admin token
jwt encode --secret "test-secret-key-for-load-testing-only" \
  '{"sub":"admin","roles":["admin"],"exp":1900000000}'

# Generate user token
jwt encode --secret "test-secret-key-for-load-testing-only" \
  '{"sub":"user1","roles":["user"],"allowed_bucket":"test-opa","exp":1900000000}'
```

Or use online JWT tools with these payloads.
