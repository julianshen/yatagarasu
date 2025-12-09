# Yatagarasu OPA Authorization Policy
# This policy controls access to OPA-protected buckets

package yatagarasu.authz

import rego.v1

default allow := false
default grant := false
default deny := false

# Grant access if user has admin role
grant if {
    input.claims.role == "admin"
}

# Grant access if user has read role and is doing GET/HEAD
grant if {
    input.claims.role == "reader"
    input.method in ["GET", "HEAD"]
}

# Grant access to specific paths for specific users
grant if {
    input.claims.sub == "alice"
    startswith(input.path, "/opa/")
}

# Deny access to sensitive paths (non-admins)
deny if {
    contains(input.path, "/secret/")
    input.claims.role != "admin"
}

# Final decision: allow if any grant rule is true AND no deny rule is true
allow if {
    grant
    not deny
}
