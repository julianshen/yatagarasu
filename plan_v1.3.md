# Yatagarasu v1.3.0 Implementation Plan

**Version**: 1.3.0
**Focus**: Deployment & Documentation
**Target**: Q1 2026

---

## Overview

This plan implements four main areas:
1. **Cache Warming API** - Pre-fetch frequently accessed objects
2. **Kubernetes Deployment** - Helm chart + Kustomize
3. **Deployment Examples** - Simple to complex scenarios
4. **Documentation Website** - GitHub Pages with tutorials

---

## Phase 1: Cache Warming API

### 1.1 Admin API Foundation
- [ ] Test: Admin API router exists at `/admin/*`
- [ ] Test: Admin endpoints require authentication
- [ ] Test: Admin JWT claims verification (role: admin)
- [ ] Test: Returns 401 for missing/invalid admin token
- [ ] Test: Returns 403 for valid JWT without admin role

### 1.2 S3 LIST Operation
- [ ] Test: S3 client supports ListObjectsV2 operation
- [ ] Test: LIST returns object keys with metadata (size, etag)
- [ ] Test: LIST supports prefix filtering
- [ ] Test: LIST supports pagination (continuation token)
- [ ] Test: LIST respects max_keys limit
- [ ] Test: Recursive LIST traverses common prefixes

### 1.3 Cache Warm Task Management
- [ ] Test: Can create prewarm task with path and options
- [ ] Test: Task ID is unique UUID
- [ ] Test: Task tracks status (pending, running, completed, failed, cancelled)
- [ ] Test: Task tracks progress (files_scanned, files_cached, bytes_cached)
- [ ] Test: Task stores start_time, end_time, duration
- [ ] Test: Multiple tasks can run concurrently

### 1.4 Prewarm API Endpoints
- [ ] Test: POST /admin/cache/prewarm creates new task
- [ ] Test: POST validates required fields (bucket, path)
- [ ] Test: POST accepts options (recursive, max_depth, max_files)
- [ ] Test: POST returns task_id and initial status
- [ ] Test: GET /admin/cache/prewarm/status/{task_id} returns progress
- [ ] Test: GET returns 404 for unknown task_id
- [ ] Test: DELETE /admin/cache/prewarm/{task_id} cancels task
- [ ] Test: DELETE sets task status to cancelled
- [ ] Test: GET /admin/cache/prewarm/tasks lists all tasks

### 1.5 Prewarm Worker
- [ ] Test: Worker fetches objects from S3 and caches them
- [ ] Test: Worker skips files exceeding max_item_size
- [ ] Test: Worker respects concurrency limit
- [ ] Test: Worker respects rate_limit (files/second)
- [ ] Test: Worker updates task progress periodically
- [ ] Test: Worker handles S3 errors gracefully
- [ ] Test: Worker stops when task is cancelled
- [ ] Test: Worker applies include_patterns filter
- [ ] Test: Worker applies exclude_patterns filter

### 1.6 Prewarm Metrics
- [ ] Test: yatagarasu_prewarm_tasks_total counter exists
- [ ] Test: yatagarasu_prewarm_files_total counter exists
- [ ] Test: yatagarasu_prewarm_bytes_total counter exists
- [ ] Test: yatagarasu_prewarm_duration_seconds histogram exists
- [ ] Test: yatagarasu_prewarm_errors_total counter exists

### 1.7 Configuration
- [ ] Test: Config supports cache.prewarm section
- [ ] Test: Config validates prewarm settings
- [ ] Test: on_startup triggers prewarm at proxy start
- [ ] Test: Default concurrency is 10
- [ ] Test: Default rate_limit is 100/s

---

## Phase 2: Helm Chart

### 2.1 Chart Structure
- [ ] Test: Chart.yaml has correct apiVersion (v2)
- [ ] Test: Chart.yaml has name, version, appVersion
- [ ] Test: values.yaml has documented defaults
- [ ] Test: Chart validates with `helm lint`
- [ ] Test: Chart templates render without errors

### 2.2 Core Templates
- [ ] Test: Deployment template creates valid Deployment
- [ ] Test: Service template creates ClusterIP service
- [ ] Test: ConfigMap template includes config.yaml
- [ ] Test: Secret template handles credentials
- [ ] Test: ServiceAccount template created
- [ ] Test: RBAC templates (Role, RoleBinding) created

