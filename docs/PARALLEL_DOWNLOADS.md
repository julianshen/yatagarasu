# Parallel Downloads Using Range Requests

## Quick Answer

**Q: Does Yatagarasu support parallel download of a large file using range requests?**

**A: YES! ✅ Full support for parallel/concurrent range requests**

---

## How It Works

Yatagarasu supports standard HTTP Range requests, which means download accelerators and clients can split a large file into chunks and download them in parallel.

### Architecture

```
Large File: 100MB (104,857,600 bytes)

Client splits into 10 chunks:
┌─────────────────────────────────────────────────────────┐
│ Chunk 1: bytes=0-10485759          (10MB)              │
│ Chunk 2: bytes=10485760-20971519   (10MB)              │
│ Chunk 3: bytes=20971520-31457279   (10MB)              │
│ ...                                                     │
│ Chunk 10: bytes=94371840-104857599 (10MB)              │
└─────────────────────────────────────────────────────────┘

Each chunk downloaded concurrently:

Connection 1 → Proxy → S3 (Chunk 1)
Connection 2 → Proxy → S3 (Chunk 2)
Connection 3 → Proxy → S3 (Chunk 3)
...
Connection 10 → Proxy → S3 (Chunk 10)

Client reassembles chunks → Complete file
```

---

## Sequence Diagram: Parallel Download

```
Client         Yatagarasu Proxy (10 workers)      S3 Backend
  |                     |                              |
  | HEAD /file.iso      |                              |
  | (Get file size)     |                              |
  |-------------------->|                              |
  |                     | HEAD /file.iso               |
  |                     |----------------------------->|
  |                     |                              |
  |     200 OK          |    200 OK                    |
  |     Content-Length: |    Content-Length: 104857600 |
  | <-------------------|<-----------------------------|
  |                     |                              |
  | [Split into 10 chunks of 10MB each]               |
  |                     |                              |
  | GET Range: 0-10MB   |                              |
  |-------------------->| GET Range: 0-10MB            |
  |                     |----------------------------->|
  |                     |                              |
  | GET Range: 10-20MB  |                              |
  |-------------------->| GET Range: 10-20MB           |
  |                     |----------------------------->|
  |                     |                              |
  | GET Range: 20-30MB  |                              |
  |-------------------->| GET Range: 20-30MB           |
  |                     |----------------------------->|
  |                     |                              |
  | ... (7 more concurrent connections)               |
  |                     |                              |
  | 206 Partial (Chunk 1)|  206 Partial                |
  |<--------------------|<-----------------------------|
  |                     |                              |
  | 206 Partial (Chunk 2)|  206 Partial                |
  |<--------------------|<-----------------------------|
  |                     |                              |
  | ... (all chunks received in parallel)             |
  |                     |                              |
  | [Reassemble chunks locally]                       |
  |                     |                              |

Performance:
- Single connection: 100MB in 10 seconds = 10MB/s
- 10 parallel connections: 100MB in 1-2 seconds = 50-100MB/s
- Speedup: 5-10x faster!
```

---

## Performance Characteristics

### Single Connection vs Parallel

| File Size | Connections | Bandwidth | Time | Notes |
|-----------|-------------|-----------|------|-------|
| 100MB | 1 | 10MB/s | 10s | Baseline |
| 100MB | 4 | 10MB/s | 2.5s | 4x speedup |
| 100MB | 8 | 10MB/s | 1.25s | 8x speedup |
| 100MB | 10 | 10MB/s | 1s | 10x speedup |
| 1GB | 1 | 10MB/s | 100s | Baseline |
| 1GB | 10 | 10MB/s | 10s | 10x speedup |

**Speedup = min(connections, bandwidth_ceiling / single_bandwidth)**

### Proxy Resource Usage

```
10 parallel range requests = 10 concurrent connections

Memory per connection: ~64KB
Total memory: 10 × 64KB = 640KB (constant!)

NOT: 10 × file_size
Example: 10 × 100MB ≠ 1GB memory usage
         10 × 64KB = 640KB memory usage ✅
```

---

## Client Examples

### 1. Using `aria2` (Download Accelerator)

```bash
# Download with 10 parallel connections
aria2c \
  --max-connection-per-server=10 \
  --min-split-size=10M \
  http://proxy/downloads/ubuntu.iso

# Output:
# [#1 SIZE:10.0MiB/100MiB CN:10 DL:50MiB]
# 10 connections, 5x faster than single connection
```

**Configuration:**
```bash
# ~/.aria2/aria2.conf
max-connection-per-server=10
min-split-size=10M
split=10
```

