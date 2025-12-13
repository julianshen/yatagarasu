# Yatagarasu Kustomize Production Example

A production-ready Kustomize deployment that uses the `overlays/prod` base with site-specific customizations.

## Overview

This example demonstrates:
- Using the production overlay as a Kustomize base
- Adding site-specific patches and configuration
- Managing secrets from environment variables
- Custom image registry configuration
- Multi-bucket setup with different auth policies

## Prerequisites

- Kubernetes cluster (1.19+)
- kubectl with kustomize support (1.14+)
- Access to S3 buckets or S3-compatible storage
- JWT secret for authenticated buckets

## Directory Structure

```
kustomize-prod/
├── kustomization.yaml    # Main kustomization referencing prod overlay
├── config.yaml           # Site-specific proxy configuration
├── namespace.yaml        # Namespace definition
└── README.md            # This file
```

## Deployment Steps

### 1. Create Namespace

```bash
kubectl apply -f examples/kubernetes/kustomize-prod/namespace.yaml
```

### 2. Create Secrets from Environment Variables

```bash
# Set your credentials as environment variables
export AWS_ACCESS_KEY_ASSETS="your-assets-access-key"
export AWS_SECRET_KEY_ASSETS="your-assets-secret-key"
export AWS_ACCESS_KEY_MEDIA="your-media-access-key"
export AWS_SECRET_KEY_MEDIA="your-media-secret-key"
export AWS_ACCESS_KEY_PRIVATE="your-private-access-key"
export AWS_SECRET_KEY_PRIVATE="your-private-secret-key"
export JWT_SECRET="your-jwt-signing-secret"

# Create the secrets
kubectl create secret generic yatagarasu-s3-credentials \
  -n yatagarasu-prod \
  --from-literal=AWS_ACCESS_KEY_ASSETS="$AWS_ACCESS_KEY_ASSETS" \
  --from-literal=AWS_SECRET_KEY_ASSETS="$AWS_SECRET_KEY_ASSETS" \
  --from-literal=AWS_ACCESS_KEY_MEDIA="$AWS_ACCESS_KEY_MEDIA" \
  --from-literal=AWS_SECRET_KEY_MEDIA="$AWS_SECRET_KEY_MEDIA" \
  --from-literal=AWS_ACCESS_KEY_PRIVATE="$AWS_ACCESS_KEY_PRIVATE" \
  --from-literal=AWS_SECRET_KEY_PRIVATE="$AWS_SECRET_KEY_PRIVATE"

kubectl create secret generic yatagarasu-jwt-secret \
  -n yatagarasu-prod \
  --from-literal=JWT_SECRET="$JWT_SECRET"
```

### 3. Preview the Deployment

```bash
# See what will be deployed
kubectl kustomize examples/kubernetes/kustomize-prod/

# Or with kustomize directly
kustomize build examples/kubernetes/kustomize-prod/
```

### 4. Deploy

```bash
kubectl apply -k examples/kubernetes/kustomize-prod/
```

### 5. Verify Deployment

```bash
# Check pods
kubectl get pods -n yatagarasu-prod

# Check all resources
kubectl get all -n yatagarasu-prod

# Check pod disruption budget
kubectl get pdb -n yatagarasu-prod

# View logs
kubectl logs -n yatagarasu-prod -l app.kubernetes.io/name=yatagarasu --tail=100

# Check deployment status
kubectl rollout status deployment/yatagarasu -n yatagarasu-prod
```

### 6. Test the Proxy

