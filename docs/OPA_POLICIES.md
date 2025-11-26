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

### Basic Admin Role Policy

```rego
# policies/yatagarasu/authz/allow.rego
package yatagarasu.authz

default allow = false

# Allow admins to access everything
allow {
    input.jwt_claims.roles[_] == "admin"
}
```

### Department-Based Access

```rego
package yatagarasu.authz

default allow = false

# Allow users to access their own department's files
allow {
    input.jwt_claims.department == path_department
}

# Extract department from path: /products/{department}/file.txt
path_department = dept {
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
allow {
    input.jwt_claims.roles[_] == "admin"
}

# Users can access their department's files
allow {
    input.jwt_claims.department == path_department
}

# Managers can access any department
allow {
    input.jwt_claims.roles[_] == "manager"
}

path_department = dept {
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
allow {
    input.jwt_claims.roles[_] == "contractor"
    is_business_hours
}

is_business_hours {
    now := time.now_ns()
    [hour, _, _] := time.clock(now)
    hour >= 9
    hour < 17
}

# Full-time employees have 24/7 access
allow {
    input.jwt_claims.employment_type == "full-time"
}
```

### IP-Based Restrictions

```rego
package yatagarasu.authz

default allow = false

# Internal users must access from internal network
allow {
    input.jwt_claims.roles[_] == "internal"
    is_internal_ip
}

is_internal_ip {
    net.cidr_contains("10.0.0.0/8", input.client_ip)
}

is_internal_ip {
    net.cidr_contains("192.168.0.0/16", input.client_ip)
}

# External users can access from anywhere
allow {
    input.jwt_claims.roles[_] == "external"
}
```

### Bucket-Specific Policies

```rego
package yatagarasu.authz

default allow = false

# Public bucket - allow all authenticated users
allow {
    input.bucket == "public-assets"
}

# Private bucket - admins only
allow {
    input.bucket == "private-data"
    input.jwt_claims.roles[_] == "admin"
}

# Products bucket - product team members
allow {
    input.bucket == "products"
    input.jwt_claims.team == "product"
}
```

### Path Pattern Matching

```rego
package yatagarasu.authz

default allow = false

# Allow access to public directories
allow {
    startswith(input.path, "/public/")
}

# Block access to hidden files
deny {
    contains(input.path, "/.")
}

# Final decision
allow {
    not deny
    some_positive_rule
}

some_positive_rule {
    input.jwt_claims.authenticated == true
}
```

### Detailed Response with Reason

```rego
package yatagarasu.authz

default allow = {
    "allowed": false,
    "reason": "No matching policy rule"
}

# Admin access
allow = result {
    input.jwt_claims.roles[_] == "admin"
    result := {
        "allowed": true,
        "reason": "Admin role grants full access"
    }
}

# Department access
allow = result {
    input.jwt_claims.department == path_department
    result := {
        "allowed": true,
        "reason": sprintf("User belongs to department %s", [path_department])
    }
}

path_department = dept {
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

test_admin_allowed {
    allow with input as {
        "jwt_claims": {"roles": ["admin"]},
        "bucket": "products",
        "path": "/products/file.txt",
        "method": "GET"
    }
}

test_non_admin_denied {
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
