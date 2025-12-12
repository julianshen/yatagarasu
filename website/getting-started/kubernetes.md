---
title: Kubernetes Quickstart
layout: default
parent: Getting Started
nav_order: 4
---

# Kubernetes Quickstart

Deploy Yatagarasu on Kubernetes in minutes.
{: .fs-6 .fw-300 }

---

## Prerequisites

- Kubernetes cluster (1.21+)
- `kubectl` configured to access your cluster
- Helm 3.x installed (for Helm deployment)

---

## Option 1: Helm (Recommended)

### Install with Helm

```bash
# Add the Helm repository
helm repo add yatagarasu https://julianshen.github.io/yatagarasu/charts
helm repo update

# Install with default values
helm install yatagarasu yatagarasu/yatagarasu

# Or install with custom values
helm install yatagarasu yatagarasu/yatagarasu \
  --set replicaCount=3 \
  --set image.tag=1.2.0
```

### Custom Values File

Create a `values.yaml` for your deployment:

```yaml
# values.yaml
replicaCount: 3

image:
  repository: ghcr.io/julianshen/yatagarasu
  tag: "1.2.0"
  pullPolicy: IfNotPresent

service:
  type: ClusterIP
  port: 8080
  metricsPort: 9090

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: s3proxy.example.com
      paths:
        - path: /
          pathType: Prefix

resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
  limits:
    memory: "1Gi"
    cpu: "1000m"

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70

config:
  server:
    address: "0.0.0.0:8080"
    threads: 4

  buckets:
    - name: "assets"
      pathPrefix: "/assets"
      s3:
        bucket: "my-bucket"
        region: "us-east-1"
      auth:
        enabled: false

  cache:
    memory:
      maxCapacity: 536870912  # 512MB
      ttlSeconds: 3600

  metrics:
    enabled: true
    port: 9090

# S3 credentials from existing secret
existingSecret:
  name: s3-credentials
  accessKeyKey: access-key
  secretKeyKey: secret-key
```

Install with custom values:

```bash
helm install yatagarasu yatagarasu/yatagarasu -f values.yaml
```

---

## Option 2: Kustomize

### Base Configuration

Create the base directory structure:

```
kustomize/
├── base/
│   ├── kustomization.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   └── secret.yaml
└── overlays/
    ├── dev/
    │   └── kustomization.yaml
    └── prod/
        └── kustomization.yaml
```

#### base/kustomization.yaml

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - deployment.yaml
  - service.yaml
  - configmap.yaml
```

#### base/deployment.yaml

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: yatagarasu
  labels:
    app: yatagarasu
spec:
  replicas: 2
  selector:
    matchLabels:
      app: yatagarasu
  template:
    metadata:
      labels:
        app: yatagarasu
    spec:
      containers:
        - name: yatagarasu
          image: ghcr.io/julianshen/yatagarasu:1.2.0
          ports:
            - name: http
              containerPort: 8080
            - name: metrics
              containerPort: 9090
          env:
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: s3-credentials
                  key: access-key
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: s3-credentials
                  key: secret-key
          volumeMounts:
            - name: config
              mountPath: /etc/yatagarasu
              readOnly: true
          livenessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "1Gi"
              cpu: "1000m"
      volumes:
        - name: config
          configMap:
            name: yatagarasu-config
```

#### base/service.yaml

```yaml
apiVersion: v1
kind: Service
metadata:
  name: yatagarasu
  labels:
    app: yatagarasu
spec:
  type: ClusterIP
  ports:
    - name: http
      port: 8080
      targetPort: http
    - name: metrics
      port: 9090
      targetPort: metrics
  selector:
    app: yatagarasu
```

#### base/configmap.yaml

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: yatagarasu-config
data:
  config.yaml: |
    server:
      address: "0.0.0.0:8080"
      threads: 4

    buckets:
      - name: "assets"
        path_prefix: "/assets"
        s3:
          bucket: "my-bucket"
          region: "us-east-1"
          access_key: "${AWS_ACCESS_KEY_ID}"
          secret_key: "${AWS_SECRET_ACCESS_KEY}"
        auth:
          enabled: false

    cache:
      memory:
        max_capacity: 536870912
        ttl_seconds: 3600

    metrics:
      enabled: true
      port: 9090
```

### Production Overlay

#### overlays/prod/kustomization.yaml

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

namespace: production

resources:
  - ../../base
  - ingress.yaml
  - hpa.yaml
  - pdb.yaml

replicas:
  - name: yatagarasu
    count: 3

images:
  - name: ghcr.io/julianshen/yatagarasu
    newTag: "1.2.0"

patches:
  - patch: |-
      - op: replace
        path: /spec/template/spec/containers/0/resources/limits/memory
        value: "2Gi"
    target:
      kind: Deployment
      name: yatagarasu
```