```bash
# Port-forward to test locally
kubectl port-forward -n yatagarasu-prod svc/yatagarasu 8080:8080 &

# Test public assets (no auth required)
curl http://localhost:8080/assets/test.txt

# Test authenticated media endpoint
TOKEN=$(jwt encode --secret "$JWT_SECRET" '{"sub":"user1","aud":"media-service"}')
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/media/video.mp4

# Test private endpoint (requires admin role)
ADMIN_TOKEN=$(jwt encode --secret "$JWT_SECRET" '{"sub":"admin","role":"admin"}')
curl -H "Authorization: Bearer $ADMIN_TOKEN" http://localhost:8080/private/secrets.json

# Check health
curl http://localhost:8080/health

# Check metrics
kubectl port-forward -n yatagarasu-prod svc/yatagarasu 9090:9090 &
curl http://localhost:9090/metrics
```

## Customization

### Change Replica Count

Edit `kustomization.yaml`:
```yaml
patches:
  - target:
      kind: Deployment
      name: yatagarasu
    patch: |-
      - op: replace
        path: /spec/replicas
        value: 10  # Change to desired count
```

### Use Different Image Registry

Edit `kustomization.yaml`:
```yaml
images:
  - name: ghcr.io/julianshen/yatagarasu
    newName: your-registry.example.com/yatagarasu
    newTag: "1.3.0"
```

### Add Custom Environment Variables

Edit `kustomization.yaml`:
```yaml
patches:
  - target:
      kind: Deployment
      name: yatagarasu
    patch: |-
      - op: add
        path: /spec/template/spec/containers/0/env/-
        value:
          name: CUSTOM_VAR
          value: custom-value
```

### Configure Additional Buckets

Edit `config.yaml` to add more bucket configurations following the existing pattern.

## Secret Rotation

To rotate secrets without downtime:

```bash
# Create new secret version
kubectl create secret generic yatagarasu-s3-credentials-v2 \
  -n yatagarasu-prod \
  --from-literal=AWS_ACCESS_KEY_ASSETS="$NEW_ACCESS_KEY" \
  ...

# Update deployment to use new secret
kubectl set env deployment/yatagarasu \
  -n yatagarasu-prod \
  --from=secret/yatagarasu-s3-credentials-v2

# Delete old secret after verification
kubectl delete secret yatagarasu-s3-credentials -n yatagarasu-prod
```

## Monitoring

### Prometheus Scrape Config

The deployment includes annotations for Prometheus auto-discovery:
- `prometheus.io/scrape: "true"`
- `prometheus.io/port: "9090"`

### Key Metrics

| Metric | Description |
|--------|-------------|
| `yatagarasu_requests_total` | Total requests by bucket and status |
| `yatagarasu_request_duration_seconds` | Request latency histogram |
| `yatagarasu_cache_hits_total` | Cache hit count |
| `yatagarasu_cache_misses_total` | Cache miss count |
| `yatagarasu_s3_requests_total` | S3 backend requests |

## Troubleshooting

### Pods not starting

```bash
# Check events
kubectl describe pod -n yatagarasu-prod -l app.kubernetes.io/name=yatagarasu

# Check if secrets exist
kubectl get secrets -n yatagarasu-prod
```

### Authentication failures

```bash
# Check JWT secret is correctly set
kubectl get secret yatagarasu-jwt-secret -n yatagarasu-prod -o yaml

# Test token locally
jwt decode "$TOKEN"
```

### S3 connection issues

```bash
# Check S3 credentials
kubectl exec -n yatagarasu-prod deploy/yatagarasu -- env | grep AWS

# Test S3 connectivity from pod
kubectl exec -n yatagarasu-prod deploy/yatagarasu -- \
  curl -I https://s3.us-west-2.amazonaws.com
```

## Cleanup

```bash
# Delete all resources
kubectl delete -k examples/kubernetes/kustomize-prod/

# Delete secrets
kubectl delete secret yatagarasu-s3-credentials -n yatagarasu-prod
kubectl delete secret yatagarasu-jwt-secret -n yatagarasu-prod

# Delete namespace
kubectl delete namespace yatagarasu-prod
```

## Related Examples

- [helm-basic](../helm-basic/) - Simple Helm deployment
- [full-stack](../full-stack/) - Complete stack with OPA, Redis, and OpenFGA
