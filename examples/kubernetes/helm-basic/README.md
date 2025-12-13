# Yatagarasu Helm Basic Example

A minimal Helm deployment example for getting started with Yatagarasu on Kubernetes.

## Prerequisites

- Kubernetes cluster (1.19+)
- Helm 3.x installed
- kubectl configured
- S3-compatible storage (AWS S3 or MinIO)

## Quick Start

### 1. Create Namespace

```bash
kubectl create namespace yatagarasu
```

### 2. Deploy MinIO (Optional - for testing)

If you don't have S3 storage, deploy MinIO for testing:

```bash
# Deploy MinIO
kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: minio
  namespace: yatagarasu
spec:
  selector:
    matchLabels:
      app: minio
  template:
    metadata:
      labels:
        app: minio
    spec:
      containers:
      - name: minio
        image: minio/minio:latest
        args: ["server", "/data", "--console-address", ":9001"]
        env:
        - name: MINIO_ROOT_USER
          value: minioadmin
        - name: MINIO_ROOT_PASSWORD
          value: minioadmin
        ports:
        - containerPort: 9000
        - containerPort: 9001
---
apiVersion: v1
kind: Service
metadata:
  name: minio
  namespace: yatagarasu
spec:
  selector:
    app: minio
  ports:
  - name: api
    port: 9000
  - name: console
    port: 9001
EOF
```

### 3. Install Yatagarasu

```bash
# From the repository root
helm install yatagarasu charts/yatagarasu \
  -n yatagarasu \
  -f examples/kubernetes/helm-basic/values.yaml
```

Or with custom values:

```bash
helm install yatagarasu charts/yatagarasu \
  -n yatagarasu \
  --set buckets[0].s3Bucket=my-bucket \
  --set buckets[0].s3Region=us-west-2 \
  --set buckets[0].s3Endpoint=https://s3.us-west-2.amazonaws.com \
  --set buckets[0].s3AccessKey=$AWS_ACCESS_KEY \
  --set buckets[0].s3SecretKey=$AWS_SECRET_KEY
```

### 4. Verify Deployment

```bash
# Check pod status
kubectl get pods -n yatagarasu

# Check service
kubectl get svc -n yatagarasu

# View logs
kubectl logs -n yatagarasu -l app.kubernetes.io/name=yatagarasu
```

### 5. Test the Proxy

```bash
# Port-forward to access locally
kubectl port-forward -n yatagarasu svc/yatagarasu 8080:8080 &

# Create a test bucket and file (if using MinIO)
kubectl port-forward -n yatagarasu svc/minio 9000:9000 &
mc alias set local http://localhost:9000 minioadmin minioadmin
mc mb local/my-bucket
echo "Hello, Yatagarasu!" | mc pipe local/my-bucket/hello.txt

# Test proxy access
curl http://localhost:8080/demo/hello.txt
# Expected output: Hello, Yatagarasu!

# Check health endpoint
curl http://localhost:8080/health
# Expected output: {"status":"healthy"}

# Check readiness endpoint
curl http://localhost:8080/ready
# Expected output: {"status":"ready"}
```

### 6. Check Metrics (Optional)

```bash
# Port-forward metrics port
kubectl port-forward -n yatagarasu svc/yatagarasu 9090:9090 &

# View Prometheus metrics
curl http://localhost:9090/metrics
```

## Configuration Options

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas | `1` |
| `image.tag` | Image version | `1.3.0` |
| `buckets[].name` | Bucket config name | `demo` |
| `buckets[].pathPrefix` | URL path prefix | `/demo` |
| `buckets[].s3Bucket` | S3 bucket name | `my-bucket` |
| `buckets[].s3Region` | S3 region | `us-east-1` |
| `buckets[].s3Endpoint` | S3 endpoint URL | `http://minio:9000` |
| `buckets[].authEnabled` | Enable JWT auth | `false` |
| `resources.limits.cpu` | CPU limit | `500m` |
| `resources.limits.memory` | Memory limit | `256Mi` |

See [charts/yatagarasu/values.yaml](../../../charts/yatagarasu/values.yaml) for all options.

## Upgrading

```bash
# Update values and upgrade
helm upgrade yatagarasu charts/yatagarasu \
  -n yatagarasu \
  -f examples/kubernetes/helm-basic/values.yaml
```

## Cleanup

```bash
# Uninstall Yatagarasu
helm uninstall yatagarasu -n yatagarasu

# Delete MinIO (if deployed)
kubectl delete deployment minio -n yatagarasu
kubectl delete svc minio -n yatagarasu

# Delete namespace
kubectl delete namespace yatagarasu
```

## Next Steps

- Enable caching with Redis: See [ha-redis overlay](../../../kustomize/overlays/ha-redis/)
- Add authentication: Set `buckets[].authEnabled=true` and configure JWT
- Add authorization: See [full-stack example](../full-stack/)
- Production deployment: See [kustomize-prod example](../kustomize-prod/)