### 2. Using `curl` (Manual Parallel)

```bash
#!/bin/bash
# parallel-download.sh

URL="http://proxy/downloads/large-file.iso"
OUTPUT="large-file.iso"

# Get file size
SIZE=$(curl -sI "$URL" | grep -i Content-Length | awk '{print $2}' | tr -d '\r')
echo "File size: $SIZE bytes"

# Calculate chunk size (10 chunks)
CHUNKS=10
CHUNK_SIZE=$((SIZE / CHUNKS))

# Download chunks in parallel
for i in $(seq 0 $((CHUNKS - 1))); do
  START=$((i * CHUNK_SIZE))
  if [ $i -eq $((CHUNKS - 1)) ]; then
    END=$((SIZE - 1))
  else
    END=$((START + CHUNK_SIZE - 1))
  fi
  
  echo "Downloading chunk $i: bytes=$START-$END"
  curl -s -r "$START-$END" "$URL" > "$OUTPUT.part$i" &
done

# Wait for all downloads
wait

# Reassemble
cat $(ls -v "$OUTPUT".part*) > "$OUTPUT"
rm "$OUTPUT".part*

echo "Download complete: $OUTPUT"
```

**Usage:**
```bash
chmod +x parallel-download.sh
./parallel-download.sh

# Downloads 10 chunks simultaneously
# Reassembles into complete file
```

### 3. Using Python with `requests`

```python
#!/usr/bin/env python3
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed

URL = "http://proxy/downloads/large-file.iso"
OUTPUT = "large-file.iso"
CHUNKS = 10

def download_chunk(start, end, chunk_id):
    """Download a single chunk"""
    headers = {"Range": f"bytes={start}-{end}"}
    response = requests.get(URL, headers=headers, stream=True)
    
    with open(f"{OUTPUT}.part{chunk_id}", "wb") as f:
        for data in response.iter_content(chunk_size=65536):
            f.write(data)
    
    print(f"✓ Chunk {chunk_id} downloaded: {start}-{end}")
    return chunk_id

def main():
    # Get file size
    response = requests.head(URL)
    file_size = int(response.headers["Content-Length"])
    print(f"File size: {file_size} bytes")
    
    # Calculate chunks
    chunk_size = file_size // CHUNKS
    ranges = []
    for i in range(CHUNKS):
        start = i * chunk_size
        end = start + chunk_size - 1 if i < CHUNKS - 1 else file_size - 1
        ranges.append((start, end, i))
    
    # Download in parallel
    with ThreadPoolExecutor(max_workers=CHUNKS) as executor:
        futures = [executor.submit(download_chunk, start, end, i) 
                   for start, end, i in ranges]
        
        for future in as_completed(futures):
            future.result()
    
    # Reassemble
    print("Reassembling chunks...")
    with open(OUTPUT, "wb") as outfile:
        for i in range(CHUNKS):
            with open(f"{OUTPUT}.part{i}", "rb") as infile:
                outfile.write(infile.read())
            os.remove(f"{OUTPUT}.part{i}")
    
    print(f"✓ Download complete: {OUTPUT}")

if __name__ == "__main__":
    import os
    main()
```

**Usage:**
```bash
python3 parallel-download.py

# Output:
# File size: 104857600 bytes
# ✓ Chunk 0 downloaded: 0-10485759
# ✓ Chunk 3 downloaded: 31457280-41943039
# ✓ Chunk 1 downloaded: 10485760-20971519
# ...
# Reassembling chunks...
# ✓ Download complete: large-file.iso
```

### 4. Using `wget` (Limited Parallel Support)

```bash
# wget doesn't support parallel chunks natively
# But you can use multiple wget instances

#!/bin/bash
FILE="http://proxy/downloads/file.iso"
SIZE=$(curl -sI "$FILE" | grep -i Content-Length | awk '{print $2}' | tr -d '\r')
CHUNKS=4
CHUNK_SIZE=$((SIZE / CHUNKS))

for i in $(seq 0 $((CHUNKS - 1))); do
  START=$((i * CHUNK_SIZE))
  END=$((i < CHUNKS - 1 ? START + CHUNK_SIZE - 1 : SIZE - 1))
  wget --header="Range: bytes=$START-$END" "$FILE" -O "file.part$i" &
done

wait
cat file.part* > file.iso
rm file.part*
```

### 5. Browser with Custom JavaScript

