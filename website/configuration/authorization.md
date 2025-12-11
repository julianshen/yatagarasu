---
title: Authorization
layout: default
parent: Configuration
nav_order: 4
---

# Authorization Configuration

Configure OPA or OpenFGA for fine-grained access control.
{: .fs-6 .fw-300 }

---

## Overview

Authorization is separate from authentication:
- **Authentication**: Verifies identity (JWT validation)
- **Authorization**: Determines permissions (OPA/OpenFGA)

Authorization runs after successful authentication.

---

## OPA (Open Policy Agent)

Policy-based authorization using Rego policies.

```yaml
buckets:
  - name: "protected"
    path_prefix: "/protected"
    s3: { ... }
    auth:
      enabled: true
      jwt: { ... }
    authorization:
      type: "opa"
      url: "http://opa:8181"
      policy_path: "yatagarasu/authz/allow"
      timeout_ms: 100
      cache_ttl_seconds: 60
```

### OPA Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `type` | string | - | Must be `"opa"` |
| `url` | string | - | OPA server URL |
| `policy_path` | string | - | Rego policy path |
| `timeout_ms` | integer | 100 | Request timeout |
| `cache_ttl_seconds` | integer | 60 | Decision cache TTL |

### OPA Input Format

Yatagarasu sends this input to OPA:

```json
{
  "input": {
    "method": "GET",
    "path": "/protected/file.txt",
    "bucket": "protected-bucket",
    "claims": {
      "sub": "user123",
      "role": "admin",
      ...
    },
    "headers": {
      "host": "example.com",
      ...
    }
  }
}
```

### Example OPA Policy

```rego
package yatagarasu.authz

default allow = false

# Allow admins to access everything
allow {
    input.claims.role == "admin"
}

# Allow users to read their own files
allow {
    input.method == "GET"
    startswith(input.path, concat("/", ["/users", input.claims.sub]))
}

# Allow access to public paths
allow {
    startswith(input.path, "/public/")
}
```

---

## OpenFGA

Fine-grained ReBAC (Relationship-Based Access Control).

```yaml
buckets:
  - name: "documents"
    path_prefix: "/docs"
    s3: { ... }
    auth:
      enabled: true
      jwt: { ... }
    authorization:
      type: "openfga"
      url: "http://openfga:8080"
      store_id: "${OPENFGA_STORE_ID}"
      model_id: "${OPENFGA_MODEL_ID}"
      timeout_ms: 100
      cache_ttl_seconds: 60
```

### OpenFGA Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `type` | string | - | Must be `"openfga"` |
| `url` | string | - | OpenFGA server URL |
| `store_id` | string | - | OpenFGA store ID |
| `model_id` | string | - | Authorization model ID |
| `timeout_ms` | integer | 100 | Request timeout |
| `cache_ttl_seconds` | integer | 60 | Decision cache TTL |

### OpenFGA Check Format

Yatagarasu performs these checks:

```
User: user:<jwt.sub>
Relation: viewer (for GET/HEAD)
Object: document:<path>
```

### Example OpenFGA Model

```dsl
model
  schema 1.1

type user

type folder
  relations
    define owner: [user]
    define editor: [user] or owner
    define viewer: [user] or editor

type document
  relations
    define parent: [folder]
    define owner: [user]
    define editor: [user] or owner or editor from parent
    define viewer: [user] or editor or viewer from parent
```

---

## Authorization Flow

```
Request with JWT
       |
       v
+----------------+
| JWT Validation |
+----------------+
       |
       | (success)
       v
+----------------+
| Authorization  |
| (OPA/OpenFGA)  |
+----------------+
       |
   +---+---+
   |       |
 Allow   Deny
   |       |
   v       v
  200     403
```

---

## Complete Examples

### OPA with Role-Based Access

```yaml
buckets:
  - name: "api"
    path_prefix: "/api"
    s3:
      bucket: "api-data"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"
    authorization:
      type: "opa"
      url: "http://opa:8181"
      policy_path: "api/authz/allow"
      cache_ttl_seconds: 300
```

OPA Policy:

```rego
package api.authz

default allow = false

# API v1 - requires api-v1 scope
allow {
    startswith(input.path, "/api/v1/")
    input.claims.scopes[_] == "api-v1"
}

# API v2 - requires api-v2 scope
allow {
    startswith(input.path, "/api/v2/")
    input.claims.scopes[_] == "api-v2"
}

# Admin endpoints
allow {
    startswith(input.path, "/api/admin/")
    input.claims.role == "admin"
}
```

### OpenFGA Document Access

```yaml
buckets:
  - name: "documents"
    path_prefix: "/docs"
    s3:
      bucket: "documents"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: true
      jwt:
        algorithm: "RS256"
        jwks_url: "https://auth.example.com/.well-known/jwks.json"
        token_sources:
          - type: "bearer"
    authorization:
      type: "openfga"
      url: "http://openfga:8080"
      store_id: "${OPENFGA_STORE_ID}"
      cache_ttl_seconds: 60
```

Relationships:

```
# User can view document
user:alice -> viewer -> document:/docs/report.pdf

# User is folder editor (inherits document access)
user:bob -> editor -> folder:/docs/team/
```

---

## Decision Caching

Authorization decisions are cached to reduce latency:

```yaml
authorization:
  cache_ttl_seconds: 60  # Cache decisions for 1 minute
```

Cache key includes:
- User identifier (JWT sub)
- Resource path
- Request method

---

## Error Handling

| Scenario | Response |
|:---------|:---------|
| Authorization server unavailable | 503 Service Unavailable |
| Authorization timeout | 503 Service Unavailable |
| Access denied | 403 Forbidden |
| Invalid authorization response | 500 Internal Error |

---

## Metrics

```prometheus
# Authorization checks
yatagarasu_authorization_checks_total{type="opa|openfga",result="allow|deny"}

# Authorization latency
yatagarasu_authorization_duration_seconds{type="opa|openfga"}

# Cache hits
yatagarasu_authorization_cache_hits_total
yatagarasu_authorization_cache_misses_total
```

---

## Best Practices

1. **Cache decisions** - Reduce latency with appropriate TTL
2. **Handle failures gracefully** - Decide on fail-open vs fail-closed
3. **Use specific policies** - Don't make policies too broad
4. **Monitor decision latency** - Keep under 100ms
5. **Test policies thoroughly** - Use unit tests for OPA policies

---

## See Also

- [OPA Authorization Tutorial](/yatagarasu/tutorials/opa-authorization/)
- [OpenFGA Tutorial](/yatagarasu/tutorials/openfga/)
- [Authentication Configuration](/yatagarasu/configuration/authentication/)
