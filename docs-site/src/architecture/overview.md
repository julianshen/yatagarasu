# System Architecture

Yatagarasu is built on the [Pingora](https://github.com/cloudflare/pingora) framework, leveraging its asynchronous, multi-threaded engine for high-throughput proxying.

## Request Flow

1. **Ingress**: Traffic enters via HTTP/HTTPS.
2. **Authentication**:
    - JWTs are validated against the configured provider.
    - If valid, the request proceeds.
3. **Authorization**:
    - OPA/OpenFGA checks are performed.
4. **Cache Lookup**:
    - **Memory**: Checks local Moka cache.
    - **Disk**: If enabled, checks local disk cache.
    - **Redis**: If enabled, checks distributed Redis cache.
5. **Upstream Request**:
    - If cache miss, requests object from S3 backend (AWS/MinIO).
6. **Response & Cache Population**:
    - Response is streamed to the client.
    - Response is asynchronously written to cache layers.
