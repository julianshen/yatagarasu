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
- [x] Test: Admin API router exists at `/admin/*`
- [x] Test: Admin endpoints require authentication
- [x] Test: Admin JWT claims verification (role: admin)
- [x] Test: Returns 401 for missing/invalid admin token
- [x] Test: Returns 403 for valid JWT without admin role

### 1.2 S3 LIST Operation
- [x] Test: S3 client supports ListObjectsV2 operation
- [x] Test: LIST returns object keys with metadata (size, etag)
- [x] Test: LIST supports prefix filtering
- [x] Test: LIST supports pagination (continuation token)
- [x] Test: LIST respects max_keys limit
- [x] Test: Recursive LIST traverses common prefixes

### 1.3 Cache Warm Task Management
- [x] Test: Can create prewarm task with path and options
- [x] Test: Task ID is unique UUID
- [x] Test: Task tracks status (pending, running, completed, failed, cancelled)
- [x] Test: Task tracks progress (files_scanned, files_cached, bytes_cached)
- [x] Test: Task stores start_time, end_time, duration
- [x] Test: Multiple tasks can run concurrently

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

### 1.5 Cache Warming Worker
- [x] Test: Worker lists objects from S3
- [x] Test: Worker filters objects by suffix/regex
- [x] Test: Worker downloads objects (HEAD then GET)
- [x] Test: Worker stores objects in cache
- [x] Test: Worker updates task progress
- [x] Test: Worker handles errors gracefully
- [x] Test: Worker respects concurrency limitsask is cancelled
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
- [x] Test: Chart.yaml has correct apiVersion (v2)
- [x] Test: Chart.yaml has name, version, appVersion
- [x] Test: values.yaml has documented defaults
- [x] Test: Chart validates with `helm lint`
- [x] Test: Chart templates render without errors

### 2.2 Core Templates
- [x] Test: Deployment template creates valid Deployment
- [x] Test: Service template creates ClusterIP service
- [x] Test: ConfigMap template includes config.yaml
- [x] Test: Secret template handles credentials
- [x] Test: ServiceAccount template created
- [x] Test: RBAC templates (Role, RoleBinding) created

### 2.3 Configuration Values
- [x] Test: image.repository defaults to ghcr.io/julianshen/yatagarasu
- [x] Test: image.tag defaults to appVersion
- [x] Test: replicaCount configurable (default: 1)
- [x] Test: resources.limits/requests configurable
- [x] Test: nodeSelector, tolerations, affinity supported
- [x] Test: podAnnotations and podLabels supported

### 2.4 S3 Bucket Configuration
- [x] Test: buckets[] array configures multiple buckets
- [x] Test: Each bucket has name, pathPrefix, s3 settings
- [x] Test: Credentials can reference existing secrets
- [x] Test: Credentials can be created from values

### 2.5 Optional Components
- [x] Test: ingress.enabled creates Ingress resource
- [x] Test: Ingress supports className and annotations
- [x] Test: Ingress supports TLS configuration
- [x] Test: serviceMonitor.enabled creates ServiceMonitor
- [x] Test: ServiceMonitor has correct endpoints
- [x] Test: podDisruptionBudget.enabled creates PDB
- [x] Test: horizontalPodAutoscaler.enabled creates HPA

### 2.6 Health and Probes
- [x] Test: livenessProbe points to /health
- [x] Test: readinessProbe points to /ready
- [x] Test: Probe settings (initialDelaySeconds, etc.) configurable

### 2.7 Advanced Features
- [x] Test: auth.jwt.* configures JWT authentication
- [x] Test: cache.* configures caching layer
- [x] Test: cache.redis.enabled adds Redis config
- [x] Test: authorization.opa.enabled adds OPA config
- [x] Test: authorization.openfga.enabled adds OpenFGA config
- [x] Test: metrics.enabled configures metrics port

---

## Phase 3: Kustomize Base

### 3.1 Base Structure
- [x] Test: base/kustomization.yaml exists
- [x] Test: base/deployment.yaml is valid
- [x] Test: base/service.yaml is valid
- [x] Test: base/configmap.yaml is valid
- [x] Test: Kustomize build succeeds: `kustomize build base/`

### 3.2 Base Resources
- [x] Test: Deployment uses latest image
- [x] Test: Service exposes port 8080
- [x] Test: ConfigMap contains minimal config
- [x] Test: Namespace not hardcoded in base

### 3.3 Overlays - Development
- [x] Test: overlays/dev/kustomization.yaml exists
- [x] Test: Dev uses single replica
- [x] Test: Dev has lower resource limits
- [x] Test: Dev uses debug logging
- [x] Test: Kustomize build succeeds: `kustomize build overlays/dev/`

