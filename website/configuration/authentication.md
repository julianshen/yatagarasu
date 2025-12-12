---
title: Authentication
layout: default
parent: Configuration
nav_order: 3
---

# Authentication Configuration

Configure JWT authentication for protected buckets.
{: .fs-6 .fw-300 }

---

## Basic Configuration

```yaml
buckets:
  - name: "private"
    path_prefix: "/private"
    s3: { ... }
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"
```

---

## Auth Options

### auth.enabled

Enable or disable authentication for this bucket.

| | |
|:--|:--|
| **Type** | `boolean` |
| **Default** | `false` |
| **Required** | No |

```yaml
auth:
  enabled: true   # JWT required
  enabled: false  # Public access
```

---

## JWT Options

### jwt.algorithm

JWT signing algorithm.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes (if auth enabled) |
| **Values** | `HS256`, `HS384`, `HS512`, `RS256`, `RS384`, `RS512`, `ES256`, `ES384`, `ES512` |

```yaml
jwt:
  algorithm: "HS256"  # HMAC with SHA-256 (symmetric)
  algorithm: "RS256"  # RSA with SHA-256 (asymmetric)
  algorithm: "ES256"  # ECDSA with SHA-256 (asymmetric)
```

---

### jwt.secret

Shared secret for HMAC algorithms (HS256, HS384, HS512).

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes (for HS* algorithms) |

```yaml
jwt:
  algorithm: "HS256"
  secret: "${JWT_SECRET}"
```

{: .warning }
Use a strong secret (32+ characters) and never commit to version control.

---

### jwt.public_key_path

Path to public key file for RSA/ECDSA algorithms.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes (for RS*/ES* algorithms without JWKS) |

```yaml
jwt:
  algorithm: "RS256"
  public_key_path: "/etc/yatagarasu/public.pem"
```

---

### jwt.jwks_url

URL to JWKS (JSON Web Key Set) endpoint for dynamic key rotation.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | No |

```yaml
jwt:
  algorithm: "RS256"
  jwks_url: "https://auth.example.com/.well-known/jwks.json"
  jwks_cache_ttl_seconds: 3600  # Cache keys for 1 hour
```

---

### jwt.token_sources

Where to look for the JWT token. Checked in order.

| | |
|:--|:--|
| **Type** | `array` |
| **Required** | Yes |

```yaml
jwt:
  token_sources:
    # Authorization: Bearer <token>
    - type: "bearer"

    # ?token=<token>
    - type: "query"
      name: "token"

    # X-Auth-Token: <token>
    - type: "header"
      name: "X-Auth-Token"
```

#### Token Source Types

| Type | Description | Options |
|:-----|:------------|:--------|
| `bearer` | `Authorization: Bearer <token>` | - |
| `query` | Query parameter | `name`: parameter name |
| `header` | Custom header | `name`: header name, `prefix`: optional prefix |

---

### jwt.claims_verification

Validate specific JWT claims.

{: .note }
> **When to use claims_verification vs authorization (OPA/OpenFGA)?**
> Use `claims_verification` for simple, static checks (e.g., role equals "admin").
> Use [OPA/OpenFGA authorization](/yatagarasu/configuration/authorization/) for complex, dynamic policies
> (e.g., resource-based access, relationship-based permissions).

| | |
|:--|:--|
| **Type** | `array` |
| **Required** | No |

```yaml
jwt:
  claims_verification:
    - claim: "role"
      operator: "equals"
      value: "admin"

    - claim: "roles"
      operator: "contains"
      value: "editor"

    - claim: "level"
      operator: "gte"
      value: 5
```

#### Operators

| Operator | Description | Example |
|:---------|:------------|:--------|
| `equals` | Exact match | `role == "admin"` |
| `not_equals` | Not equal | `status != "banned"` |
| `contains` | String/array contains | `roles contains "admin"` |
| `in` | Value in array | `role in ["admin", "mod"]` |
| `gt` | Greater than | `level > 5` |
| `lt` | Less than | `age < 18` |
| `gte` | Greater or equal | `score >= 100` |
| `lte` | Less or equal | `tries <= 3` |
| `exists` | Claim exists | `email exists` |
| `not_exists` | Claim doesn't exist | `banned not_exists` |

