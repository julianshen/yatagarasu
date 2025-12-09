# Quick Start Guide

This guide will help you get Yatagarasu up and running in minutes using Docker.

## Prerequisites

- Docker and Docker Compose installed
- An S3-compatible bucket (or we can spin one up with MinIO)

## Running with Docker Compose

The easiest way to try Yatagarasu is using the provided Docker Compose examples.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/julianshen/yatagarasu.git
   cd yatagarasu
   ```

2. **Start the Simple Stack:**
   This starts Yatagarasu along with a MinIO instance acting as the backend.
   ```bash
   cd examples/docker-compose/simple
   docker compose up -d
   ```

3. **Verify it's running:**
   ```bash
   curl -I http://localhost:8080/health
   ```
   You should see a `200 OK` response.

## Making Your First Request

1. **Populate MinIO (Backend):**
   The example includes a `create-bucket` script, but let's confirm.
   Access MinIO Console at `http://localhost:9001` (User: `minioadmin`, Pass: `minioadmin`).
   Ensure a bucket named `test-bucket` exists and upload a file named `hello.txt`.

2. **Request via Proxy:**
   Now, request that file through Yatagarasu (port 8080).
   ```bash
   curl http://localhost:8080/test-bucket/hello.txt
   ```

3. **Check Caching:**
   Request it again. It should be faster. You can check the `x-cache-status` header if you enabled debug mode, or check metrics at `http://localhost:9090/metrics`.

## Clean Up

When finished:
```bash
docker compose down
```