#### overlays/prod/hpa.yaml

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: yatagarasu
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: yatagarasu
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

### Deploy with Kustomize

```bash
# Preview the generated manifests
kubectl kustomize overlays/prod

# Apply to cluster
kubectl apply -k overlays/prod

# Check deployment status
kubectl get pods -n production -l app=yatagarasu
```

---

## Option 3: Raw Manifests

For quick testing, apply manifests directly:

```bash
# Create namespace
kubectl create namespace yatagarasu

# Create secret for S3 credentials
kubectl create secret generic s3-credentials \
  --namespace yatagarasu \
  --from-literal=access-key=AKIAIOSFODNN7EXAMPLE \
  --from-literal=secret-key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# Apply ConfigMap
kubectl apply -f - <<EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: yatagarasu-config
  namespace: yatagarasu
data:
  config.yaml: |
    server:
      address: "0.0.0.0:8080"
    buckets:
      - name: "assets"
        path_prefix: "/assets"
        s3:
          bucket: "my-bucket"
          region: "us-east-1"
          access_key: "\${AWS_ACCESS_KEY_ID}"
          secret_key: "\${AWS_SECRET_ACCESS_KEY}"
        auth:
          enabled: false
    metrics:
      enabled: true
      port: 9090
EOF

# Apply Deployment
kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: yatagarasu
  namespace: yatagarasu
spec:
  replicas: 2
  selector:
    matchLabels:
      app: yatagarasu
  template:
    metadata:
      labels:
        app: yatagarasu
    spec:
      containers:
        - name: yatagarasu
          image: ghcr.io/julianshen/yatagarasu:1.2.0
          ports:
            - containerPort: 8080
            - containerPort: 9090
          env:
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: s3-credentials
                  key: access-key
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: s3-credentials
                  key: secret-key
          volumeMounts:
            - name: config
              mountPath: /etc/yatagarasu
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
      volumes:
        - name: config
          configMap:
            name: yatagarasu-config
EOF

# Apply Service
kubectl apply -f - <<EOF
apiVersion: v1
kind: Service
metadata:
  name: yatagarasu
  namespace: yatagarasu
spec:
  type: ClusterIP
  ports:
    - name: http
      port: 8080
    - name: metrics
      port: 9090
  selector:
    app: yatagarasu
EOF

# Check status
kubectl get all -n yatagarasu
```

---

## Verify Deployment

```bash
# Check pods are running
kubectl get pods -l app=yatagarasu

# Check service
kubectl get svc yatagarasu

# Port forward to test locally
kubectl port-forward svc/yatagarasu 8080:8080

# In another terminal, test the proxy
curl http://localhost:8080/health

# View logs
kubectl logs -l app=yatagarasu -f
```

---

## Exposing with Ingress

### NGINX Ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: yatagarasu
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "0"
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
spec:
  ingressClassName: nginx
  rules:
    - host: s3proxy.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: yatagarasu
                port:
                  number: 8080
```

### With TLS

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: yatagarasu
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - s3proxy.example.com
      secretName: yatagarasu-tls
  rules:
    - host: s3proxy.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: yatagarasu
                port:
                  number: 8080
```

---

## With Redis/Valkey for Distributed Cache

Deploy Valkey for shared caching across instances:

```yaml
# valkey-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: valkey
spec:
  replicas: 1
  selector:
    matchLabels:
      app: valkey
  template:
    metadata:
      labels:
        app: valkey
    spec:
      containers:
        - name: valkey
          image: valkey/valkey:7-alpine
          ports:
            - containerPort: 6379
          resources:
            requests:
              memory: "256Mi"
            limits:
              memory: "1Gi"
---
apiVersion: v1
kind: Service
metadata:
  name: valkey
spec:
  ports:
    - port: 6379
  selector:
    app: valkey
```

Update your Yatagarasu config:

```yaml
cache:
  memory:
    max_capacity: 268435456
    ttl_seconds: 3600
  redis:
    enabled: true
    url: "redis://valkey:6379"
    max_capacity: 536870912
    ttl_seconds: 7200
```

---

## ServiceMonitor for Prometheus

If using Prometheus Operator:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: yatagarasu
  labels:
    release: prometheus
spec:
  selector:
    matchLabels:
      app: yatagarasu
  endpoints:
    - port: metrics
      interval: 15s
      path: /metrics
```

---

## Next Steps

- [High Availability](/yatagarasu/deployment/high-availability/) - Configure HA with replicas
- [Configuration Reference](/yatagarasu/configuration/) - All configuration options
- [Operations Guide](/yatagarasu/operations/) - Monitoring and troubleshooting