```javascript
// parallel-download.js
async function parallelDownload(url, chunks = 10) {
    // Get file size
    const response = await fetch(url, { method: 'HEAD' });
    const fileSize = parseInt(response.headers.get('Content-Length'));
    const chunkSize = Math.ceil(fileSize / chunks);
    
    console.log(`Downloading ${fileSize} bytes in ${chunks} chunks...`);
    
    // Download chunks in parallel
    const chunkPromises = [];
    for (let i = 0; i < chunks; i++) {
        const start = i * chunkSize;
        const end = Math.min(start + chunkSize - 1, fileSize - 1);
        
        const promise = fetch(url, {
            headers: { 'Range': `bytes=${start}-${end}` }
        }).then(res => res.arrayBuffer());
        
        chunkPromises.push(promise);
    }
    
    // Wait for all chunks
    const chunkArrays = await Promise.all(chunkPromises);
    
    // Combine chunks
    const totalLength = chunkArrays.reduce((acc, arr) => acc + arr.byteLength, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    
    for (const chunk of chunkArrays) {
        combined.set(new Uint8Array(chunk), offset);
        offset += chunk.byteLength;
    }
    
    console.log('Download complete!');
    return combined;
}

// Usage
parallelDownload('http://proxy/downloads/file.iso', 10)
    .then(data => {
        // Create download link
        const blob = new Blob([data]);
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'file.iso';
        a.click();
    });
```

---

## Proxy Behavior

### Independent Connections

Each range request is handled **independently**:

```
Connection 1: bytes=0-10MB
  - Separate TCP connection
  - Separate S3 request
  - Independent streaming
  - Memory: 64KB buffer

Connection 2: bytes=10-20MB
  - Separate TCP connection
  - Separate S3 request
  - Independent streaming
  - Memory: 64KB buffer

...

Total memory: N × 64KB (NOT N × chunk_size)
```

### No Special Configuration Needed

```yaml
# Range requests work automatically
buckets:
  - name: "downloads"
    path_prefix: "/downloads"
    s3:
      bucket: "my-downloads"
      region: "us-east-1"
    # No special config for parallel downloads!
```

### Authentication Still Enforced

```bash
# Each parallel connection needs JWT
for i in {1..10}; do
  curl -H "Authorization: Bearer $TOKEN" \
       -H "Range: bytes=$START-$END" \
       http://proxy/private/file.iso &
done
```

---

## Performance Analysis

### Bandwidth Utilization

```
Single connection: Uses ~10% of available bandwidth
Parallel connections: Can saturate available bandwidth

Example:
- Link capacity: 100MB/s
- Single connection: Limited by TCP window, latency
- 10 parallel connections: Can reach 80-100MB/s
```

### Optimal Number of Connections

```
Optimal connections ≈ (Bandwidth × RTT) / (TCP Window)

Typical values:
- Low latency (10ms): 4-6 connections
- Medium latency (50ms): 8-12 connections
- High latency (200ms): 16-32 connections

Diminishing returns beyond:
- Proxy can handle 100s of concurrent connections
- But client limited by CPU/network stack
```

### S3 Cost Impact

```
Single download: 1 GET request + 100MB transfer
Parallel download (10 chunks): 10 GET requests + 100MB transfer

S3 GET cost: $0.0004 per 1000 requests
Extra cost: 9 additional GETs = $0.0000036

Transfer cost: Same (100MB regardless)
$0.09/GB × 0.1GB = $0.009

Total extra cost: $0.0000036 (negligible!)
Benefit: 10x faster download ✅
```

---

## Use Cases

### 1. Large Software Distribution

```bash
# Download 4GB Linux ISO
aria2c \
  --max-connection-per-server=16 \
  --min-split-size=256M \
  http://proxy/downloads/ubuntu-24.04.iso

# Result: 
# Single connection: 400 seconds (10MB/s)
# 16 connections: 40 seconds (100MB/s)
# 10x faster! 🚀
```

### 2. Video Game Downloads

```python
# Steam-like parallel download
download_parallel(
    url="http://proxy/games/cyberpunk-50gb.bin",
    chunks=20,
    output="game.bin"
)

# 50GB in 10 minutes instead of 100 minutes
```

### 3. Backup/Restore Operations

```bash
# Download database backup (10GB)
./parallel-download.sh \
  http://proxy/backups/db-backup-20251021.tar.gz

# Restore faster to minimize downtime
```

### 4. CDN Mirror Sync

```python
# Sync large files to CDN edge nodes
for file in large_files:
    download_parallel(file, chunks=32)
    
# Faster mirror synchronization
```

---

