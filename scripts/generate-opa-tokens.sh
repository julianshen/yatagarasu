#!/bin/bash
# Generate JWT tokens for OPA load testing
# Uses HMAC-SHA256 with the test secret

JWT_SECRET="test-secret-key-for-load-testing-only"

# Base64url encode (no padding, replace + with -, / with _)
base64url_encode() {
    openssl base64 -e -A | tr '+/' '-_' | tr -d '='
}

# HMAC-SHA256 signature
hmac_sha256() {
    echo -n "$1" | openssl dgst -sha256 -hmac "$JWT_SECRET" -binary | base64url_encode
}

# Generate JWT
generate_jwt() {
    local header='{"alg":"HS256","typ":"JWT"}'
    local payload="$1"

    local header_b64=$(echo -n "$header" | base64url_encode)
    local payload_b64=$(echo -n "$payload" | base64url_encode)
    local signature=$(hmac_sha256 "${header_b64}.${payload_b64}")

    echo "${header_b64}.${payload_b64}.${signature}"
}

# Far future expiry (year 2030)
EXP=1893456000

echo "Generating JWT tokens for OPA load testing..."
echo ""

# Admin token (full access)
ADMIN_PAYLOAD='{"sub":"admin","roles":["admin"],"exp":'"${EXP}"'}'
ADMIN_TOKEN=$(generate_jwt "$ADMIN_PAYLOAD")
echo "ADMIN_TOKEN:"
echo "$ADMIN_TOKEN"
echo ""

# User token (limited access)
USER_PAYLOAD='{"sub":"user1","roles":["user"],"allowed_bucket":"test-opa","exp":'"${EXP}"'}'
USER_TOKEN=$(generate_jwt "$USER_PAYLOAD")
echo "USER_TOKEN:"
echo "$USER_TOKEN"
echo ""

# Denied token (no roles)
DENIED_PAYLOAD='{"sub":"denied","roles":[],"exp":'"${EXP}"'}'
DENIED_TOKEN=$(generate_jwt "$DENIED_PAYLOAD")
echo "DENIED_TOKEN:"
echo "$DENIED_TOKEN"
echo ""

# Export for k6
echo "# Copy these to run k6 tests:"
echo "export ADMIN_TOKEN='$ADMIN_TOKEN'"
echo "export USER_TOKEN='$USER_TOKEN'"
echo "export DENIED_TOKEN='$DENIED_TOKEN'"
