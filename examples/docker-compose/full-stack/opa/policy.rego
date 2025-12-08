# Yatagarasu OPA Authorization Policy
# This policy controls access to OPA-protected buckets

package yatagarasu.authz

import rego.v1

default allow := false

# Allow access if user has admin role
allow if {
    input.claims.role == "admin"
}

# Allow access if user has read role and is doing GET/HEAD
allow if {
    input.claims.role == "reader"
    input.method in ["GET", "HEAD"]
}

# Allow access to specific paths for specific users
allow if {
    input.claims.sub == "alice"
    startswith(input.path, "/opa/")
}

# Deny access to sensitive paths
deny if {
    contains(input.path, "/secret/")
    input.claims.role != "admin"
}