### 2.3 Configuration Values
- [ ] Test: image.repository defaults to ghcr.io/julianshen/yatagarasu
- [ ] Test: image.tag defaults to appVersion
- [ ] Test: replicaCount configurable (default: 1)
- [ ] Test: resources.limits/requests configurable
- [ ] Test: nodeSelector, tolerations, affinity supported
- [ ] Test: podAnnotations and podLabels supported

### 2.4 S3 Bucket Configuration
- [ ] Test: buckets[] array configures multiple buckets
- [ ] Test: Each bucket has name, pathPrefix, s3 settings
- [ ] Test: Credentials can reference existing secrets
- [ ] Test: Credentials can be created from values

### 2.5 Optional Components
- [ ] Test: ingress.enabled creates Ingress resource
- [ ] Test: Ingress supports className and annotations
- [ ] Test: Ingress supports TLS configuration
- [ ] Test: serviceMonitor.enabled creates ServiceMonitor
- [ ] Test: ServiceMonitor has correct endpoints
- [ ] Test: podDisruptionBudget.enabled creates PDB
- [ ] Test: horizontalPodAutoscaler.enabled creates HPA

### 2.6 Health and Probes
- [ ] Test: livenessProbe points to /health
- [ ] Test: readinessProbe points to /ready
- [ ] Test: Probe settings (initialDelaySeconds, etc.) configurable

### 2.7 Advanced Features
- [ ] Test: auth.jwt.* configures JWT authentication
- [ ] Test: cache.* configures caching layer
- [ ] Test: cache.redis.enabled adds Redis config
- [ ] Test: authorization.opa.enabled adds OPA config
- [ ] Test: authorization.openfga.enabled adds OpenFGA config
- [ ] Test: metrics.enabled configures metrics port

---

## Phase 3: Kustomize Base

### 3.1 Base Structure
- [ ] Test: base/kustomization.yaml exists
- [ ] Test: base/deployment.yaml is valid
- [ ] Test: base/service.yaml is valid
- [ ] Test: base/configmap.yaml is valid
- [ ] Test: Kustomize build succeeds: `kustomize build base/`

### 3.2 Base Resources
- [ ] Test: Deployment uses latest image
- [ ] Test: Service exposes port 8080
- [ ] Test: ConfigMap contains minimal config
- [ ] Test: Namespace not hardcoded in base

### 3.3 Overlays - Development
- [ ] Test: overlays/dev/kustomization.yaml exists
- [ ] Test: Dev uses single replica
- [ ] Test: Dev has lower resource limits
- [ ] Test: Dev uses debug logging
- [ ] Test: Kustomize build succeeds: `kustomize build overlays/dev/`

### 3.4 Overlays - Production
- [ ] Test: overlays/prod/kustomization.yaml exists
- [ ] Test: Prod uses 3 replicas
- [ ] Test: Prod has production resource limits
- [ ] Test: Prod has PodDisruptionBudget
- [ ] Test: Prod has anti-affinity rules
- [ ] Test: Kustomize build succeeds: `kustomize build overlays/prod/`

### 3.5 Overlays - HA with Redis
- [ ] Test: overlays/ha-redis/kustomization.yaml exists
- [ ] Test: Includes Redis StatefulSet
- [ ] Test: ConfigMap enables Redis cache
- [ ] Test: Kustomize build succeeds: `kustomize build overlays/ha-redis/`

### 3.6 Overlays - Full Stack (OPA + OpenFGA)
- [ ] Test: overlays/full-stack/kustomization.yaml exists
- [ ] Test: Includes OPA Deployment
- [ ] Test: Includes OpenFGA Deployment
- [ ] Test: ConfigMap has OPA/OpenFGA settings
- [ ] Test: Kustomize build succeeds: `kustomize build overlays/full-stack/`

---

## Phase 4: Docker Compose Examples

### 4.1 Simple Example
- [ ] Test: examples/docker-compose/simple/docker-compose.yml exists
- [ ] Test: Simple has yatagarasu + minio services
- [ ] Test: Simple has README.md with instructions
- [ ] Test: `docker compose config` validates
- [ ] Test: Example starts successfully

### 4.2 HA with Redis Example
- [ ] Test: examples/docker-compose/ha-redis/docker-compose.yml exists
- [ ] Test: HA has multiple yatagarasu replicas
- [ ] Test: HA has Redis service
- [ ] Test: HA has nginx load balancer
- [ ] Test: Example config enables Redis cache
- [ ] Test: `docker compose config` validates

