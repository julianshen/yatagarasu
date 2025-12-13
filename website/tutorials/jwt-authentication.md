---
title: JWT Authentication
layout: default
parent: Tutorials
nav_order: 3
---

# JWT Authentication

Protect your S3 buckets with JWT tokens.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Configure JWT authentication on a bucket
- Generate and validate JWT tokens
- Use different token sources (header, query param)
- Add custom claims verification

## Prerequisites

- Completed the [Basic Proxy Setup](/yatagarasu/tutorials/basic-proxy/) tutorial
- Basic understanding of JWT tokens

---

## Step 1: Environment Setup

We'll use Docker Compose for this tutorial. Create a directory:

```bash
mkdir jwt-tutorial && cd jwt-tutorial
```

Create `docker-compose.yml`:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - JWT_SECRET=my-super-secret-key-for-jwt-signing
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
    depends_on:
      minio:
        condition: service_healthy

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 3

  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set local http://minio:9000 minioadmin minioadmin;
      mc mb local/public-bucket --ignore-existing;
      mc mb local/private-bucket --ignore-existing;
      echo 'Public content!' | mc pipe local/public-bucket/hello.txt;
      echo 'Private content!' | mc pipe local/private-bucket/secret.txt;
      "
```

---

## Step 2: Configure JWT Authentication

Create `config.yaml`:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  # Public bucket - no authentication
  - name: "public"
    path_prefix: "/public"
    s3:
      bucket: "public-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

  # Private bucket - JWT required
  - name: "private"
    path_prefix: "/private"
    s3:
      bucket: "private-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"  # Authorization: Bearer <token>
          - type: "query"
            name: "token"   # ?token=<token>

logging:
  level: "debug"
```

---

## Step 3: Start Services

```bash
docker compose up -d

# Wait for services to be ready
sleep 5

# Verify setup
docker compose ps
```

---

## Step 4: Test Public Access

```bash
# Public bucket works without authentication
curl http://localhost:8080/public/hello.txt
# Output: Public content!
```

---

## Step 5: Test Protected Access (No Token)

```bash
# Private bucket requires authentication
curl -i http://localhost:8080/private/secret.txt
# HTTP/1.1 401 Unauthorized
# {"error":"Missing authentication token"}
```

---

## Step 6: Generate a JWT Token

You can generate tokens using various tools. Here's how to do it with different methods:

### Option A: Using jwt.io (Web)

