---
title: Image Optimization
layout: default
parent: Tutorials
nav_order: 7
---

# Image Optimization Tutorial

Resize, crop, and optimize images on-the-fly.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Enable image optimization for a bucket
- Resize and crop images via URL parameters
- Convert between formats (JPEG, WebP, AVIF)
- Use auto-format selection for optimal delivery
- Secure image URLs with signatures

## Prerequisites

- Completed the [Basic Proxy Setup](/yatagarasu/tutorials/basic-proxy/) tutorial
- Docker installed

---

## Step 1: Setup

Create a tutorial directory:

```bash
mkdir image-optimization-tutorial && cd image-optimization-tutorial
```

Create `docker-compose.yml`:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:latest
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
    depends_on:
      minio:
        condition: service_healthy

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 3

  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set local http://minio:9000 minioadmin minioadmin;
      mc mb local/images --ignore-existing;
      curl -sL https://picsum.photos/2000/1500 | mc pipe local/images/photo.jpg;
      curl -sL https://picsum.photos/1000/1000 | mc pipe local/images/square.jpg;
      echo 'Test images uploaded!';
      "
```

---

## Step 2: Configure Image Optimization

Create `config.yaml`:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "images"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

    image_optimization:
      enabled: true
      max_width: 4096
      max_height: 4096
      quality: 85
      auto_format: true
      auto_rotate: true

cache:
  memory:
    max_capacity: 104857600 # 100MB
    ttl_seconds: 3600

metrics:
  enabled: true
  port: 9090
```

---

## Step 3: Start and Test

```bash
docker compose up -d
sleep 10  # Wait for images to upload

# Get original image
curl -s http://localhost:8080/img/photo.jpg -o original.jpg
ls -la original.jpg
# ~200KB+ original

# Resize to 800px width
curl -s "http://localhost:8080/img/photo.jpg?w=800" -o resized.jpg
ls -la resized.jpg
# Smaller file, 800px wide
```

---

## Step 4: Resize Operations

```bash
# Fixed width (auto height)
curl -s "http://localhost:8080/img/photo.jpg?w=400" -o w400.jpg

# Fixed height (auto width)
curl -s "http://localhost:8080/img/photo.jpg?h=300" -o h300.jpg

# Both dimensions with fit mode
curl -s "http://localhost:8080/img/photo.jpg?w=400&h=400&fit=cover" -o cover.jpg

# Retina display (2x DPR)
curl -s "http://localhost:8080/img/photo.jpg?w=200&dpr=2" -o retina.jpg
# Results in 400px wide image
```

---

## Step 5: Format Conversion

```bash
# Convert to WebP
curl -s "http://localhost:8080/img/photo.jpg?w=800&fmt=webp" -o photo.webp
ls -la photo.webp
# Typically 30-50% smaller than JPEG

# Convert to AVIF (best compression)
curl -s "http://localhost:8080/img/photo.jpg?w=800&fmt=avif" -o photo.avif
ls -la photo.avif
# Typically 50-70% smaller than JPEG

# Auto-format based on Accept header
curl -s -H "Accept: image/webp" \
  "http://localhost:8080/img/photo.jpg?w=800&fmt=auto" -o auto.webp
```

---

## Step 6: Cropping

```bash
# Cover crop (fills dimensions, crops excess)
curl -s "http://localhost:8080/img/photo.jpg?w=400&h=400&fit=cover" -o cover.jpg

# Crop with gravity
curl -s "http://localhost:8080/img/photo.jpg?w=400&h=400&fit=cover&g=north" -o north.jpg

# Smart crop (entropy-based)
curl -s "http://localhost:8080/img/photo.jpg?w=400&h=400&fit=cover&g=smart" -o smart.jpg

# Manual crop region
curl -s "http://localhost:8080/img/photo.jpg?cx=100&cy=50&cw=500&ch=400" -o manual.jpg
```

---

## Step 7: View Metrics

```bash
curl -s http://localhost:9090/metrics | grep image
```

Key metrics:

- `yatagarasu_image_processing_total` - Total operations
- `yatagarasu_image_bytes_saved_total` - Bytes saved
- `yatagarasu_image_cache_hits_total` - Cache hit rate

---

## Step 8: Rotation & Flip

```bash
# Rotate 90° clockwise
curl -s "http://localhost:8080/img/photo.jpg?w=400&rot=90" -o rotated.jpg

# Flip horizontally
curl -s "http://localhost:8080/img/photo.jpg?w=400&flip=h" -o flipped.jpg

# EXIF auto-rotation (enabled by default)
# Disable with auto_rotate=0
curl -s "http://localhost:8080/img/photo.jpg?w=400&auto_rotate=0" -o no-rotate.jpg
```

---

## Step 9: Quality Optimization

```bash
# Low quality (smaller file)
curl -s "http://localhost:8080/img/photo.jpg?w=800&q=60" -o q60.jpg

# High quality (larger file)
curl -s "http://localhost:8080/img/photo.jpg?w=800&q=95" -o q95.jpg

# Compare file sizes
ls -la q60.jpg q95.jpg
```

---

## Cleanup

```bash
docker compose down -v
cd .. && rm -rf image-optimization-tutorial
```

---

## Common Use Cases

### Responsive Images

```html
<img
  srcset="
    /img/photo.jpg?w=400   400w,
    /img/photo.jpg?w=800   800w,
    /img/photo.jpg?w=1200 1200w
  "
  sizes="(max-width: 600px) 400px, (max-width: 1200px) 800px, 1200px"
  src="/img/photo.jpg?w=800"
  alt="Responsive image"
/>
```

### Thumbnail Gallery

```html
<img src="/img/photo.jpg?w=150&h=150&fit=cover" alt="Thumbnail" />
```

### Modern Format with Fallback

```html
<picture>
  <source srcset="/img/photo.jpg?w=800&fmt=avif" type="image/avif" />
  <source srcset="/img/photo.jpg?w=800&fmt=webp" type="image/webp" />
  <img src="/img/photo.jpg?w=800" alt="Photo" />
</picture>
```

### Auto-Format (Recommended)

```html
<!-- Server selects best format based on Accept header -->
<img src="/img/photo.jpg?w=800&fmt=auto" alt="Auto-format" />
```

---

## Best Practices

1. **Use `fmt=auto`** - Let the server choose optimal format
2. **Set reasonable max dimensions** - Prevent abuse with `max_width`/`max_height`
3. **Enable caching** - Processed variants are cached automatically
4. **Use quality 80-85** - Good balance of size and quality
5. **Consider AVIF** - Best compression but slower encoding

---

## Next Steps

- [Image Parameters Reference](/yatagarasu/reference/image-parameters/) - All parameters
- [Image Configuration](/yatagarasu/configuration/image-optimization/) - Full config reference
- [Caching Tutorial](/yatagarasu/tutorials/caching/) - Optimize cache for images