### 4.3 Full Stack Example (HA + OPA + OpenFGA)
- [ ] Test: examples/docker-compose/full-stack/docker-compose.yml exists
- [ ] Test: Full stack has OPA service
- [ ] Test: Full stack has OpenFGA service
- [ ] Test: Full stack has PostgreSQL for OpenFGA
- [ ] Test: Includes sample OPA policy file
- [ ] Test: Includes OpenFGA model file
- [ ] Test: README explains all components
- [ ] Test: `docker compose config` validates

### 4.4 Example Documentation
- [ ] Test: Each example has README.md
- [ ] Test: README has prerequisites
- [ ] Test: README has quick start commands
- [ ] Test: README has verification steps
- [ ] Test: README has cleanup commands

---

## Phase 5: Kubernetes Examples

### 5.1 Basic Helm Example
- [ ] Test: examples/kubernetes/helm-basic/ directory exists
- [ ] Test: Has values.yaml with minimal config
- [ ] Test: Has README.md with helm install commands
- [ ] Test: README shows how to verify deployment
- [ ] Test: README shows how to test proxy

### 5.2 Production Kustomize Example
- [ ] Test: examples/kubernetes/kustomize-prod/ directory exists
- [ ] Test: Uses overlays/prod as base
- [ ] Test: Has site-specific patches
- [ ] Test: Has README.md with kubectl apply commands
- [ ] Test: Shows namespace creation
- [ ] Test: Shows secret creation from env vars

### 5.3 Full Stack Kubernetes Example
- [ ] Test: examples/kubernetes/full-stack/ directory exists
- [ ] Test: Deploys yatagarasu + Redis + OPA + OpenFGA
- [ ] Test: Has Ingress configuration
- [ ] Test: Has NetworkPolicy for security
- [ ] Test: README is comprehensive walkthrough

---

## Phase 6: Documentation Website

### 6.1 Site Infrastructure (GitHub Pages + mdBook/MkDocs)
- [ ] Test: .github/workflows/docs.yml deploys on push
- [ ] Test: docs-site/ directory has site config
- [ ] Test: Site builds without errors
- [ ] Test: Site deploys to gh-pages branch
- [ ] Test: Site accessible at julianshen.github.io/yatagarasu

### 6.2 Quick Start Guide
- [ ] Test: docs-site/src/quickstart.md exists
- [ ] Test: Covers Docker quick start (5 min)
- [ ] Test: Covers binary installation
- [ ] Test: Covers first request example
- [ ] Test: Has copy-paste commands

### 6.3 Configuration Tutorials
- [ ] Test: docs-site/src/tutorials/ directory exists
- [ ] Test: Tutorial: Basic multi-bucket setup
- [ ] Test: Tutorial: Adding JWT authentication
- [ ] Test: Tutorial: Enabling caching
- [ ] Test: Tutorial: Setting up OPA policies
- [ ] Test: Tutorial: OpenFGA authorization
- [ ] Test: Each tutorial is step-by-step

### 6.4 Architecture Documentation
- [ ] Test: docs-site/src/architecture/ directory exists
- [ ] Test: Doc: Overall system architecture
- [ ] Test: Doc: Request flow diagram
- [ ] Test: Doc: Caching layer design
- [ ] Test: Doc: Authentication flow
- [ ] Test: Doc: Authorization flow (OPA/OpenFGA)
- [ ] Test: Doc: Streaming architecture

### 6.5 Deployment Guide
- [ ] Test: docs-site/src/deployment/ directory exists
- [ ] Test: Doc: Docker deployment
- [ ] Test: Doc: Kubernetes with Helm
- [ ] Test: Doc: Kubernetes with Kustomize
- [ ] Test: Doc: Production checklist
- [ ] Test: Doc: Scaling guidelines

### 6.6 API Reference
- [ ] Test: docs-site/src/api/ directory exists
- [ ] Test: Doc: Proxy endpoints (GET, HEAD, OPTIONS)
- [ ] Test: Doc: Health endpoints (/health, /ready)
- [ ] Test: Doc: Metrics endpoint
- [ ] Test: Doc: Admin API (cache prewarm)
- [ ] Test: Doc: Error codes and responses

