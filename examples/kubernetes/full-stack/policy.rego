# Yatagarasu Authorization Policy
# OPA Rego policy for role-based access control
package yatagarasu.authz

import rego.v1

# Default decisions
default allow := false
default grant := false
default deny := false

# Grant access to admins for all paths
grant if {
    input.claims.role == "admin"
}

# Grant access to editors for non-admin paths
grant if {
    input.claims.role == "editor"
    not startswith(input.path, "/admin")
}

# Grant access to viewers for public paths only
grant if {
    input.claims.role == "viewer"
    startswith(input.path, "/public")
}

# Deny access to sensitive paths for non-admins
deny if {
    contains(input.path, "/sensitive/")
    input.claims.role != "admin"
}

# Deny access to .env files
deny if {
    endswith(input.path, ".env")
}

# Deny access to hidden files (starting with .)
deny if {
    path_parts := split(input.path, "/")
    some part in path_parts
    startswith(part, ".")
    part != ""
}

# Final allow decision: grant must be true and deny must be false
allow if {
    grant
    not deny
}

# Helper: Check if user has specific permission
has_permission(permission) if {
    some perm in input.claims.permissions
    perm == permission
}

# Alternative grant based on explicit permissions
grant if {
    has_permission("s3:read")
}

# Audit logging helper (can be queried separately)
audit := {
    "allowed": allow,
    "user": input.claims.sub,
    "role": input.claims.role,
    "path": input.path,
    "method": input.method,
    "timestamp": time.now_ns()
}