1. Go to [jwt.io](https://jwt.io)
2. Set algorithm to HS256
3. In the payload, add:
   ```json
   {
     "sub": "user123",
     "exp": 1999999999,
     "iat": 1600000000
   }
   ```
4. In the secret, enter: `my-super-secret-key-for-jwt-signing`
5. Copy the encoded token

### Option B: Using Node.js

```bash
# Install jsonwebtoken
npm install jsonwebtoken

# Generate token
node -e "
const jwt = require('jsonwebtoken');
const token = jwt.sign(
  { sub: 'user123', role: 'admin' },
  'my-super-secret-key-for-jwt-signing',
  { expiresIn: '1h' }
);
console.log(token);
"
```

### Option C: Using Python

```bash
pip install pyjwt

python3 -c "
import jwt
import time
token = jwt.encode(
    {'sub': 'user123', 'role': 'admin', 'exp': int(time.time()) + 3600},
    'my-super-secret-key-for-jwt-signing',
    algorithm='HS256'
)
print(token)
"
```

### Option D: Using a Pre-generated Token

For this tutorial, use this token (valid until 2033):

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIiwicm9sZSI6ImFkbWluIiwiZXhwIjoyMDAwMDAwMDAwfQ.5H8dSbD0Nx9j5RY9V8t5SkXhR7vB3kQr9X5gG7tGqIc
```

---

## Step 7: Access with JWT Token

### Using Authorization Header (Bearer)

```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIiwicm9sZSI6ImFkbWluIiwiZXhwIjoyMDAwMDAwMDAwfQ.5H8dSbD0Nx9j5RY9V8t5SkXhR7vB3kQr9X5gG7tGqIc"

curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/private/secret.txt
# Output: Private content!
```

### Using Query Parameter

```bash
curl "http://localhost:8080/private/secret.txt?token=$TOKEN"
# Output: Private content!
```

---

## Step 8: Add Custom Claims Verification

Update `config.yaml` to require specific claims:

```yaml
buckets:
  - name: "admin-only"
    path_prefix: "/admin"
    s3:
      bucket: "private-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"
        claims_verification:
          - claim: "role"
            operator: "equals"
            value: "admin"
```

Restart to apply changes:

```bash
docker compose restart yatagarasu
```

Now only tokens with `"role": "admin"` will work for the `/admin` path.

---

## Step 9: Using RS256 (Asymmetric Keys)

For production, RS256 with key pairs is more secure.

### Generate RSA Keys

```bash
# Generate private key
openssl genrsa -out private.pem 2048

# Generate public key
openssl rsa -in private.pem -pubout -out public.pem
```

### Configure RS256

```yaml
buckets:
  - name: "secure"
    path_prefix: "/secure"
    s3:
      bucket: "private-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        algorithm: "RS256"
        public_key_path: "/etc/yatagarasu/public.pem"
        token_sources:
          - type: "bearer"
```

Mount the public key:

```yaml
services:
  yatagarasu:
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
      - ./public.pem:/etc/yatagarasu/public.pem:ro
```

---

## Step 10: Using JWKS Endpoints

For dynamic key rotation, use JWKS:

```yaml
buckets:
  - name: "jwks-protected"
    path_prefix: "/jwks"
    s3:
      bucket: "private-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        algorithm: "RS256"
        jwks_url: "https://your-auth-server/.well-known/jwks.json"
        jwks_cache_ttl_seconds: 3600
        token_sources:
          - type: "bearer"
```

---

## Claims Verification Operators

| Operator | Description | Example |
|:---------|:------------|:--------|
| `equals` | Exact match | `role == "admin"` |
| `not_equals` | Not equal | `status != "banned"` |
| `contains` | String contains | `email contains "@example.com"` |
| `in` | Value in array | `role in ["admin", "moderator"]` |
| `gt` | Greater than (numbers) | `level > 5` |
| `lt` | Less than (numbers) | `age < 18` |
| `gte` | Greater or equal | `score >= 100` |
| `lte` | Less or equal | `tries <= 3` |

### Example with Multiple Claims

```yaml
claims_verification:
  - claim: "role"
    operator: "in"
    value: ["admin", "editor"]
  - claim: "email_verified"
    operator: "equals"
    value: true
  - claim: "subscription_level"
    operator: "gte"
    value: 2
```

---

## Custom Token Headers

Support tokens in custom headers:

```yaml
jwt:
  token_sources:
    - type: "header"
      name: "X-Auth-Token"
      prefix: ""  # No prefix needed
    - type: "header"
      name: "Authorization"
      prefix: "Bearer "
    - type: "query"
      name: "access_token"
```

```bash
# Using custom header
curl -H "X-Auth-Token: $TOKEN" http://localhost:8080/private/secret.txt
```

---

## Cleanup

```bash
docker compose down -v
cd .. && rm -rf jwt-tutorial
```

---

## Best Practices

1. **Use RS256 in production** - Asymmetric keys are more secure
2. **Short token expiry** - Use short-lived tokens (15 min - 1 hour)
3. **Validate claims** - Always verify relevant claims (issuer, audience, etc.)
4. **Use HTTPS** - Always use TLS in production
5. **Rotate secrets** - Regularly rotate signing keys

---

## Troubleshooting

### "Invalid signature"

- Verify the secret/key matches between token issuer and Yatagarasu
- Check the algorithm matches (HS256 vs RS256)

### "Token expired"

- Check the `exp` claim in your token
- Generate a new token with a future expiration

### "Missing required claim"

- Ensure your token includes all claims specified in `claims_verification`

---

## Next Steps

- [Multi-Bucket Routing](/yatagarasu/tutorials/multi-bucket/) - Configure multiple buckets
- [OPA Authorization](/yatagarasu/tutorials/opa-authorization/) - Policy-based access control
- [OpenFGA](/yatagarasu/tutorials/openfga/) - Fine-grained authorization