### 6.7 Troubleshooting Guide
- [ ] Test: docs-site/src/troubleshooting.md exists
- [ ] Test: Common errors and solutions
- [ ] Test: Debug logging instructions
- [ ] Test: Performance troubleshooting
- [ ] Test: FAQ section

### 6.8 Examples Section
- [ ] Test: docs-site/src/examples/ directory exists
- [ ] Test: Links to Docker Compose examples
- [ ] Test: Links to Kubernetes examples
- [ ] Test: Configuration snippets
- [ ] Test: Integration patterns (CDN, auth providers)

---

## Phase 7: Integration Testing

### 7.1 Cache Warming E2E
- [ ] Test: Prewarm API creates task and warms cache
- [ ] Test: Progress updates correctly during warming
- [ ] Test: Cache hits increase after warming
- [ ] Test: Cancellation stops warming

### 7.2 Helm Chart E2E
- [ ] Test: `helm install` deploys working proxy
- [ ] Test: `helm upgrade` updates configuration
- [ ] Test: `helm uninstall` cleans up resources

### 7.3 Kustomize E2E
- [ ] Test: `kubectl apply -k` deploys working proxy
- [ ] Test: Each overlay produces valid resources
- [ ] Test: Overlays can be customized with patches

### 7.4 Example Validation
- [ ] Test: All Docker Compose examples start and work
- [ ] Test: All Kubernetes examples deploy and work
- [ ] Test: Documentation commands work as written

---

## Directory Structure (Final)

```
yatagarasu/
├── charts/
│   └── yatagarasu/
│       ├── Chart.yaml
│       ├── values.yaml
│       ├── templates/
│       │   ├── deployment.yaml
│       │   ├── service.yaml
│       │   ├── configmap.yaml
│       │   ├── secret.yaml
│       │   ├── ingress.yaml
│       │   ├── servicemonitor.yaml
│       │   ├── pdb.yaml
│       │   ├── hpa.yaml
│       │   └── _helpers.tpl
│       └── README.md
│
├── kustomize/
│   ├── base/
│   │   ├── kustomization.yaml
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── configmap.yaml
│   └── overlays/
│       ├── dev/
│       ├── prod/
│       ├── ha-redis/
│       └── full-stack/
│
├── examples/
│   ├── docker-compose/
│   │   ├── simple/
│   │   ├── ha-redis/
│   │   └── full-stack/
│   └── kubernetes/
│       ├── helm-basic/
│       ├── kustomize-prod/
│       └── full-stack/
│
├── docs-site/
│   ├── book.toml (or mkdocs.yml)
│   └── src/
│       ├── SUMMARY.md
│       ├── quickstart.md
│       ├── tutorials/
│       ├── architecture/
│       ├── deployment/
│       ├── api/
│       ├── troubleshooting.md
│       └── examples/
│
└── src/
    ├── admin/              # NEW: Admin API
    │   ├── mod.rs
    │   └── prewarm.rs
    └── cache/
        └── warming.rs      # NEW: Cache warming worker
```

---

## Implementation Order

1. **Phase 1** (Cache Warming) - Core new feature
2. **Phase 4** (Docker Examples) - Easiest deployment option
3. **Phase 2** (Helm Chart) - Standard K8s deployment
4. **Phase 3** (Kustomize) - GitOps-friendly deployment
5. **Phase 5** (K8s Examples) - Usage examples
6. **Phase 6** (Documentation) - Comprehensive docs
7. **Phase 7** (Integration) - End-to-end validation

---

## Estimated Effort

| Phase | Items | Complexity |
|-------|-------|------------|
| Phase 1: Cache Warming | 45 tests | High |
| Phase 2: Helm Chart | 35 tests | Medium |
| Phase 3: Kustomize | 25 tests | Medium |
| Phase 4: Docker Examples | 20 tests | Low |
| Phase 5: K8s Examples | 15 tests | Low |
| Phase 6: Documentation | 35 tests | Medium |
| Phase 7: Integration | 12 tests | Medium |
| **Total** | **187 tests** | |

---

## Success Criteria

- [ ] All 187 tests passing
- [ ] `helm lint` passes
- [ ] `kustomize build` succeeds for all overlays
- [ ] All Docker Compose examples start successfully
- [ ] Documentation site builds and deploys
- [ ] Cache warming API documented and tested
- [ ] Users can deploy in <5 minutes with any method

---

**Ready to start? Say "go" to implement the first test!**
