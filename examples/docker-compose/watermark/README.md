# Watermarking Example

Demonstrates Yatagarasu's server-side watermarking feature with text watermarks, template variables, and multiple positioning modes.

## Features Demonstrated

- **Text watermarks** with configurable font, color, opacity
- **Template variables**: `{{date}}`, `{{jwt.sub}}`, `{{ip}}`, `{{datetime}}`
- **Positioning modes**: bottom-right, bottom-left, tiled, diagonal-band
- **Pattern matching**: Different watermarks for `.jpg`, `.png`, and other files
- **Per-bucket configuration**: Public (no watermark), preview (watermarked), premium (auth required)

## Prerequisites

- Docker and Docker Compose installed
- Ports 8080, 9000, 9001, 9090 available
- Internet connection (to download sample images)

## Quick Start

```bash
# Start the services
docker compose up -d

# Wait for services to be ready (about 30 seconds)
until [ "$(curl -s -o /dev/null -w '%{http_code}' http://localhost:8080/health)" == "200" ]; do
  echo "Waiting for services..."; sleep 2
done
echo "Ready!"

# Compare watermarked vs non-watermarked images
curl "http://localhost:8080/public/nature.jpg?w=800" -o public.jpg
curl "http://localhost:8080/preview/nature.jpg?w=800" -o preview.jpg

# Open both images to compare
open public.jpg preview.jpg  # macOS
# xdg-open public.jpg preview.jpg  # Linux
```

## Bucket Configuration

| Path Prefix | Bucket | Auth | Watermark | Description |
|-------------|--------|------|-----------|-------------|
| `/public` | public-assets | No | No | Public images, no restrictions |
| `/premium` | premium-assets | Yes | No | Premium content for subscribers |
| `/preview` | premium-assets | No | Yes | Preview of premium content |
| `/docs` | documents | Yes | Yes | Documents with user tracking |

## Watermark Patterns

### Preview Images (`/preview/*`)

| Pattern | Watermark Style | Description |
|---------|-----------------|-------------|
| `*.jpg` | Bottom-right + Bottom-left | "PREVIEW - 2025-12-25" + site URL |
| `*.png` | Tiled at -30° | "SAMPLE" repeated across image |
| `*` (default) | Diagonal band | "PREVIEW ONLY" diagonal stripe |

### Documents (`/docs/*`)

All documents get user tracking watermarks:
- Bottom-center: "Licensed to: {username}"
- Top-right: "Downloaded: {datetime} from {ip}"

## Testing Different Watermarks

```bash
# JPG watermark (bottom-right text)
curl "http://localhost:8080/preview/nature.jpg?w=800" -o watermark-jpg.jpg

# PNG watermark (tiled pattern) - if you have a PNG
curl "http://localhost:8080/preview/abstract.png?w=600" -o watermark-png.png

# Document with user tracking (requires JWT)
TOKEN=$(python3 -c "
import jwt, time
print(jwt.encode({'sub': 'alice@example.com', 'exp': int(time.time()) + 3600}, 'super-secret-key-for-demo-only', algorithm='HS256'))
")
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/docs/report-cover.jpg?w=800" -o document.jpg
```

## Template Variables

The watermark text can include dynamic values:

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `{{date}}` | Current date | `2025-12-25` |
| `{{datetime}}` | ISO 8601 datetime | `2025-12-25T14:30:00Z` |
| `{{timestamp}}` | Unix timestamp | `1735134600` |
| `{{ip}}` | Client IP address | `192.168.1.100` |
| `{{jwt.sub}}` | JWT subject claim | `alice@example.com` |
| `{{jwt.*}}` | Any JWT claim | Custom claim value |
| `{{header.X-Name}}` | Request header | Header value |
| `{{path}}` | Request path | `/preview/nature.jpg` |
| `{{bucket}}` | Bucket name | `premium-assets` |

## Configuration Highlights

```yaml
# Text watermark with positioning
watermark:
  enabled: true
  rules:
    - pattern: "*.jpg"
      watermarks:
        - type: text
          text: "PREVIEW - {{date}}"
          font_size: 32
          color: "#FF0000"
          opacity: 0.4
          position: bottom-right
          margin: 20

# Tiled watermark with rotation
    - pattern: "*.png"
      watermarks:
        - type: text
          text: "SAMPLE"
          font_size: 36
          color: "#888888"
          opacity: 0.15
          position: tiled
          tile_spacing: 150
          rotation: -30
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| yatagarasu | 8080 | S3 Proxy with watermarking |
| yatagarasu | 9090 | Prometheus metrics |
| minio | 9000 | S3 API |
| minio | 9001 | MinIO Console |

## MinIO Console

Access the MinIO web console at http://localhost:9001

- **Username**: minioadmin
- **Password**: minioadmin

## Cleanup

```bash
# Stop and remove containers
docker compose down

# Also remove volumes (deletes all data)
docker compose down -v
```

## Troubleshooting

**Images not downloading?**
- The example tries to download from picsum.photos
- If that fails, it creates placeholder files
- You can manually upload images to MinIO console

**Watermarks not appearing?**
- Ensure you're using the `/preview` path, not `/public`
- Check that image optimization is requested (`?w=800`)
- Watermarks only apply during image processing

**JWT authentication failing?**
- Use the Python snippet above to generate a valid token
- Ensure the secret matches: `super-secret-key-for-demo-only`
- Check token expiration

## Next Steps

- Try [simple example](../simple/) for basic setup
- Try [full-stack example](../full-stack/) for OPA/OpenFGA authorization
- See [watermark documentation](../../../docs/WATERMARKS.md) for full configuration reference
