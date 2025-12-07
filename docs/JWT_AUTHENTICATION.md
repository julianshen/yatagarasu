# YATAGARASU - JWT AUTHENTICATION GUIDE

**Version**: v1.2.0
**Feature**: JWT Authentication with RS256/ES256/JWKS Support

---

## TABLE OF CONTENTS

1. [Overview](#overview)
2. [Supported Algorithms](#supported-algorithms)
3. [Configuration Examples](#configuration-examples)
4. [JWKS Integration](#jwks-integration)
5. [Token Sources](#token-sources)
6. [Claims Verification](#claims-verification)
7. [Admin Authentication](#admin-authentication)
8. [Troubleshooting](#troubleshooting)

---

## OVERVIEW

Yatagarasu supports flexible JWT authentication with multiple algorithms and token sources. Authentication can be configured per-bucket, allowing mixed public/private bucket setups.

### Key Features

- **Multiple algorithms**: HS256/384/512, RS256/384/512, ES256/384
- **JWKS support**: Automatic key rotation from JWKS endpoints
- **Flexible token sources**: Header, query parameter, custom headers
- **Claims verification**: Custom claim rules with operators
- **Admin authentication**: Separate claims for admin API access

---

## SUPPORTED ALGORITHMS

### Symmetric (HMAC)

| Algorithm | Key Size | Use Case |
|-----------|----------|----------|
| HS256 | 256-bit | General purpose, fast |
| HS384 | 384-bit | Higher security |
| HS512 | 512-bit | Maximum HMAC security |

**Pros**: Fast, simple key management
**Cons**: Shared secret must be distributed securely

### Asymmetric (RSA)

| Algorithm | Key Size | Use Case |
|-----------|----------|----------|
| RS256 | 2048-bit+ | Standard RSA signatures |
| RS384 | 2048-bit+ | Higher security |
| RS512 | 2048-bit+ | Maximum RSA security |

**Pros**: Private key stays with issuer, public key can be shared
**Cons**: Slower than HMAC, larger keys

### Asymmetric (ECDSA)

| Algorithm | Curve | Use Case |
|-----------|-------|----------|
| ES256 | P-256 | Modern, efficient |
| ES384 | P-384 | Higher security |

**Pros**: Smaller keys, faster than RSA at equivalent security
**Cons**: Slightly more complex implementation

---

## CONFIGURATION EXAMPLES

### Example 1: HS256 with Shared Secret

The simplest configuration using HMAC-SHA256:

```yaml
buckets:
  - name: private-bucket
    path: /private
    jwt:
      enabled: true
      algorithm: HS256
      secret: "${JWT_SECRET}"  # Use environment variable
      token_sources:
        - type: header
          name: Authorization
          prefix: "Bearer "
```

**Environment**:
```bash
export JWT_SECRET="your-256-bit-secret-key-here"
```

### Example 2: RS256 with RSA Public Key

Using RSA signatures with a PEM public key file:

```yaml
buckets:
  - name: secure-bucket
    path: /secure
    jwt:
      enabled: true
      algorithm: RS256
      rsa_public_key_path: /etc/yatagarasu/keys/public.pem
      token_sources:
        - type: header
          name: Authorization
          prefix: "Bearer "
```

**Generate RSA Keys**:
```bash
# Generate private key (keep secure)
openssl genrsa -out private.pem 2048

# Extract public key (distribute this)
openssl rsa -in private.pem -pubout -out public.pem
```

### Example 3: ES256 with ECDSA Key

Using ECDSA P-256 curve for efficient signatures:

```yaml
buckets:
  - name: api-bucket
    path: /api
    jwt:
      enabled: true
      algorithm: ES256
      ecdsa_public_key_path: /etc/yatagarasu/keys/ec-public.pem
      token_sources:
        - type: header
          name: Authorization
          prefix: "Bearer "
```

**Generate ECDSA Keys**:
```bash
# Generate private key (keep secure)
openssl ecparam -name prime256v1 -genkey -noout -out ec-private.pem

# Extract public key (distribute this)
openssl ec -in ec-private.pem -pubout -out ec-public.pem
```

---

## JWKS INTEGRATION

JWKS (JSON Web Key Set) allows automatic key rotation and multi-key support.

### Configuration

```yaml
buckets:
  - name: oauth-bucket
    path: /oauth
    jwt:
      enabled: true
      algorithm: RS256  # Default algorithm
      jwks_url: "https://your-idp.com/.well-known/jwks.json"
      jwks_refresh_interval_secs: 3600  # Refresh every hour
      token_sources:
        - type: header
          name: Authorization
          prefix: "Bearer "
```

### How JWKS Works

1. **Startup**: Yatagarasu fetches JWKS from the configured URL
2. **Caching**: Keys are cached for the configured interval
3. **Validation**: When validating a token, the `kid` (key ID) header is used to select the correct key
4. **Refresh**: Keys are automatically refreshed when cache expires
5. **Fallback**: On refresh failure, cached keys continue to work

### Common JWKS Providers

| Provider | JWKS URL Format |
|----------|-----------------|
| Auth0 | `https://{domain}/.well-known/jwks.json` |
| Okta | `https://{domain}/oauth2/default/v1/keys` |
| Keycloak | `https://{domain}/realms/{realm}/protocol/openid-connect/certs` |
| AWS Cognito | `https://cognito-idp.{region}.amazonaws.com/{userPoolId}/.well-known/jwks.json` |
| Google | `https://www.googleapis.com/oauth2/v3/certs` |

### JWKS Response Format

```json
{
  "keys": [
    {
      "kty": "RSA",
      "kid": "key-id-1",
      "use": "sig",
      "alg": "RS256",
      "n": "modulus-base64url",
      "e": "AQAB"
    },
    {
      "kty": "EC",
      "kid": "key-id-2",
      "use": "sig",
      "alg": "ES256",
      "crv": "P-256",
      "x": "x-coordinate-base64url",
      "y": "y-coordinate-base64url"
    }
  ]
}
```

---

## TOKEN SOURCES

Tokens can be extracted from multiple sources:

### Bearer Header (Default)

```yaml
token_sources:
  - type: header
    name: Authorization
    prefix: "Bearer "
```

**Request**: `Authorization: Bearer eyJhbGciOiJIUzI1NiIs...`

### Query Parameter

```yaml
token_sources:
  - type: query
    name: token
```

**Request**: `GET /path?token=eyJhbGciOiJIUzI1NiIs...`

### Custom Header

```yaml
token_sources:
  - type: header
    name: X-Auth-Token
```

**Request**: `X-Auth-Token: eyJhbGciOiJIUzI1NiIs...`

### Multiple Sources (Priority Order)

```yaml
token_sources:
  - type: header
    name: Authorization
    prefix: "Bearer "
  - type: header
    name: X-Auth-Token
  - type: query
    name: token
```

Sources are tried in order; first valid token wins.

---

## CLAIMS VERIFICATION

Custom claim rules allow fine-grained access control.

### Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `equals` | Exact match | `role equals admin` |
| `contains` | Substring match | `email contains @company.com` |
| `in` | Array membership | `role in [admin, editor]` |
| `gt` | Greater than (numeric) | `exp gt 1700000000` |
| `lt` | Less than (numeric) | `iat lt 1800000000` |
| `regex` | Regular expression | `email regex ^.*@company\.com$` |

### Configuration Examples

```yaml
jwt:
  enabled: true
  algorithm: HS256
  secret: "${JWT_SECRET}"
  claims:
    # Role must be admin
    - name: role
      operator: equals
      value: admin

    # Email must end with company domain
    - name: email
      operator: contains
      value: "@company.com"

    # Scope must include read:files
    - name: scope
      operator: contains
      value: "read:files"

    # User ID must be in allowed list
    - name: sub
      operator: in
      value: ["user-1", "user-2", "user-3"]
```

### Nested Claims

Access nested claims using dot notation:

```yaml
claims:
  - name: user.department
    operator: equals
    value: engineering

  - name: permissions.files.read
    operator: equals
    value: "true"
```

---

## ADMIN AUTHENTICATION

Phase 65.1 added admin claims for cache management API access.

### Configuration

```yaml
jwt:
  enabled: true
  algorithm: HS256
  secret: "${JWT_SECRET}"
  token_sources:
    - type: header
      name: Authorization
      prefix: "Bearer "
  # Regular claims for bucket access
  claims:
    - name: role
      operator: in
      value: ["user", "admin"]
  # Admin claims for /admin/* endpoints
  admin_claims:
    - name: role
      operator: equals
      value: admin
```

### Admin Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/admin/cache/stats` | GET | Cache statistics |
| `/admin/cache/purge` | POST | Purge cache entries |
| `/admin/cache/invalidate/{key}` | DELETE | Invalidate specific entry |

### Example Admin Token

```json
{
  "sub": "admin-user",
  "role": "admin",
  "iat": 1700000000,
  "exp": 1700086400
}
```

---

## TROUBLESHOOTING

### Common Issues

#### 1. "Invalid signature"

**Causes**:
- Wrong secret/key
- Algorithm mismatch
- Key encoding issues

**Solution**:
```bash
# Verify token manually
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq .

# Check algorithm in header
echo $TOKEN | cut -d'.' -f1 | base64 -d 2>/dev/null | jq .alg
```

#### 2. "Token expired"

**Causes**:
- Token `exp` claim is in the past
- Clock skew between issuer and proxy

**Solution**:
```bash
# Check expiration
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq '.exp'

# Compare with current time
date +%s
```

#### 3. "Key not found" (JWKS)

**Causes**:
- `kid` in token not in JWKS
- JWKS cache not refreshed

**Solution**:
```bash
# Check kid in token
echo $TOKEN | cut -d'.' -f1 | base64 -d 2>/dev/null | jq '.kid'

# Fetch JWKS and verify kid exists
curl -s https://your-idp.com/.well-known/jwks.json | jq '.keys[].kid'
```

#### 4. "Claims verification failed"

**Causes**:
- Required claim missing
- Claim value doesn't match rule

**Solution**:
```bash
# Decode and inspect claims
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq .

# Compare with configured claim rules
```

### Debug Logging

Enable debug logging for JWT validation:

```yaml
logging:
  level: debug
  format: json
```

Look for log entries with:
- `jwt_validation` tag
- `claims_check` tag
- `jwks_fetch` tag

---

## SECURITY BEST PRACTICES

1. **Use asymmetric keys** (RS256/ES256) in production
2. **Rotate keys regularly** via JWKS
3. **Keep secrets out of config** - use environment variables
4. **Set appropriate TTLs** - short-lived tokens reduce exposure
5. **Validate all required claims** - don't just check signature
6. **Use HTTPS** for JWKS endpoints
7. **Monitor failed authentications** - alert on spikes

---

## SEE ALSO

- [OPENFGA.md](OPENFGA.md) - Relationship-based authorization
- [OPA_POLICIES.md](OPA_POLICIES.md) - Policy-based authorization
- [OPERATIONS.md](OPERATIONS.md) - Monitoring and alerts

---

*Generated: December 2025*
*Yatagarasu v1.2.0 JWT Authentication Guide*
