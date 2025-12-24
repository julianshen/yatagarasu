---
title: Watermarks
parent: Configuration
nav_order: 8
description: "Server-side watermarking for images"
---

# Watermarks

Yatagarasu supports server-side watermarking of images, allowing you to apply text or image watermarks during image processing. Watermarks are enforced at the proxy level and cannot be bypassed by clients.

## Features

- **Text watermarks** with configurable font, size, color, and opacity
- **Image watermarks** from HTTPS URLs
- **Template variables** for dynamic text (JWT claims, IP, dates)
- **11 positioning modes**: 9-grid corners/centers, tiled, diagonal band
- **Per-bucket configuration** with glob pattern path matching
- **LRU caching** for watermark images

## Basic Configuration

```yaml
buckets:
  - name: previews
    path_prefix: /preview
    s3:
      bucket: my-bucket
      region: us-east-1
    watermark:
      enabled: true
      cache_ttl_seconds: 3600
      rules:
        - pattern: "*.jpg"
          watermarks:
            - type: text
              text: "Preview - {{date}}"
              font_size: 24
              color: "#FF0000"
              opacity: 0.4
              position: bottom-right
              margin: 20
```

## Watermark Types

### Text Watermarks

Render text directly onto images with full styling control.

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `text` | string | required | Text to render (supports template variables) |
| `font_size` | integer | 24 | Font size in pixels |
| `color` | string | "#FFFFFF" | Hex color (#RGB or #RRGGBB) |
| `opacity` | float | 0.5 | Opacity from 0.0 to 1.0 |
| `position` | string | required | Positioning mode |
| `margin` | integer | 10 | Margin from edges in pixels |
| `rotation` | integer | 0 | Rotation angle in degrees |
| `tile_spacing` | integer | 100 | Spacing between tiles (for tiled position) |

```yaml
- type: text
  text: "Copyright {{jwt.org}}"
  font_size: 18
  color: "#FFFFFF"
  opacity: 0.5
  position: bottom-right
```

### Image Watermarks

Overlay an image (logo, badge) onto the target image.

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `source` | string | required | HTTPS URL to watermark image |
| `width` | integer | - | Resize width (preserves aspect ratio) |
| `height` | integer | - | Resize height (preserves aspect ratio) |
| `opacity` | float | 0.5 | Opacity from 0.0 to 1.0 |
| `position` | string | required | Positioning mode |
| `margin` | integer | 10 | Margin from edges |

```yaml
- type: image
  source: "https://cdn.example.com/logo.png"
  width: 100
  opacity: 0.7
  position: top-left
```

## Positioning Modes

### 9-Grid Positions

Standard corner and center positions:

| Position | Description |
|:---------|:------------|
| `top-left` | Top-left corner |
| `top-center` | Top center |
| `top-right` | Top-right corner |
| `center-left` | Middle left |
| `center` | Center of image |
| `center-right` | Middle right |
| `bottom-left` | Bottom-left corner |
| `bottom-center` | Bottom center |
| `bottom-right` | Bottom-right corner |

### Tiled

Repeats the watermark in a grid pattern:

```yaml
- type: text
  text: "SAMPLE"
  position: tiled
  tile_spacing: 150
  rotation: -30
  opacity: 0.15
```

### Diagonal Band

Renders watermarks in a diagonal stripe:

```yaml
- type: text
  text: "PREVIEW ONLY"
  position: diagonal-band
  rotation: -45
  opacity: 0.25
```

## Template Variables

Dynamic values can be inserted into text watermarks:

| Variable | Example Output | Description |
|:---------|:---------------|:------------|
| `{{jwt.sub}}` | `user@example.com` | JWT subject claim |
| `{{jwt.iss}}` | `auth.example.com` | JWT issuer claim |
| `{{jwt.<claim>}}` | `Acme Inc` | Custom JWT claim |
| `{{ip}}` | `192.168.1.100` | Client IP address |
| `{{header.X-Name}}` | header value | Request header |
| `{{path}}` | `/preview/photo.jpg` | Request path |
| `{{bucket}}` | `previews` | Bucket name |
| `{{date}}` | `2025-12-25` | Current date |
| `{{datetime}}` | `2025-12-25T14:30:00Z` | ISO 8601 datetime |
| `{{timestamp}}` | `1735134600` | Unix timestamp |

## Pattern Matching

Rules are matched against request paths using glob patterns:

| Pattern | Matches |
|:--------|:--------|
| `*.jpg` | All JPEG files |
| `*.png` | All PNG files |
| `/products/*` | Files directly under /products/ |
| `/preview/**/*.jpg` | JPEGs in any subdirectory of /preview/ |
| `*` | All files (default fallback) |

Rules are evaluated in order; **first match wins**.

## Common Use Cases

### Copyright Protection

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "*"
      watermarks:
        - type: text
          text: "Copyright {{jwt.org}} {{date}}"
          position: bottom-right
          opacity: 0.4
          margin: 15
```

### Preview Images with Tiled Watermark

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "*"
      watermarks:
        - type: text
          text: "PREVIEW"
          font_size: 48
          color: "#888888"
          opacity: 0.15
          position: tiled
          tile_spacing: 200
          rotation: -45
```

### User Tracking for Documents

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "*"
      watermarks:
        - type: text
          text: "Licensed to: {{jwt.sub}}"
          font_size: 12
          position: bottom-center
          opacity: 0.5
        - type: text
          text: "Downloaded: {{datetime}} from {{ip}}"
          font_size: 10
          position: top-right
          opacity: 0.3
```

### Different Watermarks by File Type

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "*.jpg"
      watermarks:
        - type: text
          text: "PREVIEW"
          position: bottom-right

    - pattern: "*.png"
      watermarks:
        - type: text
          text: "SAMPLE"
          position: tiled
          rotation: -30

    - pattern: "*"
      watermarks:
        - type: text
          text: "PREVIEW ONLY"
          position: diagonal-band
```

## Processing Pipeline

Watermarks are applied at a specific point in the image processing pipeline:

```
1. Decode image
2. EXIF auto-rotation
3. Crop/Resize/Rotate/Flip
4. Effects (blur, sharpen, brightness, contrast)
5. *** APPLY WATERMARKS ***
6. Encode to output format
```

{: .note }
Watermarks only apply when image processing is requested (e.g., `?w=800`). Original images without processing parameters are served without watermarks.

## Caching

- Watermark images from URLs are cached in memory (LRU)
- Cache TTL is configurable via `cache_ttl_seconds`
- Processed images with watermarks are cached separately from originals