## Limitations and Considerations

### 1. Connection Limits

**Client-side:**
```
Browsers: 6-8 connections per domain (HTTP/1.1)
aria2: Configurable (default 1, max ~16 practical)
Custom clients: Limited by OS (thousands possible)
```

**Proxy-side:**
```
Yatagarasu: No per-client connection limit
Can handle 1000s of concurrent connections
Memory scales linearly: connections × 64KB
```

### 2. S3 Rate Limits

```
S3 GET requests: 5,500 per second per prefix
Parallel downloads unlikely to hit this limit

Example:
100 clients × 10 connections = 1,000 concurrent requests
Still well below S3 limits ✅
```

### 3. File Integrity

**Important:** Client must verify file integrity after reassembly

```bash
# Calculate checksum
md5sum file.iso

# Or use SHA-256
sha256sum file.iso

# Compare with expected hash
```

**Best practice:** Provide checksums in API response:

```bash
curl -I http://proxy/downloads/file.iso

# Response:
HTTP/1.1 200 OK
Content-Length: 104857600
ETag: "abc123def456"
X-Content-SHA256: "7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a..."
```

### 4. Partial Failure Handling

```python
def download_with_retry(start, end, chunk_id, max_retries=3):
    for attempt in range(max_retries):
        try:
            download_chunk(start, end, chunk_id)
            return True
        except Exception as e:
            print(f"Chunk {chunk_id} failed (attempt {attempt+1})")
            if attempt == max_retries - 1:
                raise
    return False
```

---

## Monitoring

### Metrics to Track

```
# Proxy metrics
yatagarasu_concurrent_connections{bucket="downloads"}
yatagarasu_range_requests_total{bucket="downloads"}
yatagarasu_bytes_transferred{bucket="downloads"}

# Per-file metrics
yatagarasu_active_downloads{file="large.iso"}
yatagarasu_parallel_chunks{file="large.iso"}
```

### Logs

```json
{
  "level": "info",
  "message": "Range request",
  "client": "192.168.1.100",
  "file": "/downloads/ubuntu.iso",
  "range": "bytes=0-10485759",
  "chunk": "1/10",
  "connection_id": "conn-123"
}
```

---

## Testing

### Test Parallel Download Functionality

```bash
# Test script
#!/bin/bash

URL="http://proxy/test/10mb.bin"
CHUNKS=4

echo "Testing parallel download with $CHUNKS chunks..."

# Download chunks
for i in $(seq 0 $((CHUNKS - 1))); do
  START=$((i * 2621440))  # 2.5MB per chunk
  END=$((START + 2621439))
  
  curl -s -r "$START-$END" "$URL" > "chunk.$i" &
done

wait

# Verify
cat chunk.* > downloaded.bin
ORIGINAL_MD5=$(curl -s "$URL" | md5sum | cut -d' ' -f1)
DOWNLOAD_MD5=$(md5sum downloaded.bin | cut -d' ' -f1)

if [ "$ORIGINAL_MD5" == "$DOWNLOAD_MD5" ]; then
  echo "✓ Parallel download test PASSED"
else
  echo "✗ Parallel download test FAILED"
fi

rm chunk.* downloaded.bin
```

---

## Summary

| Aspect | Details |
|--------|---------|
| **Supported?** | ✅ YES - Full HTTP Range support |
| **Configuration** | ✅ None needed - works automatically |
| **Connections** | ✅ Unlimited concurrent range requests |
| **Memory** | ✅ Constant: connections × 64KB |
| **Performance** | ✅ 5-10x speedup typical |
| **Authentication** | ✅ JWT required per connection |
| **Caching** | ⚠️ Range requests NOT cached |
| **S3 Cost** | ✅ Negligible extra cost |

### Key Benefits

✅ **Faster downloads** - 5-10x speedup for large files  
✅ **Better bandwidth utilization** - Saturate available bandwidth  
✅ **Resume capability** - Download specific missing chunks  
✅ **Scalable** - Proxy handles 1000s of concurrent connections  
✅ **Standard HTTP** - Works with any Range-compatible client  

### Best Practices

1. ✅ Use 4-16 connections for most files
2. ✅ Verify checksums after reassembly
3. ✅ Handle partial failures with retries
4. ✅ Monitor concurrent connections
5. ✅ Adjust chunk size based on file size (1-10MB per chunk)

---

**Bottom Line:** Yatagarasu fully supports parallel downloads via Range requests with **no special configuration needed**. Use tools like `aria2` or custom scripts to download large files 5-10x faster! 🚀
