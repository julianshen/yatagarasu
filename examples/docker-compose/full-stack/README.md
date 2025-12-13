# Full Stack Docker Compose Example

Complete production-like setup with HA proxy instances, Redis cache, OPA policy engine, and OpenFGA fine-grained authorization.

## Architecture

```
                         ┌─────────────┐
                         │   Client    │
                         └──────┬──────┘
                                │
                         ┌──────▼──────┐
                         │    Nginx    │ :8080
                         │ Load Balancer│
                         └──────┬──────┘
                                │
            ┌───────────────────┼───────────────────┐
            │                   │                   │
     ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
     │ Yatagarasu  │     │ Yatagarasu  │     │ Yatagarasu  │
     │  Instance 1 │     │  Instance 2 │     │  Instance N │
     └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
            │                   │                   │
            └───────────────────┼───────────────────┘
                                │
      ┌─────────────┬───────────┴───────────┬─────────────┐
      │             │                       │             │
┌─────▼─────┐ ┌─────▼─────┐          ┌──────▼─────┐ ┌─────▼─────┐
│   Redis   │ │   MinIO   │          │    OPA     │ │  OpenFGA  │
│   Cache   │ │ S3 Backend│          │   Policy   │ │  AuthZ    │
└───────────┘ └───────────┘          └────────────┘ └─────┬─────┘
                                                          │
                                                    ┌─────▼─────┐
                                                    │ PostgreSQL│
                                                    └───────────┘
```

## Prerequisites

- Docker and Docker Compose installed
- Ports 8080, 8181, 8082, 6379, 9000, 9001, 9090 available
- Python 3 with PyJWT for token generation: `pip install pyjwt`

## Quick Start

```bash
# Step 1: Start all services
docker compose up -d

# Step 2: Get OpenFGA store ID (created by openfga-setup)
export OPENFGA_STORE_ID=$(docker logs yatagarasu-full-openfga-setup 2>&1 | grep OPENFGA_STORE_ID | cut -d= -f2)
echo "Store ID: $OPENFGA_STORE_ID"

# Step 3: Restart yatagarasu with the store ID
docker compose up -d yatagarasu

# Test public bucket (no auth required)
curl http://localhost:8080/public/hello.txt

# Generate a test JWT token
TOKEN=$(python3 -c "
import jwt
import time
token = jwt.encode({
    'sub': 'alice',
    'role': 'admin',
    'exp': int(time.time()) + 3600
}, 'your-super-secret-jwt-key-change-in-production', algorithm='HS256')
print(token)
")

# Test OPA-protected bucket
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/opa/test.txt

# Test OpenFGA-protected bucket (alice=owner, bob=viewer, charlie=denied)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/fga/secret.txt
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| nginx | 8080 | Load balancer (entry point) |
| nginx | 9090 | Aggregated metrics |
| opa | 8181 | Open Policy Agent |
| openfga | 8082 | OpenFGA authorization |
| redis | 6379 | Shared cache |
| minio | 9000 | S3 API |
| minio | 9001 | MinIO Console |

## Buckets

| Path | Bucket | Authorization |
|------|--------|---------------|
| `/public/*` | public-assets | None (public) |
| `/opa/*` | opa-protected | OPA policy |
| `/fga/*` | openfga-protected | OpenFGA (after config) |

## OPA Policy

The included policy (`opa/policy.rego`) implements:

- **Admin role**: Full access to all paths
- **Reader role**: GET/HEAD access only
- **User-specific**: User "alice" can access `/opa/*`

Test the policy:
```bash
# Query OPA directly
curl -X POST http://localhost:8181/v1/data/yatagarasu/authz/allow \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "jwt_claims": {"sub": "alice", "role": "reader"},
      "method": "GET",
      "path": "/opa/test.txt"
    }
  }'
```

## OpenFGA Setup

OpenFGA requires a store ID which is created dynamically by the `openfga-setup` container.

```bash
# Get the store ID from setup logs
export OPENFGA_STORE_ID=$(docker logs yatagarasu-full-openfga-setup 2>&1 | grep OPENFGA_STORE_ID | cut -d= -f2)
echo "Store ID: $OPENFGA_STORE_ID"

# Restart yatagarasu with the store ID
docker compose up -d yatagarasu
```

**Note:** The store ID changes each time `openfga-setup` runs. If you restart all services with `docker compose down && docker compose up -d`, you'll need to get the new store ID and restart yatagarasu again.

### Authorization Model

The model defines three relation types:
- **viewer**: Read access (GET/HEAD requests)
- **editor**: Write access (PUT/POST requests)
- **owner**: Full access including delete (inherits viewer and editor)

Files inherit permissions from their parent bucket via the `parent` relation.

## Configuration Files

| File | Description |
|------|-------------|
| `config.yaml` | Proxy configuration |
| `nginx.conf` | Load balancer config |
| `opa/policy.rego` | OPA authorization policy |
| `openfga/model.json` | OpenFGA authorization model |
| `openfga/tuples.json` | Initial authorization tuples |

## Monitoring

```bash
# Health check
curl http://localhost:8080/health

# Prometheus metrics
curl http://localhost:9090/metrics

# Redis cache stats
docker exec yatagarasu-full-redis redis-cli INFO stats

# OPA decision logs
docker logs yatagarasu-full-opa
```

## Cleanup

```bash
# Stop all services
docker compose down

# Remove volumes (deletes all data)
docker compose down -v
```

## Customization

### Adding Users to OpenFGA

```bash
# Get store ID
STORE_ID=$(docker logs yatagarasu-full-openfga-setup 2>&1 | grep "OPENFGA_STORE_ID=" | cut -d= -f2)

# Add a new user with viewer access to the bucket
curl -X POST "http://localhost:8082/stores/$STORE_ID/write" \
  -H "Content-Type: application/json" \
  -d '{
    "writes": {
      "tuple_keys": [{
        "user": "user:dave",
        "relation": "viewer",
        "object": "bucket:openfga-protected"
      }]
    }
  }'

# For file-level access, also add the parent relationship
curl -X POST "http://localhost:8082/stores/$STORE_ID/write" \
  -H "Content-Type: application/json" \
  -d '{
    "writes": {
      "tuple_keys": [{
        "user": "bucket:openfga-protected",
        "relation": "parent",
        "object": "file:openfga-protected/newfile.txt"
      }]
    }
  }'
```

### Modifying OPA Policy

Edit `opa/policy.rego` and restart:
```bash
docker compose restart opa
```