---

## Algorithm Configurations

### HS256 (Symmetric)

```yaml
jwt:
  algorithm: "HS256"
  secret: "${JWT_SECRET}"
  token_sources:
    - type: "bearer"
```

### RS256 (Asymmetric - RSA)

```yaml
jwt:
  algorithm: "RS256"
  public_key_path: "/etc/yatagarasu/rsa_public.pem"
  token_sources:
    - type: "bearer"
```

### ES256 (Asymmetric - ECDSA)

```yaml
jwt:
  algorithm: "ES256"
  public_key_path: "/etc/yatagarasu/ec_public.pem"
  token_sources:
    - type: "bearer"
```

### JWKS (Dynamic Keys)

```yaml
jwt:
  algorithm: "RS256"
  jwks_url: "https://auth.example.com/.well-known/jwks.json"
  jwks_cache_ttl_seconds: 3600
  token_sources:
    - type: "bearer"
```

---

## Complete Examples

### Simple HS256

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
```

### Multiple Token Sources

```yaml
auth:
  enabled: true
  jwt:
    secret: "${JWT_SECRET}"
    algorithm: "HS256"
    token_sources:
      # Try Authorization header first
      - type: "bearer"

      # Then query parameter
      - type: "query"
        name: "access_token"

      # Finally custom header
      - type: "header"
        name: "X-API-Key"
```

### Role-Based Access

```yaml
auth:
  enabled: true
  jwt:
    secret: "${JWT_SECRET}"
    algorithm: "HS256"
    token_sources:
      - type: "bearer"
    claims_verification:
      - claim: "role"
        operator: "in"
        value: ["admin", "editor", "viewer"]
      - claim: "email_verified"
        operator: "equals"
        value: true
```

### Auth0 Integration

```yaml
auth:
  enabled: true
  jwt:
    algorithm: "RS256"
    jwks_url: "https://your-tenant.auth0.com/.well-known/jwks.json"
    jwks_cache_ttl_seconds: 3600
    token_sources:
      - type: "bearer"
    claims_verification:
      - claim: "iss"
        operator: "equals"
        value: "https://your-tenant.auth0.com/"
      - claim: "aud"
        operator: "contains"
        value: "your-api-identifier"
```

### Keycloak Integration

```yaml
auth:
  enabled: true
  jwt:
    algorithm: "RS256"
    jwks_url: "https://keycloak.example.com/realms/myrealm/protocol/openid-connect/certs"
    jwks_cache_ttl_seconds: 3600
    token_sources:
      - type: "bearer"
    claims_verification:
      - claim: "iss"
        operator: "equals"
        value: "https://keycloak.example.com/realms/myrealm"
```

---

## Generating Keys

### RSA Keys (RS256)

```bash
# Generate private key
openssl genrsa -out private.pem 2048

# Extract public key
openssl rsa -in private.pem -pubout -out public.pem
```

### ECDSA Keys (ES256)

```bash
# Generate private key
openssl ecparam -genkey -name prime256v1 -noout -out private.pem

# Extract public key
openssl ec -in private.pem -pubout -out public.pem
```

---

## Error Responses

| Error | HTTP Status | Description |
|:------|:------------|:------------|
| Missing token | 401 | No token found in configured sources |
| Invalid token | 401 | Token failed signature validation |
| Expired token | 401 | Token `exp` claim is in the past |
| Invalid claims | 403 | Claims verification failed |

---

## Best Practices

1. **Use RS256/ES256 in production** - Asymmetric algorithms are more secure
2. **Use JWKS for key rotation** - Enables zero-downtime key rotation
3. **Set short expiry times** - 15 minutes to 1 hour
4. **Validate issuer and audience** - Prevent token misuse
5. **Use HTTPS** - Always encrypt tokens in transit

---

## See Also

- [JWT Authentication Tutorial](/yatagarasu/tutorials/jwt-authentication/)
- [Authorization Configuration](/yatagarasu/configuration/authorization/)
- [OPA Tutorial](/yatagarasu/tutorials/opa-authorization/)
