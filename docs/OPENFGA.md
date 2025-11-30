# OpenFGA Integration Design

**Status**: Design Document
**Plan Reference**: [plan_v1.2.md](../plan_v1.2.md) - Milestone 3, Phases 48-50
**Documentation**: https://openfga.dev/docs

---

## Table of Contents

1. [Overview](#overview)
2. [OpenFGA Core Concepts](#openfga-core-concepts)
3. [Comparison: OPA vs OpenFGA](#comparison-opa-vs-openfga)
4. [Architecture Design](#architecture-design)
5. [Configuration Schema](#configuration-schema)
6. [Authorization Model Design](#authorization-model-design)
7. [Implementation Details](#implementation-details)
8. [API Integration](#api-integration)
9. [Caching Strategy](#caching-strategy)
10. [Error Handling](#error-handling)
11. [Testing Strategy](#testing-strategy)
12. [Performance Considerations](#performance-considerations)
13. [Migration Path](#migration-path)
14. [References](#references)

---

## Overview

OpenFGA is a high-performance, flexible authorization system based on Google's Zanzibar paper. It provides **Relationship-Based Access Control (ReBAC)**, which complements the existing OPA (Open Policy Agent) policy-based authorization in Yatagarasu.

### Why OpenFGA for Yatagarasu?

1. **Object Hierarchies**: Natural fit for S3's folder/file structure
2. **Sharing Models**: Easily model "user X shared folder Y with user Z"
3. **Team Hierarchies**: Support organizational access patterns
4. **Scalability**: Designed for millions of relationships
5. **Fine-Grained**: Per-object permission checks

### Plan Reference

This design corresponds to **Milestone 3: OpenFGA Integration** in [plan_v1.2.md](../plan_v1.2.md):

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 48 | OpenFGA Client Foundation | Planned |
| Phase 49 | OpenFGA Integration with Proxy | Planned |
| Phase 50 | OpenFGA Testing & Documentation | Planned |

---

## OpenFGA Core Concepts

### 1. Stores

A **store** is an isolated container for authorization data. Each store has:
- Its own authorization model
- Its own relationship tuples
- Independent access control

```
OpenFGA Server
├── Store: "production"     → prod authorization data
├── Store: "staging"        → staging authorization data
└── Store: "development"    → dev authorization data
```

**Yatagarasu Design**: One store per environment or one store shared across all buckets.

### 2. Authorization Model

The **authorization model** defines:
- **Types**: Categories of objects (user, folder, file, team)
- **Relations**: How types relate to each other (owner, viewer, editor)
- **Computed Relations**: Relations derived from other relations

```dsl
model
  schema 1.1

type user

type folder
  relations
    define owner: [user]
    define editor: [user, team#member] or owner
    define viewer: [user, team#member] or editor

type file
  relations
    define parent: [folder]
    define owner: [user]
    define editor: [user] or editor from parent
    define viewer: [user] or viewer from parent or editor
```

### 3. Relationship Tuples

**Tuples** are the fundamental data units representing relationships:

```
(user:alice, owner, folder:documents)
(user:bob, viewer, folder:documents)
(team:engineering#member, editor, folder:code)
(file:report.pdf, parent, folder:documents)
```

Format: `(user, relation, object)`

### 4. Check API

The **Check API** answers: "Does user X have relation R to object O?"

```http
POST /stores/{store_id}/check
Content-Type: application/json

{
  "tuple_key": {
    "user": "user:alice",
    "relation": "viewer",
    "object": "file:report.pdf"
  }
}
```

Response:
```json
{
  "allowed": true
}
```

### 5. Batch Check API

For multiple checks in a single request (performance optimization):

```http
POST /stores/{store_id}/batch-check
Content-Type: application/json

{
  "checks": [
    { "tuple_key": { "user": "user:alice", "relation": "viewer", "object": "file:a.pdf" } },
    { "tuple_key": { "user": "user:alice", "relation": "viewer", "object": "file:b.pdf" } }
  ]
}
```

---

## Comparison: OPA vs OpenFGA

| Feature | OPA (Rego) | OpenFGA |
|---------|------------|---------|
| **Model** | Policy-based (ABAC) | Relationship-based (ReBAC) |
| **Language** | Rego (declarative) | Authorization Model DSL |
| **Best For** | Complex business rules | Object hierarchies, sharing |
| **Query** | "Can user X do Y given context Z?" | "Does user have relation R to object O?" |
| **Performance** | In-memory evaluation | Graph traversal with caching |
| **State** | Stateless (policy + input) | Stateful (stores relationships) |
| **Use Cases** | Time-based access, file type rules | Folder sharing, team permissions |

### When to Use Each

**Use OPA when:**
- Access depends on request attributes (time, IP, file type)
- Complex boolean logic needed
- Decisions based on JWT claims
- No persistent relationships needed

**Use OpenFGA when:**
- Modeling object hierarchies (folders → files)
- User-to-user sharing (Alice shares with Bob)
- Team/group-based access
- Permission inheritance needed

**Yatagarasu Supports Both**: Configure per-bucket based on use case.

---

## Architecture Design

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Client Request                             │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Yatagarasu Proxy                             │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                        Router (path → bucket)                    ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                   │                                  │
│                                   ▼                                  │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    JWT Authentication                            ││
│  │                  (extract user identity)                         ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                   │                                  │
│                                   ▼                                  │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                  Authorization Provider                          ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  ││
│  │  │   JWT Only  │  │    OPA      │  │       OpenFGA           │  ││
│  │  │  (claims)   │  │ (policy)    │  │  (relationships)        │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                   │                                  │
│                                   ▼                                  │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    S3 Proxy / Cache                              ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    ▼              ▼              ▼
              ┌──────────┐  ┌──────────┐  ┌──────────────┐
              │   OPA    │  │ OpenFGA  │  │     S3       │
              │  Server  │  │  Server  │  │   Backend    │
              └──────────┘  └──────────┘  └──────────────┘
```

### Module Structure

```
src/
├── auth/
│   ├── mod.rs              # Auth module root, trait definitions
│   ├── jwt.rs              # JWT validation (existing)
│   ├── openfga/            # NEW: OpenFGA module
│   │   ├── mod.rs          # OpenFGA module root
│   │   ├── client.rs       # HTTP client for OpenFGA API
│   │   ├── config.rs       # Configuration types
│   │   ├── cache.rs        # Authorization decision cache
│   │   ├── types.rs        # Request/response types
│   │   └── authorizer.rs   # Authorization logic
│   └── provider.rs         # Auth provider abstraction
├── opa/
│   └── mod.rs              # OPA integration (existing)
└── proxy/
    └── mod.rs              # Proxy with auth integration
```

### Authorization Flow

```
1. Request arrives → Extract bucket from path
2. Load bucket config → Determine auth provider (jwt/opa/openfga)
3. If OpenFGA:
   a. Extract user ID from JWT claims (configurable claim)
   b. Build OpenFGA user string: "user:{user_id}"
   c. Build OpenFGA object string: "{type}:{bucket}/{path}"
   d. Determine relation from HTTP method: GET→viewer, PUT→editor
   e. Check cache for (user, relation, object)
   f. If cache miss, call OpenFGA Check API
   g. Cache result with TTL
   h. Return allow/deny decision
4. If allowed → Proxy to S3
5. If denied → Return 403 Forbidden
```

---

## Configuration Schema

### YAML Configuration

```yaml
# config.yaml
buckets:
  # Bucket with OpenFGA authorization
  - name: "shared-files"
    path_prefix: "/shared"
    s3:
      bucket: "shared-bucket"
      region: "us-east-1"
      endpoint: "http://localhost:9000"
    auth:
      enabled: true
      provider: "openfga"  # Options: "jwt", "opa", "openfga"

      # JWT config (for user identity extraction)
      jwt:
        enabled: true
        algorithm: "RS256"
        jwks_url: "https://auth.example.com/.well-known/jwks.json"

      # OpenFGA-specific configuration
      openfga:
        # OpenFGA server endpoint
        endpoint: "http://localhost:8080"

        # Store ID (required)
        store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"

        # Authorization model ID (optional, uses latest if not specified)
        authorization_model_id: "01GXSA8YR785C4FYS3C0RTG7B1"

        # API authentication (optional)
        api_token: "${OPENFGA_API_TOKEN}"

        # Request timeout in milliseconds
        timeout_ms: 100

        # Fail mode when OpenFGA is unavailable
        fail_mode: "closed"  # "open" or "closed"

        # User extraction from JWT
        user_claim: "sub"        # JWT claim containing user ID
        user_type: "user"        # OpenFGA type for users

        # Object type mapping
        object_type: "file"      # Default type for S3 objects
        folder_type: "folder"    # Type for folder paths (optional)

        # Relation mapping from HTTP methods
        relation_mapping:
          GET: "viewer"
          HEAD: "viewer"
          PUT: "editor"
          POST: "editor"
          DELETE: "owner"

        # Decision caching
        cache:
          enabled: true
          ttl_seconds: 60           # Cache TTL for allowed decisions
          negative_ttl_seconds: 30  # Cache TTL for denied decisions
          max_entries: 10000        # Maximum cache entries

  # Mixed: OPA for some buckets, OpenFGA for others
  - name: "policy-files"
    path_prefix: "/policy"
    auth:
      enabled: true
      provider: "opa"
      opa:
        url: "http://localhost:8181"
        policy_path: "s3/authz/allow"
```

### Rust Configuration Types

```rust
// src/auth/openfga/config.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenFgaConfig {
    /// OpenFGA server endpoint URL
    pub endpoint: String,

    /// Store ID for authorization data
    pub store_id: String,

    /// Optional authorization model ID (uses latest if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_model_id: Option<String>,

    /// Optional API token for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,

    /// Request timeout in milliseconds (default: 100)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Fail mode when OpenFGA is unavailable
    #[serde(default)]
    pub fail_mode: FailMode,

    /// JWT claim containing user ID
    #[serde(default = "default_user_claim")]
    pub user_claim: String,

    /// OpenFGA type for users
    #[serde(default = "default_user_type")]
    pub user_type: String,

    /// OpenFGA type for objects
    #[serde(default = "default_object_type")]
    pub object_type: String,

    /// OpenFGA type for folders (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_type: Option<String>,

    /// HTTP method to relation mapping
    #[serde(default = "default_relation_mapping")]
    pub relation_mapping: HashMap<String, String>,

    /// Decision caching configuration
    #[serde(default)]
    pub cache: OpenFgaCacheConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OpenFgaCacheConfig {
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,

    #[serde(default = "default_negative_ttl")]
    pub negative_ttl_seconds: u64,

    #[serde(default = "default_max_entries")]
    pub max_entries: u64,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FailMode {
    Open,
    #[default]
    Closed,
}

// Default value functions
fn default_timeout() -> u64 { 100 }
fn default_user_claim() -> String { "sub".to_string() }
fn default_user_type() -> String { "user".to_string() }
fn default_object_type() -> String { "file".to_string() }
fn default_cache_enabled() -> bool { true }
fn default_ttl() -> u64 { 60 }
fn default_negative_ttl() -> u64 { 30 }
fn default_max_entries() -> u64 { 10_000 }

fn default_relation_mapping() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("GET".to_string(), "viewer".to_string());
    map.insert("HEAD".to_string(), "viewer".to_string());
    map.insert("PUT".to_string(), "editor".to_string());
    map.insert("POST".to_string(), "editor".to_string());
    map.insert("DELETE".to_string(), "owner".to_string());
    map
}
```

---

## Authorization Model Design

### Recommended Model for S3 Proxy

```dsl
model
  schema 1.1

# Base user type
type user

# Team/group type for organizational access
type team
  relations
    define member: [user]
    define admin: [user]

# Bucket type for bucket-level permissions
type bucket
  relations
    define owner: [user, team#admin]
    define admin: [user, team#admin] or owner
    define editor: [user, team#member] or admin
    define viewer: [user, team#member] or editor

# Folder type for hierarchical permissions
type folder
  relations
    define parent: [bucket, folder]
    define owner: [user]
    define editor: [user, team#member] or owner or editor from parent
    define viewer: [user, team#member] or editor or viewer from parent

# File type for object-level permissions
type file
  relations
    define parent: [folder, bucket]
    define owner: [user]
    define editor: [user] or owner or editor from parent
    define viewer: [user] or editor or viewer from parent
```

### Permission Inheritance Flow

```
bucket:shared-files
    │
    ├── (user:alice, owner, bucket:shared-files)
    │   └── Alice has owner/admin/editor/viewer on bucket
    │
    └── folder:shared-files/documents
            │
            ├── (folder:shared-files/documents, parent, bucket:shared-files)
            │   └── Folder inherits permissions from bucket
            │
            ├── (user:bob, editor, folder:shared-files/documents)
            │   └── Bob has editor/viewer on this folder
            │
            └── file:shared-files/documents/report.pdf
                    │
                    ├── (file:..., parent, folder:shared-files/documents)
                    │   └── File inherits permissions from folder
                    │
                    └── Bob can view/edit report.pdf (inherited)
```

### Object Naming Convention

For Yatagarasu, we'll use the following naming convention for OpenFGA objects:

```
# Buckets
bucket:{bucket_name}
Example: bucket:shared-files

# Folders (paths ending with /)
folder:{bucket_name}/{path}
Example: folder:shared-files/documents/

# Files (paths not ending with /)
file:{bucket_name}/{path}
Example: file:shared-files/documents/report.pdf

# Users (from JWT)
user:{user_id}
Example: user:alice@example.com

# Teams (optional)
team:{team_id}
Example: team:engineering
```

---

## Implementation Details

### OpenFGA Client

```rust
// src/auth/openfga/client.rs

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

use super::config::OpenFgaConfig;
use super::types::*;

/// Error types for OpenFGA operations
#[derive(Debug, Clone)]
pub enum OpenFgaError {
    Timeout { timeout_ms: u64 },
    ConnectionFailed(String),
    InvalidResponse(String),
    ApiError { code: u16, message: String },
    InvalidConfiguration(String),
}

/// OpenFGA HTTP Client
pub struct OpenFgaClient {
    config: OpenFgaConfig,
    http_client: Client,
}

impl OpenFgaClient {
    /// Create a new OpenFGA client
    pub fn new(config: OpenFgaConfig) -> Result<Self, OpenFgaError> {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| OpenFgaError::InvalidConfiguration(e.to_string()))?;

        Ok(Self { config, http_client })
    }

    /// Check if a user has a relation to an object
    pub async fn check(
        &self,
        user: &str,
        relation: &str,
        object: &str,
    ) -> Result<bool, OpenFgaError> {
        let url = format!(
            "{}/stores/{}/check",
            self.config.endpoint,
            self.config.store_id
        );

        let request = CheckRequest {
            tuple_key: TupleKey {
                user: user.to_string(),
                relation: relation.to_string(),
                object: object.to_string(),
            },
            authorization_model_id: self.config.authorization_model_id.clone(),
        };

        let mut req_builder = self.http_client
            .post(&url)
            .json(&request);

        // Add API token if configured
        if let Some(ref token) = self.config.api_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OpenFgaError::Timeout { timeout_ms: self.config.timeout_ms }
                } else {
                    OpenFgaError::ConnectionFailed(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(OpenFgaError::ApiError {
                code: status,
                message: body,
            });
        }

        let check_response: CheckResponse = response
            .json()
            .await
            .map_err(|e| OpenFgaError::InvalidResponse(e.to_string()))?;

        Ok(check_response.allowed)
    }

    /// Batch check multiple tuples
    pub async fn batch_check(
        &self,
        checks: Vec<TupleKey>,
    ) -> Result<Vec<bool>, OpenFgaError> {
        let url = format!(
            "{}/stores/{}/batch-check",
            self.config.endpoint,
            self.config.store_id
        );

        let request = BatchCheckRequest {
            checks: checks.into_iter().map(|tuple_key| CheckRequest {
                tuple_key,
                authorization_model_id: self.config.authorization_model_id.clone(),
            }).collect(),
        };

        let mut req_builder = self.http_client
            .post(&url)
            .json(&request);

        if let Some(ref token) = self.config.api_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let response = req_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                OpenFgaError::Timeout { timeout_ms: self.config.timeout_ms }
            } else {
                OpenFgaError::ConnectionFailed(e.to_string())
            }
        })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(OpenFgaError::ApiError { code: status, message: body });
        }

        let batch_response: BatchCheckResponse = response
            .json()
            .await
            .map_err(|e| OpenFgaError::InvalidResponse(e.to_string()))?;

        Ok(batch_response.results.into_iter().map(|r| r.allowed).collect())
    }
}

pub type SharedOpenFgaClient = Arc<OpenFgaClient>;
```

### Request/Response Types

```rust
// src/auth/openfga/types.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TupleKey {
    pub user: String,
    pub relation: String,
    pub object: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckRequest {
    pub tuple_key: TupleKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_model_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckResponse {
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchCheckRequest {
    pub checks: Vec<CheckRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchCheckResponse {
    pub results: Vec<CheckResponse>,
}
```

### Authorization Decision Cache

```rust
// src/auth/openfga/cache.rs

use moka::future::Cache;
use sha2::{Digest, Sha256};
use std::time::Duration;

use super::config::OpenFgaCacheConfig;

/// Cache for OpenFGA authorization decisions
pub struct OpenFgaCache {
    /// Cache for allowed decisions (longer TTL)
    allowed_cache: Cache<String, bool>,
    /// Cache for denied decisions (shorter TTL)
    denied_cache: Cache<String, bool>,
}

impl OpenFgaCache {
    pub fn new(config: &OpenFgaCacheConfig) -> Self {
        let allowed_cache = Cache::builder()
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .max_capacity(config.max_entries)
            .build();

        let denied_cache = Cache::builder()
            .time_to_live(Duration::from_secs(config.negative_ttl_seconds))
            .max_capacity(config.max_entries / 10) // Smaller for denied
            .build();

        Self { allowed_cache, denied_cache }
    }

    /// Generate cache key from check parameters
    pub fn cache_key(user: &str, relation: &str, object: &str) -> String {
        let input = format!("{}:{}:{}", user, relation, object);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get cached decision
    pub async fn get(&self, user: &str, relation: &str, object: &str) -> Option<bool> {
        let key = Self::cache_key(user, relation, object);

        // Check allowed cache first (more likely)
        if let Some(allowed) = self.allowed_cache.get(&key).await {
            return Some(allowed);
        }

        // Check denied cache
        self.denied_cache.get(&key).await
    }

    /// Store decision in cache
    pub async fn put(&self, user: &str, relation: &str, object: &str, allowed: bool) {
        let key = Self::cache_key(user, relation, object);

        if allowed {
            self.allowed_cache.insert(key, true).await;
        } else {
            self.denied_cache.insert(key, false).await;
        }
    }
}
```

### Authorizer Implementation

```rust
// src/auth/openfga/authorizer.rs

use super::cache::OpenFgaCache;
use super::client::{OpenFgaClient, OpenFgaError, SharedOpenFgaClient};
use super::config::{FailMode, OpenFgaConfig};
use crate::auth::Claims;

/// Result of an OpenFGA authorization decision
#[derive(Debug)]
pub struct OpenFgaDecision {
    pub allowed: bool,
    pub error: Option<OpenFgaError>,
    pub fail_open: bool,
    pub cached: bool,
}

impl OpenFgaDecision {
    pub fn allowed() -> Self {
        Self { allowed: true, error: None, fail_open: false, cached: false }
    }

    pub fn denied() -> Self {
        Self { allowed: false, error: None, fail_open: false, cached: false }
    }

    pub fn cached(allowed: bool) -> Self {
        Self { allowed, error: None, fail_open: false, cached: true }
    }

    pub fn fail_open(error: OpenFgaError) -> Self {
        Self { allowed: true, error: Some(error), fail_open: true, cached: false }
    }

    pub fn fail_closed(error: OpenFgaError) -> Self {
        Self { allowed: false, error: Some(error), fail_open: false, cached: false }
    }
}

/// OpenFGA Authorizer
pub struct OpenFgaAuthorizer {
    client: SharedOpenFgaClient,
    config: OpenFgaConfig,
    cache: Option<OpenFgaCache>,
}

impl OpenFgaAuthorizer {
    pub fn new(client: SharedOpenFgaClient, config: OpenFgaConfig) -> Self {
        let cache = if config.cache.enabled {
            Some(OpenFgaCache::new(&config.cache))
        } else {
            None
        };

        Self { client, config, cache }
    }

    /// Check authorization for a request
    pub async fn authorize(
        &self,
        claims: &Claims,
        bucket: &str,
        path: &str,
        method: &str,
    ) -> OpenFgaDecision {
        // 1. Extract user ID from JWT claims
        let user_id = match self.extract_user_id(claims) {
            Some(id) => id,
            None => {
                tracing::warn!("User claim '{}' not found in JWT", self.config.user_claim);
                return OpenFgaDecision::denied();
            }
        };

        // 2. Build OpenFGA user string
        let user = format!("{}:{}", self.config.user_type, user_id);

        // 3. Build OpenFGA object string
        let object = self.build_object(bucket, path);

        // 4. Determine relation from HTTP method
        let relation = match self.config.relation_mapping.get(method) {
            Some(r) => r.as_str(),
            None => {
                tracing::warn!("No relation mapping for HTTP method '{}'", method);
                return OpenFgaDecision::denied();
            }
        };

        tracing::debug!(
            user = %user,
            relation = %relation,
            object = %object,
            "OpenFGA authorization check"
        );

        // 5. Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(allowed) = cache.get(&user, relation, &object).await {
                tracing::debug!(allowed = allowed, "OpenFGA cache hit");
                return OpenFgaDecision::cached(allowed);
            }
        }

        // 6. Call OpenFGA Check API
        let result = self.client.check(&user, relation, &object).await;

        // 7. Handle result
        match result {
            Ok(allowed) => {
                // Cache the decision
                if let Some(ref cache) = self.cache {
                    cache.put(&user, relation, &object, allowed).await;
                }

                if allowed {
                    OpenFgaDecision::allowed()
                } else {
                    OpenFgaDecision::denied()
                }
            }
            Err(e) => {
                tracing::error!(error = ?e, "OpenFGA check failed");

                match self.config.fail_mode {
                    FailMode::Open => OpenFgaDecision::fail_open(e),
                    FailMode::Closed => OpenFgaDecision::fail_closed(e),
                }
            }
        }
    }

    /// Extract user ID from JWT claims
    fn extract_user_id(&self, claims: &Claims) -> Option<String> {
        // Try standard claims first
        match self.config.user_claim.as_str() {
            "sub" => claims.sub.clone(),
            "iss" => claims.iss.clone(),
            // Check custom claims
            claim => claims.custom.get(claim)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }
    }

    /// Build OpenFGA object string from bucket and path
    fn build_object(&self, bucket: &str, path: &str) -> String {
        // Determine if this is a folder (ends with /) or file
        let object_type = if path.ends_with('/') {
            self.config.folder_type.as_ref()
                .unwrap_or(&self.config.object_type)
        } else {
            &self.config.object_type
        };

        // Normalize path (remove leading /)
        let normalized_path = path.trim_start_matches('/');

        format!("{}:{}/{}", object_type, bucket, normalized_path)
    }
}
```

---

## API Integration

### OpenFGA API Endpoints Used

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/stores/{store_id}/check` | POST | Single authorization check |
| `/stores/{store_id}/batch-check` | POST | Multiple authorization checks |
| `/stores/{store_id}/read` | POST | Read tuples (admin/debugging) |
| `/stores/{store_id}/write` | POST | Write tuples (admin/setup) |

### Authentication Methods

OpenFGA supports three authentication methods:

1. **None** (development): No authentication required
2. **Pre-shared Key**: API token in `Authorization: Bearer {token}` header
3. **OIDC**: JWT token from identity provider

Yatagarasu supports **pre-shared key** via configuration:

```yaml
openfga:
  api_token: "${OPENFGA_API_TOKEN}"
```

### Request/Response Examples

**Check Request:**
```http
POST /stores/01ARZ3NDEKTSV4RRFFQ69G5FAV/check HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Authorization: Bearer {token}

{
  "tuple_key": {
    "user": "user:alice@example.com",
    "relation": "viewer",
    "object": "file:shared-files/documents/report.pdf"
  },
  "authorization_model_id": "01GXSA8YR785C4FYS3C0RTG7B1"
}
```

**Check Response:**
```json
{
  "allowed": true
}
```

---

## Caching Strategy

### Two-Tier TTL Strategy

| Decision | TTL | Rationale |
|----------|-----|-----------|
| Allowed | 60s | Longer cache for positive decisions |
| Denied | 30s | Shorter cache for denials (permissions may be granted) |

### Cache Key Design

```
SHA256(user:relation:object)
```

Example:
```
SHA256("user:alice:viewer:file:shared-files/docs/report.pdf")
→ "a1b2c3d4e5f6..."
```

### Cache Invalidation Strategies

1. **TTL-based**: Automatic expiration (default)
2. **Manual purge**: Admin endpoint to clear cache (future)
3. **Webhook-based**: OpenFGA change events (advanced, future)

### Metrics

```rust
// Cache metrics to expose via Prometheus
yatagarasu_openfga_cache_hits_total
yatagarasu_openfga_cache_misses_total
yatagarasu_openfga_checks_total{result="allowed|denied|error"}
yatagarasu_openfga_check_duration_seconds
```

---

## Error Handling

### Error Types and HTTP Responses

| OpenFGA Error | Fail-Closed Response | Fail-Open Response |
|---------------|---------------------|-------------------|
| Timeout | 503 Service Unavailable | 200 (log warning) |
| Connection Failed | 503 Service Unavailable | 200 (log warning) |
| Invalid Store ID | 500 Internal Error | 500 Internal Error |
| Invalid Tuple | 400 Bad Request | 400 Bad Request |
| Unauthorized | 500 Internal Error | 500 Internal Error |

### Logging

```rust
// Error logging example
match result {
    Ok(allowed) => tracing::info!(
        user = %user,
        object = %object,
        allowed = allowed,
        "OpenFGA check complete"
    ),
    Err(OpenFgaError::Timeout { timeout_ms }) => tracing::error!(
        timeout_ms = timeout_ms,
        "OpenFGA check timed out"
    ),
    Err(e) => tracing::error!(
        error = ?e,
        "OpenFGA check failed"
    ),
}
```

---

## Testing Strategy

### Unit Tests (Phase 48)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
            endpoint: "http://localhost:8080"
            store_id: "test-store"
        "#;
        let config: OpenFgaConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.endpoint, "http://localhost:8080");
    }

    #[test]
    fn test_build_object_file() {
        let authorizer = create_test_authorizer();
        let object = authorizer.build_object("bucket", "/path/to/file.pdf");
        assert_eq!(object, "file:bucket/path/to/file.pdf");
    }

    #[test]
    fn test_build_object_folder() {
        let authorizer = create_test_authorizer();
        let object = authorizer.build_object("bucket", "/path/to/folder/");
        assert_eq!(object, "folder:bucket/path/to/folder/");
    }

    #[tokio::test]
    async fn test_cache_key_deterministic() {
        let key1 = OpenFgaCache::cache_key("user:alice", "viewer", "file:x");
        let key2 = OpenFgaCache::cache_key("user:alice", "viewer", "file:x");
        assert_eq!(key1, key2);
    }
}
```

### Integration Tests (Phase 50)

```rust
#[tokio::test]
#[ignore] // Requires OpenFGA server
async fn test_openfga_integration() {
    // Start OpenFGA via docker-compose
    // Create test store and model
    // Write test tuples
    // Run authorization checks
}
```

### Load Tests (Phase 50)

```javascript
// k6/openfga-load.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '1m', target: 100 },  // Ramp up
    { duration: '5m', target: 500 },  // Sustained load
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<100'],  // P95 < 100ms with caching
  },
};

export default function () {
  const res = http.get('http://localhost:8080/shared/test-file.txt', {
    headers: { 'Authorization': `Bearer ${__ENV.JWT_TOKEN}` },
  });

  check(res, {
    'status is 200': (r) => r.status === 200,
    'has X-OpenFGA-Decision header': (r) => r.headers['X-OpenFGA-Decision'] !== undefined,
  });

  sleep(0.1);
}
```

---

## Performance Considerations

### Latency Targets

| Scenario | Target P95 | Notes |
|----------|-----------|-------|
| Cache hit | <1ms | In-memory lookup |
| Cache miss (OpenFGA) | <10ms | Single check |
| Cache miss (complex inheritance) | <50ms | Graph traversal |
| With network latency | <100ms | Realistic production |

### Optimization Strategies

1. **Connection Pooling**: Reuse HTTP connections to OpenFGA
2. **Batch Checks**: Use batch API when checking multiple objects
3. **Pre-warming**: Populate cache on startup for common objects
4. **Local Caching**: Moka provides efficient concurrent cache access

### Capacity Planning

| Metric | Recommendation |
|--------|---------------|
| Cache size | 10,000 entries per GB RAM |
| OpenFGA connections | 10-50 per proxy instance |
| Check rate | 1,000-10,000 RPS per OpenFGA instance |

---

## Migration Path

### From JWT-Only to OpenFGA

1. Deploy OpenFGA server
2. Create authorization model
3. Import initial tuples (script provided)
4. Enable OpenFGA on test bucket
5. Monitor authorization decisions
6. Roll out to production buckets

### From OPA to OpenFGA

1. Analyze existing OPA policies
2. Design equivalent OpenFGA model
3. Migrate relationship data
4. Run both in parallel (shadow mode)
5. Compare decisions
6. Switch over

### Rollback Plan

If issues arise with OpenFGA:
1. Change `provider: "openfga"` to `provider: "jwt"` or `provider: "opa"`
2. Reload configuration
3. No data migration needed (stateless proxy)

---

## References

### Official Documentation
- [OpenFGA Documentation](https://openfga.dev/docs)
- [OpenFGA API Reference](https://openfga.dev/api/service)
- [Authorization Model DSL](https://openfga.dev/docs/configuration-language)
- [Modeling Best Practices](https://openfga.dev/docs/modeling)

### Related Yatagarasu Documentation
- [OPA Integration](./OPA_INTEGRATION.md) - Existing policy-based authorization
- [JWT Authentication](./JWT_AUTHENTICATION.md) - Token validation
- [plan_v1.2.md](../plan_v1.2.md) - Development plan with OpenFGA phases

### Google Zanzibar Paper
- [Zanzibar: Google's Consistent, Global Authorization System](https://research.google/pubs/pub48190/)

---

## Appendix: Docker Compose Setup

```yaml
# docker-compose.openfga.yml
version: '3.8'

services:
  openfga:
    image: openfga/openfga:latest
    container_name: yatagarasu-openfga
    ports:
      - "8080:8080"   # HTTP API
      - "8081:8081"   # gRPC API
      - "3000:3000"   # Playground UI
    environment:
      - OPENFGA_DATASTORE_ENGINE=memory
      - OPENFGA_PLAYGROUND_ENABLED=true
    command: run
    healthcheck:
      test: ["CMD", "/usr/local/bin/grpc_health_probe", "-addr=:8081"]
      interval: 5s
      timeout: 3s
      retries: 5

  # Optional: PostgreSQL for persistent storage
  # postgres:
  #   image: postgres:15
  #   environment:
  #     POSTGRES_USER: openfga
  #     POSTGRES_PASSWORD: openfga
  #     POSTGRES_DB: openfga
  #   volumes:
  #     - openfga-data:/var/lib/postgresql/data

volumes:
  openfga-data:
```

### Quick Start Commands

```bash
# Start OpenFGA
docker-compose -f docker-compose.openfga.yml up -d

# Create a store
curl -X POST http://localhost:8080/stores \
  -H "Content-Type: application/json" \
  -d '{"name": "yatagarasu"}'

# Response: {"id": "01ARZ3NDEKTSV4RRFFQ69G5FAV", "name": "yatagarasu", ...}

# Create authorization model
curl -X POST "http://localhost:8080/stores/{store_id}/authorization-models" \
  -H "Content-Type: application/json" \
  -d @openfga-model.json

# Write a tuple
curl -X POST "http://localhost:8080/stores/{store_id}/write" \
  -H "Content-Type: application/json" \
  -d '{
    "writes": {
      "tuple_keys": [
        {"user": "user:alice", "relation": "owner", "object": "bucket:shared-files"}
      ]
    }
  }'

# Check authorization
curl -X POST "http://localhost:8080/stores/{store_id}/check" \
  -H "Content-Type: application/json" \
  -d '{
    "tuple_key": {
      "user": "user:alice",
      "relation": "viewer",
      "object": "file:shared-files/report.pdf"
    }
  }'
```

---

*Document Version: 1.0*
*Last Updated: 2025-11-30*
*Author: Claude Code*