### 3.4 Overlays - Production
- [x] Test: overlays/prod/kustomization.yaml exists
- [x] Test: Prod uses 3 replicas
- [x] Test: Prod has production resource limits
- [x] Test: Prod has PodDisruptionBudget
- [x] Test: Prod has anti-affinity rules
- [x] Test: Kustomize build succeeds: `kustomize build overlays/prod/`

### 3.5 Overlays - HA with Redis
- [x] Test: overlays/ha-redis/kustomization.yaml exists
- [x] Test: Includes Redis StatefulSet
- [x] Test: ConfigMap enables Redis cache
- [x] Test: Kustomize build succeeds: `kustomize build overlays/ha-redis/`

### 3.6 Overlays - Full Stack (OPA + OpenFGA)
- [x] Test: overlays/full-stack/kustomization.yaml exists
- [x] Test: Includes OPA Deployment
- [x] Test: Includes OpenFGA Deployment
- [x] Test: ConfigMap has OPA/OpenFGA settings
- [x] Test: Kustomize build succeeds: `kustomize build overlays/full-stack/`

---

## Phase 4: Docker Compose Examples

### 4.1 Simple Example
- [x] Test: examples/docker-compose/simple/docker-compose.yml exists
- [x] Test: Simple has yatagarasu + minio services
- [x] Test: Simple has README.md with instructions
- [x] Test: `docker compose config` validates
- [ ] Test: Example starts successfully

### 4.2 HA with Redis Example
- [x] Test: examples/docker-compose/ha-redis/docker-compose.yml exists
- [x] Test: HA has multiple yatagarasu replicas
- [x] Test: HA has Redis service
- [x] Test: HA has nginx load balancer
- [x] Test: Example config enables Redis cache
- [x] Test: `docker compose config` validates

### 4.3 Full Stack Example (HA + OPA + OpenFGA)
- [x] Test: examples/docker-compose/full-stack/docker-compose.yml exists
- [x] Test: Full stack has OPA service
- [x] Test: Full stack has OpenFGA service
- [x] Test: Full stack has PostgreSQL for OpenFGA
- [x] Test: Includes sample OPA policy file
- [x] Test: Includes OpenFGA model file
- [x] Test: README explains all components
- [x] Test: `docker compose config` validates

### 4.4 Example Documentation
- [x] Test: Each example has README.md
- [x] Test: README has prerequisites
- [x] Test: README has quick start commands
- [x] Test: README has verification steps
- [x] Test: README has cleanup commands

---

## Phase 5: Kubernetes Examples

### 5.1 Basic Helm Example
- [x] Test: examples/kubernetes/helm-basic/ directory exists
- [x] Test: Has values.yaml with minimal config
- [x] Test: Has README.md with helm install commands
- [x] Test: README shows how to verify deployment
- [x] Test: README shows how to test proxy

### 5.2 Production Kustomize Example
- [x] Test: examples/kubernetes/kustomize-prod/ directory exists
- [x] Test: Uses base with production patches (standalone)
- [x] Test: Has site-specific patches
- [x] Test: Has README.md with kubectl apply commands
- [x] Test: Shows namespace creation
- [x] Test: Shows secret creation from env vars

### 5.3 Full Stack Kubernetes Example
- [x] Test: examples/kubernetes/full-stack/ directory exists
- [x] Test: Deploys yatagarasu + Redis + OPA + OpenFGA
- [x] Test: Has Ingress configuration
- [x] Test: Has NetworkPolicy for security
- [x] Test: README is comprehensive walkthrough

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

## Development Rules

### Branch and PR Strategy

**IMPORTANT**: Do NOT commit directly to the `master` branch.

1. **Create a feature branch for each phase**:
   ```bash
   git checkout -b feature/v1.3-phase1-cache-warming
   git checkout -b feature/v1.3-phase2-helm-chart
   git checkout -b feature/v1.3-phase3-kustomize
   # etc.
   ```

2. **Create a Pull Request when phase is complete**:
   - All tests for the phase must pass
   - PR title: `[v1.3.0] Phase N: <Phase Name>`
   - PR description includes checklist of completed items
   - Request review before merging

3. **PR Naming Convention**:
   - `feature/v1.3-phase1-cache-warming`
   - `feature/v1.3-phase2-helm-chart`
   - `feature/v1.3-phase3-kustomize`
   - `feature/v1.3-phase4-docker-examples`
   - `feature/v1.3-phase5-k8s-examples`
   - `feature/v1.3-phase6-documentation`
   - `feature/v1.3-phase7-integration`

4. **Merge to master only after**:
   - CI passes
   - Code review approved (if applicable)
   - All phase tests are marked complete in plan

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
