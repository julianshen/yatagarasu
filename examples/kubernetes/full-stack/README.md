# Yatagarasu Full Stack Kubernetes Example

A production-ready, security-hardened deployment with all components: Yatagarasu S3 proxy, Redis caching, OPA policy engine, OpenFGA fine-grained authorization, and PostgreSQL.

## Architecture

```
                    ┌─────────────────────────────────────────────────┐
                    │                  Kubernetes                     │
                    │                                                 │
Internet ──────────►│  ┌─────────┐     ┌──────────────┐              │
                    │  │ Ingress │────►│  Yatagarasu  │──────────────┼───► S3/MinIO
                    │  │ (NGINX) │     │   (3 pods)   │              │
                    │  └─────────┘     └──────┬───────┘              │
                    │                         │                       │
                    │         ┌───────────────┼───────────────┐       │
                    │         │               │               │       │
                    │         ▼               ▼               ▼       │
                    │    ┌─────────┐     ┌─────────┐     ┌─────────┐ │
                    │    │  Redis  │     │   OPA   │     │ OpenFGA │ │
                    │    │ (cache) │     │ (policy)│     │ (authz) │ │
                    │    └─────────┘     └─────────┘     └────┬────┘ │
                    │                                         │       │
                    │                                         ▼       │
                    │                                   ┌──────────┐  │
                    │                                   │ Postgres │  │
                    │                                   │   (db)   │  │
                    │                                   └──────────┘  │
                    │                                                 │
                    │  ────────────── NetworkPolicy ──────────────── │
                    │  (Zero-trust: explicit allow only)              │
                    └─────────────────────────────────────────────────┘
```

## Components

| Component | Purpose | Replicas |
|-----------|---------|----------|
| **Yatagarasu** | S3 proxy with JWT auth | 3 |
| **Redis** | Response caching (LRU) | 1 |
| **OPA** | Policy-based authorization | 2 |
| **OpenFGA** | Fine-grained access control | 2 |
| **PostgreSQL** | OpenFGA storage | 1 |
| **Ingress** | TLS termination, routing | - |

## Prerequisites

- Kubernetes cluster (1.19+)
- kubectl with kustomize support
- NGINX Ingress Controller (or compatible)
- Storage class for PostgreSQL PVC
- TLS certificate for ingress (or cert-manager)

## Directory Structure

```
full-stack/
├── kustomization.yaml          # Main kustomization
├── namespace.yaml              # Namespace definition
├── serviceaccount.yaml         # Service account
├── config.yaml                 # Yatagarasu configuration
├── policy.rego                 # OPA authorization policy
├── yatagarasu-deployment.yaml  # Proxy deployment
├── yatagarasu-service.yaml     # Proxy service
├── yatagarasu-configmap.yaml   # Config placeholder
├── redis-deployment.yaml       # Cache deployment
├── redis-service.yaml          # Cache service
├── opa-deployment.yaml         # OPA deployment
├── opa-service.yaml            # OPA service
├── opa-configmap.yaml          # Policy placeholder
├── openfga-deployment.yaml     # OpenFGA deployment
├── openfga-service.yaml        # OpenFGA service
├── postgres-deployment.yaml    # Database deployment
├── postgres-service.yaml       # Database service
├── postgres-pvc.yaml           # Database storage
├── ingress.yaml                # External access
├── network-policy.yaml         # Zero-trust networking
└── README.md                   # This file
```

## Deployment

### 1. Prepare Secrets

Edit `kustomization.yaml` to set production secrets:

```yaml
secretGenerator:
  - name: yatagarasu-secrets
    literals:
      - JWT_SECRET=your-production-jwt-secret
      - AWS_ACCESS_KEY=your-aws-access-key
      - AWS_SECRET_KEY=your-aws-secret-key

  - name: postgres-credentials
    literals:
      - POSTGRES_USER=openfga
      - POSTGRES_PASSWORD=secure-password-here
      - POSTGRES_DB=openfga
      - POSTGRES_URI=postgres://openfga:secure-password-here@postgres:5432/openfga?sslmode=disable
```

### 2. Configure TLS Certificate

Option A: Using cert-manager (recommended):
```yaml
# Add to ingress.yaml annotations
cert-manager.io/cluster-issuer: letsencrypt-prod
```

Option B: Create secret manually:
```bash
kubectl create secret tls yatagarasu-tls \
  -n yatagarasu-full \
  --cert=path/to/tls.crt \
  --key=path/to/tls.key
```

### 3. Update Domain

Edit `ingress.yaml` to set your domain:
```yaml
spec:
  tls:
    - hosts:
        - your-domain.example.com
      secretName: yatagarasu-tls
  rules:
    - host: your-domain.example.com
```

### 4. Preview Deployment

```bash
# View all resources that will be created
kubectl kustomize examples/kubernetes/full-stack/
```

### 5. Deploy

```bash
# Apply all resources
kubectl apply -k examples/kubernetes/full-stack/
```

### 6. Verify Deployment

```bash
# Check all pods
kubectl get pods -n yatagarasu-full -w

# Expected output (after ~1-2 minutes):
# NAME                         READY   STATUS    RESTARTS   AGE
# openfga-xxx                  1/1     Running   0          60s
# opa-xxx                      1/1     Running   0          60s
# postgres-xxx                 1/1     Running   0          60s
# redis-xxx                    1/1     Running   0          60s
# yatagarasu-xxx               1/1     Running   0          60s

# Check services
kubectl get svc -n yatagarasu-full

# Check ingress
kubectl get ingress -n yatagarasu-full

# View logs
kubectl logs -n yatagarasu-full -l app.kubernetes.io/name=yatagarasu -f
```

