# Authentication & Authorization

## JWT Authentication

Yatagarasu can validate JSON Web Tokens (JWT) to secure access.

```yaml
auth:
  jwt:
    enabled: true
    # The issuer URL (e.g., Auth0, Keycloak)
    issuer: "https://my-auth-provider.com/"
    # Audience to validate
    audience: "yatagarasu-proxy"
```

When enabled, all requests must include a valid `Authorization: Bearer <token>` header.

## OPA Authorization

For fine-grained access control (e.g., "User X can only read from Bucket Y"), integrate Open Policy Agent (OPA).

```yaml
auth:
  opa:
    enabled: true
    url: "http://localhost:8181/v1/data/s3/allow"
```

Yatagarasu sends the request details (user, path, method) to OPA, which returns a boolean decision.

## OpenFGA (ReBAC)

For relationship-based access control (e.g., "User X is an editor of Folder Y"), use OpenFGA.

```yaml
auth:
  openfga:
    enabled: true
    store_id: "01H0..."
    endpoint: "http://localhost:8081"
```
