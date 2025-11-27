# Load Test Authorization Policy
# Package: yatagarasu.authz
#
# This policy is designed for load testing OPA integration.
# It supports various test scenarios:
# - Admin role: Full access
# - User role: Access to allowed_bucket only
# - Public paths: /public/* accessible to all
# - Denied: Users with no roles are denied

package yatagarasu.authz

default allow = false

# Allow admins to access everything
allow if {
    input.jwt_claims.roles[_] == "admin"
}

# Allow users to access their allowed bucket
allow if {
    input.jwt_claims.roles[_] == "user"
    input.bucket == input.jwt_claims.allowed_bucket
}

# Allow access to public paths (for mixed workload testing)
allow if {
    startswith(input.path, "/public/")
}

# Allow users with specific bucket access claim
allow if {
    input.bucket == input.jwt_claims.bucket_access[_]
}