## Testing

### Local Testing (Port Forward)

```bash
# Forward proxy port
kubectl port-forward -n yatagarasu-full svc/yatagarasu 8080:8080 &

# Health check
curl http://localhost:8080/health
# {"status":"healthy"}

# Test public endpoint (no auth)
curl http://localhost:8080/public/test.txt

# Test authenticated endpoint
TOKEN=$(jwt encode --secret 'your-jwt-secret' '{"sub":"user1","role":"viewer"}')
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/users/file.txt
```

### Test via Ingress

```bash
# Test HTTPS access
curl https://your-domain.example.com/health

# Test with authentication
curl -H "Authorization: Bearer $TOKEN" https://your-domain.example.com/users/file.txt
```

## Security Features

### Network Policies

The deployment includes comprehensive NetworkPolicies:

| Policy | Description |
|--------|-------------|
| `default-deny-ingress` | Denies all ingress by default |
| `yatagarasu-ingress` | Allows ingress from Ingress controller only |
| `redis-ingress` | Allows connections only from Yatagarasu |
| `opa-ingress` | Allows connections only from Yatagarasu |
| `openfga-ingress` | Allows connections only from Yatagarasu |
| `postgres-ingress` | Allows connections only from OpenFGA |

### Security Contexts

All containers run with:
- `runAsNonRoot: true`
- `readOnlyRootFilesystem: true`
- `allowPrivilegeEscalation: false`
- Dropped ALL capabilities

### Pod Anti-Affinity

Yatagarasu pods are scheduled on different nodes when possible for high availability.

## Authorization

### OPA Policy

The included `policy.rego` implements role-based access:

| Role | Permissions |
|------|-------------|
| `admin` | All paths |
| `editor` | All paths except `/admin` |
| `viewer` | Only `/public` paths |

Denied patterns:
- Paths containing `/sensitive/`
- Files ending with `.env`
- Hidden files (starting with `.`)

### OpenFGA Integration

For fine-grained access control, configure OpenFGA:

```bash
# Port forward to OpenFGA
kubectl port-forward -n yatagarasu-full svc/openfga 8080:8080 &

# Create store
curl -X POST http://localhost:8080/stores \
  -H "Content-Type: application/json" \
  -d '{"name": "yatagarasu"}'

# Note the store_id and update config.yaml
```

## Monitoring

### Prometheus Metrics

Metrics are exposed on port 9090:

```bash
kubectl port-forward -n yatagarasu-full svc/yatagarasu 9090:9090 &
curl http://localhost:9090/metrics
```

Key metrics:
- `yatagarasu_requests_total` - Request count by bucket/status
- `yatagarasu_request_duration_seconds` - Latency histogram
- `yatagarasu_cache_hits_total` / `_misses_total` - Cache efficiency
- `yatagarasu_s3_requests_total` - Backend requests

### Health Endpoints

| Endpoint | Port | Description |
|----------|------|-------------|
| `/health` | 8080 | Liveness probe |
| `/ready` | 8080 | Readiness probe |
| `/metrics` | 9090 | Prometheus metrics |

## Customization

### Scaling

```bash
# Scale Yatagarasu
kubectl scale deployment yatagarasu -n yatagarasu-full --replicas=5

# Scale OPA
kubectl scale deployment opa -n yatagarasu-full --replicas=3
```

### Custom OPA Policy

Edit `policy.rego` and reapply:
```bash
kubectl apply -k examples/kubernetes/full-stack/
```

### Add Buckets

Edit `config.yaml` to add bucket configurations, then:
```bash
kubectl apply -k examples/kubernetes/full-stack/
kubectl rollout restart deployment/yatagarasu -n yatagarasu-full
```

## Troubleshooting

### Pods not starting

```bash
# Check events
kubectl describe pod -n yatagarasu-full <pod-name>

# Check if PVC is bound
kubectl get pvc -n yatagarasu-full
```

### Network connectivity issues

```bash
# Test from Yatagarasu pod
kubectl exec -n yatagarasu-full deploy/yatagarasu -- nc -zv redis 6379
kubectl exec -n yatagarasu-full deploy/yatagarasu -- nc -zv opa 8181
kubectl exec -n yatagarasu-full deploy/yatagarasu -- nc -zv openfga 8080
```

### Authorization failures

```bash
# Check OPA decision
kubectl exec -n yatagarasu-full deploy/opa -- \
  curl -X POST http://localhost:8181/v1/data/yatagarasu/authz/allow \
  -d '{"input":{"claims":{"role":"viewer"},"path":"/public/test.txt"}}'

# Check OPA logs
kubectl logs -n yatagarasu-full -l app.kubernetes.io/name=opa
```

### PostgreSQL issues

```bash
# Check PostgreSQL logs
kubectl logs -n yatagarasu-full -l app.kubernetes.io/name=postgres

# Connect to database
kubectl exec -it -n yatagarasu-full deploy/postgres -- psql -U openfga -d openfga
```

## Cleanup

```bash
# Delete all resources
kubectl delete -k examples/kubernetes/full-stack/

# Delete PVC (data will be lost)
kubectl delete pvc postgres-data -n yatagarasu-full

# Delete namespace
kubectl delete namespace yatagarasu-full
```

## Related Examples

- [helm-basic](../helm-basic/) - Simple Helm deployment for getting started
- [kustomize-prod](../kustomize-prod/) - Production Kustomize deployment
- [Docker Compose examples](../../docker-compose/) - Local development setup
