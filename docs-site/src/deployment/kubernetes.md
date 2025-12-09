# Kubernetes Deployment

## Helm (Recommended)

Yatagarasu provides a production-ready Helm chart.

```bash
helm install yatagarasu ./charts/yatagarasu \
  --set server.replicas=3 \
  --set cache.redis.enabled=true
```

## Kustomize

For GitOps workflows, use the Kustomize overlays.

```bash
# Apply the production overlay
kubectl apply -k kustomize/overlays/prod
```

This overlay includes:
- 3 Replicas
- PodDisruptionBudget
- Anti-Affinity rules
- Resource limits
