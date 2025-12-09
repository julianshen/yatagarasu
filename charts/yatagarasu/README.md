# Yatagarasu Helm Chart

A Helm chart for deploying Yatagarasu - a high-performance S3 proxy built with Rust and Cloudflare Pingora.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.0+
- S3-compatible storage backend (AWS S3, MinIO, etc.)

## Installation

### Add the Helm repository (if published)

```bash
helm repo add yatagarasu https://julianshen.github.io/yatagarasu
helm repo update
```

### Install from local chart

```bash
# Clone the repository
git clone https://github.com/julianshen/yatagarasu.git
cd yatagarasu

# Install with default values
helm install my-yatagarasu ./charts/yatagarasu

# Install with custom values
helm install my-yatagarasu ./charts/yatagarasu -f my-values.yaml
```

## Configuration

### Basic Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas | `1` |
| `image.repository` | Image repository | `ghcr.io/julianshen/yatagarasu` |
| `image.tag` | Image tag | `""` (uses appVersion) |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |

### Server Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `server.port` | Server HTTP port | `8080` |
| `metrics.enabled` | Enable metrics endpoint | `true` |
| `metrics.port` | Metrics port | `9090` |

### Bucket Configuration

Buckets are configured in the `buckets` array. Each bucket supports:

| Parameter | Description | Required |
|-----------|-------------|----------|
| `name` | Bucket identifier | Yes |
| `pathPrefix` | URL path prefix for routing | Yes |
| `s3.bucket` | S3 bucket name | Yes |
| `s3.region` | AWS region | Yes |
| `s3.endpoint` | Custom S3 endpoint (for MinIO, etc.) | No |
| `s3.accessKey` | S3 access key (stored in Secret) | No* |
| `s3.secretKey` | S3 secret key (stored in Secret) | No* |
| `s3.existingSecret` | Use existing secret for credentials | No* |
| `s3.accessKeySecretKey` | Key in existing secret for access key | No |
| `s3.secretKeySecretKey` | Key in existing secret for secret key | No |
| `auth.enabled` | Enable authentication for this bucket | No |
| `auth.jwt.secret` | JWT secret for validation | No |
| `auth.jwt.existingSecret` | Use existing secret for JWT | No |
| `auth.jwt.secretKeySecretKey` | Key in existing secret for JWT secret | No |

*Either provide `accessKey`/`secretKey` or use `existingSecret`

### Cache Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `cache.enabled` | Enable caching | `true` |
| `cache.memory.maxCapacity` | Max memory cache size (bytes) | `104857600` (100MB) |
| `cache.memory.ttlSeconds` | Cache TTL in seconds | `300` |
| `cache.redis.enabled` | Enable Redis cache tier | `false` |
| `cache.redis.url` | Redis connection URL | `redis://localhost:6379` |

### Authorization Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `authorization.opa.enabled` | Enable OPA authorization | `false` |
| `authorization.opa.url` | OPA endpoint URL | `""` |
| `authorization.openFga.enabled` | Enable OpenFGA authorization | `false` |
| `authorization.openFga.url` | OpenFGA endpoint URL | `""` |

### Service Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `service.type` | Kubernetes service type | `ClusterIP` |
| `service.port` | Service port | `8080` |

### Ingress Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `ingress.enabled` | Enable ingress | `false` |
| `ingress.className` | Ingress class name | `""` |
| `ingress.annotations` | Ingress annotations | `{}` |
| `ingress.hosts` | Ingress host configuration | See values.yaml |
| `ingress.tls` | Ingress TLS configuration | `[]` |

### Autoscaling Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `autoscaling.enabled` | Enable HPA | `false` |
| `autoscaling.minReplicas` | Minimum replicas | `1` |
| `autoscaling.maxReplicas` | Maximum replicas | `10` |
| `autoscaling.targetCPUUtilizationPercentage` | Target CPU utilization | `80` |

### Pod Disruption Budget

| Parameter | Description | Default |
|-----------|-------------|---------|
| `podDisruptionBudget.enabled` | Enable PDB | `false` |
| `podDisruptionBudget.minAvailable` | Minimum available pods | `1` |

### Service Monitor (Prometheus)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `serviceMonitor.enabled` | Enable ServiceMonitor | `false` |
| `serviceMonitor.interval` | Scrape interval | `30s` |
| `serviceMonitor.scrapeTimeout` | Scrape timeout | `10s` |

## Examples

### Minimal Installation

```yaml
# minimal-values.yaml
buckets:
  - name: public
    path_prefix: /public
    s3:
      bucket: my-public-bucket
      region: us-east-1
      existingSecret: my-s3-credentials
```

### Production Setup with HA

```yaml
# production-values.yaml
replicaCount: 3

buckets:
  - name: assets
    path_prefix: /assets
    s3:
      bucket: production-assets
      region: us-west-2
      existingSecret: assets-s3-credentials
    auth:
      enabled: true
      jwt:
        existingSecret: jwt-secret

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70

podDisruptionBudget:
  enabled: true
  minAvailable: 2

cache:
  enabled: true
  memory:
    maxCapacity: 536870912  # 512MB
    ttlSeconds: 600
  redis:
    enabled: true
    url: redis://redis-master:6379

resources:
  limits:
    cpu: 1000m
    memory: 512Mi
  requests:
    cpu: 250m
    memory: 256Mi

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: assets.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: assets-tls
      hosts:
        - assets.example.com

serviceMonitor:
  enabled: true
  interval: 15s
```

### Using External Secrets

Create secrets before installing:

```bash
# Create S3 credentials secret
kubectl create secret generic my-s3-credentials \
  --from-literal=access-key=AKIAIOSFODNN7EXAMPLE \
  --from-literal=secret-key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# Create JWT secret
kubectl create secret generic jwt-secret \
  --from-literal=jwt-secret=your-super-secret-key
```

Then reference them in values:

```yaml
buckets:
  - name: private
    path_prefix: /private
    s3:
      bucket: private-bucket
      region: us-east-1
      existingSecret: my-s3-credentials
      accessKeySecretKey: access-key  # key name in secret
      secretKeySecretKey: secret-key  # key name in secret
    auth:
      enabled: true
      jwt:
        existingSecret: jwt-secret
        secretKeySecretKey: jwt-secret  # key name in secret
```

## Upgrading

```bash
helm upgrade my-yatagarasu ./charts/yatagarasu -f my-values.yaml
```

## Uninstalling

```bash
helm uninstall my-yatagarasu
```

## Development

### Testing the chart

```bash
# Lint the chart
helm lint ./charts/yatagarasu

# Render templates locally
helm template test ./charts/yatagarasu -f values.yaml

# Dry run installation
helm install test ./charts/yatagarasu --dry-run --debug
```

## License

Apache-2.0
