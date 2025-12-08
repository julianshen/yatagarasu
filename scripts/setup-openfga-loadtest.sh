#!/bin/bash
#
# Setup script for OpenFGA load testing
#
# This script creates an OpenFGA store, writes the authorization model,
# and creates sample tuples for load testing.
#
# Prerequisites:
#   - OpenFGA server running (e.g., docker run -d -p 8081:8080 openfga/openfga run)
#   - curl and jq installed
#
# Usage:
#   ./scripts/setup-openfga-loadtest.sh [openfga_url]
#
# Environment variables:
#   OPENFGA_URL - OpenFGA server URL (default: http://localhost:8081)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OPENFGA_URL="${1:-${OPENFGA_URL:-http://localhost:8081}}"

echo "================================================="
echo "OpenFGA Load Test Setup"
echo "================================================="
echo "OpenFGA URL: $OPENFGA_URL"
echo ""

# Check if OpenFGA is reachable
echo "Checking OpenFGA connectivity..."
if ! curl -sf "${OPENFGA_URL}/healthz" > /dev/null 2>&1; then
    echo "ERROR: Cannot connect to OpenFGA at ${OPENFGA_URL}"
    echo ""
    echo "Please start OpenFGA first:"
    echo "  docker run -d -p 8081:8080 --name openfga openfga/openfga run"
    echo ""
    exit 1
fi
echo "OpenFGA is reachable."
echo ""

# Create a new store
echo "Creating OpenFGA store 'yatagarasu-loadtest'..."
STORE_RESPONSE=$(curl -sf -X POST "${OPENFGA_URL}/stores" \
    -H "Content-Type: application/json" \
    -d '{"name": "yatagarasu-loadtest"}')

if [ $? -ne 0 ]; then
    echo "ERROR: Failed to create store"
    exit 1
fi

STORE_ID=$(echo "$STORE_RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$STORE_ID" ]; then
    echo "ERROR: Failed to parse store ID from response"
    echo "Response: $STORE_RESPONSE"
    exit 1
fi

echo "Store created successfully!"
echo "Store ID: $STORE_ID"
echo ""

# Write authorization model
echo "Writing authorization model..."
MODEL_RESPONSE=$(curl -sf -X POST "${OPENFGA_URL}/stores/${STORE_ID}/authorization-models" \
    -H "Content-Type: application/json" \
    -d @"${PROJECT_DIR}/openfga/model.json")

if [ $? -ne 0 ]; then
    echo "ERROR: Failed to write authorization model"
    exit 1
fi

MODEL_ID=$(echo "$MODEL_RESPONSE" | grep -o '"authorization_model_id":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$MODEL_ID" ]; then
    echo "ERROR: Failed to parse model ID from response"
    echo "Response: $MODEL_RESPONSE"
    exit 1
fi

echo "Authorization model written successfully!"
echo "Model ID: $MODEL_ID"
echo ""

# Write relationship tuples
echo "Writing authorization tuples..."
TUPLES_RESPONSE=$(curl -sf -X POST "${OPENFGA_URL}/stores/${STORE_ID}/write" \
    -H "Content-Type: application/json" \
    -d @"${PROJECT_DIR}/openfga/tuples.json")

if [ $? -ne 0 ]; then
    echo "ERROR: Failed to write tuples"
    exit 1
fi

echo "Tuples written successfully!"
echo ""

# Verify setup by checking a permission
echo "Verifying setup with a test check..."
CHECK_RESPONSE=$(curl -sf -X POST "${OPENFGA_URL}/stores/${STORE_ID}/check" \
    -H "Content-Type: application/json" \
    -d '{
        "tuple_key": {
            "user": "user:alice",
            "relation": "viewer",
            "object": "bucket:test-openfga"
        }
    }')

ALLOWED=$(echo "$CHECK_RESPONSE" | grep -o '"allowed":[^,}]*' | cut -d':' -f2)

if [ "$ALLOWED" = "true" ]; then
    echo "Verification successful! user:alice has viewer access to bucket:test-openfga"
else
    echo "WARNING: Verification check returned: $CHECK_RESPONSE"
fi

echo ""
echo "================================================="
echo "OpenFGA Setup Complete!"
echo "================================================="
echo ""
echo "Store ID: $STORE_ID"
echo "Model ID: $MODEL_ID"
echo ""
echo "To start the proxy with OpenFGA authorization:"
echo ""
echo "  export OPENFGA_STORE_ID=$STORE_ID"
echo "  cargo run --release -- --config config/loadtest/config.loadtest-openfga.yaml"
echo ""
echo "To run load tests:"
echo ""
echo "  k6 run k6/openfga-load.js"
echo ""
echo "Test users configured:"
echo "  - user:alice   - viewer on bucket:test-openfga"
echo "  - user:bob     - editor on bucket:test-openfga"
echo "  - user:charlie - viewer on file:test-openfga/test-1kb.txt only"
echo "  - user:diana   - owner on bucket:test-openfga"
echo ""
echo "================================================="

# Export for shell use
echo ""
echo "# Run this to set environment variable:"
echo "export OPENFGA_STORE_ID=$STORE_ID"
