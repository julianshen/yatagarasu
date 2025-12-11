---
title: Kubernetes Deployment
layout: default
parent: Deployment
nav_order: 2
---

# Kubernetes Deployment

Deploy Yatagarasu on Kubernetes for production scale.
{: .fs-6 .fw-300 }

---

## Deployment Methods

| Method | Best For |
|:-------|:---------|
| Helm | Standard deployments, managed values |
| Kustomize | GitOps workflows, environment overlays |
| Raw Manifests | Quick testing, custom requirements |

---

## Helm Deployment

### Install Chart

```bash
# Add repository
helm repo add yatagarasu https://julianshen.github.io/yatagarasu/charts
helm repo update

# Install with defaults
helm install yatagarasu yatagarasu/yatagarasu -n yatagarasu --create-namespace

# Install with custom values
helm install yatagarasu yatagarasu/yatagarasu \
  -n yatagarasu --create-namespace \
  -f values.yaml
```

### values.yaml

```yaml
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
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "0"
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: s3proxy.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: yatagarasu-tls
      hosts:
        - s3proxy.example.com

resources:
  requests:
    cpu: 250m
    memory: 256Mi
  limits:
    cpu: 1000m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

podDisruptionBudget:
  enabled: true
  minAvailable: 1

config:
  server:
    address: "0.0.0.0:8080"
    threads: 4

  buckets:
    - name: "assets"
      pathPrefix: "/assets"
      s3:
        bucket: "production-assets"
        region: "us-east-1"
      auth:
        enabled: false

  cache:
    memory:
      maxCapacity: 536870912
      ttlSeconds: 3600
    redis:
      enabled: true
      url: "redis://redis:6379"

  metrics:
    enabled: true
    port: 9090

existingSecret:
  name: yatagarasu-credentials
  keys:
    awsAccessKey: aws-access-key
    awsSecretKey: aws-secret-key

serviceMonitor:
  enabled: true
  interval: 15s
```

### Upgrade

```bash
helm upgrade yatagarasu yatagarasu/yatagarasu \
  -n yatagarasu \
  -f values.yaml
```

---

## Kustomize Deployment

### Directory Structure

```
kustomize/
├── base/
│   ├── kustomization.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   └── hpa.yaml
└── overlays/
    ├── dev/
    │   ├── kustomization.yaml
    │   └── patches/
    ├── staging/
    │   ├── kustomization.yaml
    │   └── patches/
    └── prod/
        ├── kustomization.yaml
        ├── patches/
        └── resources/
```

### base/kustomization.yaml

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - deployment.yaml
  - service.yaml
  - configmap.yaml
  - hpa.yaml

commonLabels:
  app: yatagarasu
  app.kubernetes.io/name: yatagarasu
```

### base/deployment.yaml

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: yatagarasu
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
      securityContext:
        runAsNonRoot: true
        runAsUser: 65532
        fsGroup: 65532
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
            periodSeconds: 5
          resources:
            requests:
              cpu: 250m
              memory: 256Mi
            limits:
              cpu: 1000m
              memory: 1Gi
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop:
                - ALL
      volumes:
        - name: config
          configMap:
            name: yatagarasu-config
```

### overlays/prod/kustomization.yaml

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

namespace: production

resources:
  - ../../base
  - ingress.yaml
  - pdb.yaml
  - networkpolicy.yaml

replicas:
  - name: yatagarasu
    count: 5

images:
  - name: ghcr.io/julianshen/yatagarasu
    newTag: "1.2.0"

patches:
  - path: patches/resources.yaml
  - path: patches/env.yaml
```

### Deploy

```bash
# Preview
kubectl kustomize overlays/prod

# Apply
kubectl apply -k overlays/prod

# With dry-run
kubectl apply -k overlays/prod --dry-run=client
```

---

## Essential Resources

### Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: s3-credentials
type: Opaque
stringData:
  access-key: "AKIAIOSFODNN7EXAMPLE"
  secret-key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
```

### ConfigMap

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
          bucket: "production-assets"
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

### Ingress (NGINX)

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: yatagarasu
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "0"
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-request-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
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

### HorizontalPodAutoscaler

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
  minReplicas: 2
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
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Percent
          value: 10
          periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
        - type: Percent
          value: 100
          periodSeconds: 15
```

### PodDisruptionBudget

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: yatagarasu
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app: yatagarasu
```

### NetworkPolicy

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: yatagarasu
spec:
  podSelector:
    matchLabels:
      app: yatagarasu
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: monitoring
      ports:
        - protocol: TCP
          port: 9090
  egress:
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
      ports:
        - protocol: TCP
          port: 443  # S3
        - protocol: TCP
          port: 6379 # Redis
```

---

## ServiceMonitor (Prometheus Operator)

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
  namespaceSelector:
    matchNames:
      - production
```

---

## Rolling Updates

```yaml
spec:
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 25%
      maxUnavailable: 0
```

Trigger update:

```bash
# Update image
kubectl set image deployment/yatagarasu \
  yatagarasu=ghcr.io/julianshen/yatagarasu:1.2.1

# Rollback
kubectl rollout undo deployment/yatagarasu

# Check status
kubectl rollout status deployment/yatagarasu
```

---

## Operations

### Hot Reload

```bash
# Trigger config reload
kubectl exec deployment/yatagarasu -- kill -HUP 1

# Or restart pods
kubectl rollout restart deployment/yatagarasu
```

### Debugging

```bash
# Check pods
kubectl get pods -l app=yatagarasu

# View logs
kubectl logs -l app=yatagarasu -f

# Describe pod
kubectl describe pod yatagarasu-xxx

# Port forward
kubectl port-forward svc/yatagarasu 8080:8080
```

---

## See Also

- [High Availability](/yatagarasu/deployment/high-availability/)
- [Operations Guide](/yatagarasu/operations/)
- [Kubernetes Quickstart](/yatagarasu/getting-started/kubernetes/)
