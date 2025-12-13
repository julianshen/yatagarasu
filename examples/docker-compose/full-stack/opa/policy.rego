# Yatagarasu OPA Authorization Policy
# This policy controls access to OPA-protected buckets
#
# Input from yatagarasu proxy:
#   - jwt_claims: JWT claims from the authenticated token
#   - bucket: Name of the bucket being accessed
#   - path: Request path within the bucket
#   - method: HTTP method (GET, HEAD)
#   - client_ip: Client IP address (optional)

package yatagarasu.authz

import rego.v1

default allow := false
default grant := false
default deny := false

# Grant access if user has admin role
grant if {
    input.jwt_claims.role == "admin"
}

# Grant access if user has read role and is doing GET/HEAD
grant if {
    input.jwt_claims.role == "reader"
    input.method in ["GET", "HEAD"]
}

# Grant access to specific paths for specific users
grant if {
    input.jwt_claims.sub == "alice"
    startswith(input.path, "/opa/")
}

# Deny access to sensitive paths (non-admins)
deny if {
    contains(input.path, "/secret/")
    input.jwt_claims.role != "admin"
}

# Final decision: allow if any grant rule is true AND no deny rule is true
allow if {
    grant
    not deny
}
