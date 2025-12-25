---
title: Watermarks
layout: default
parent: Configuration
nav_order: 8
---

# Watermarks

Server-side watermarking for image protection and branding.
{: .fs-6 .fw-300 }

---

## Overview

Yatagarasu supports server-side watermarking of images, allowing you to apply text or image watermarks during image processing. Watermarks are enforced at the proxy level and cannot be bypassed by clients.

**Features:**
- Text watermarks with configurable font, size, color, and opacity
- Image watermarks from HTTPS URLs
- Template variables for dynamic text (JWT claims, IP, dates)
- 11 positioning modes: 9-grid corners/centers, tiled, diagonal band
- Per-bucket configuration with glob pattern path matching
- LRU caching for watermark images

---

## Configuration

Watermarks are configured per-bucket with pattern-based rules:

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
        - pattern: "**/*.jpg"
          watermarks:
            - type: text
              text: "Preview - {{date}}"
              font_size: 24
              color: "#FF0000"
              opacity: 0.5
              position: bottom-right
              margin: 20
```

### Configuration Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `enabled` | boolean | `false` | Enable watermarking for this bucket |
| `cache_ttl_seconds` | integer | `3600` | TTL for cached watermark images |
| `rules` | array | `[]` | List of pattern-based watermark rules |

---

## Watermark Types

### Text Watermarks

Render text directly onto images with full styling control.

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `text` | string | required | Text to render (supports template variables) |
| `font_size` | integer | `24` | Font size in pixels |
| `color` | string | `#FFFFFF` | Hex color (#RGB or #RRGGBB) |
| `opacity` | float | `0.5` | Opacity from 0.0 to 1.0 |
| `position` | string | required | Positioning mode |
| `margin` | integer | `10` | Margin from edges in pixels |
| `rotation` | integer | `0` | Rotation angle in degrees |
| `tile_spacing` | integer | `100` | Spacing between tiles (for tiled position) |

```yaml
- type: text
  text: "Copyright {{jwt.org}}"
  font_size: 18
  color: "#FFFFFF"
  opacity: 0.5
  position: bottom-right
  margin: 15
```

---

### Image Watermarks

Overlay an image (logo, badge) onto the target image.

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `source` | string | required | HTTPS URL to watermark image |
| `width` | integer | - | Resize width (preserves aspect ratio) |
| `height` | integer | - | Resize height (preserves aspect ratio) |
| `opacity` | float | `0.5` | Opacity from 0.0 to 1.0 |
| `position` | string | required | Positioning mode |
| `margin` | integer | `10` | Margin from edges |

```yaml
- type: image
  source: "https://cdn.example.com/logo.png"
  width: 100
  opacity: 0.7
  position: top-left
  margin: 15
```

---

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

---

### Tiled

Repeats the watermark in a grid pattern across the entire image:

```yaml
- type: text
  text: "SAMPLE"
  font_size: 48
  color: "#888888"
  opacity: 0.15
  position: tiled
  tile_spacing: 150
  rotation: -30
```

---

### Diagonal Band

Renders watermarks in a diagonal stripe pattern:

```yaml
- type: text
  text: "PREVIEW ONLY"
  font_size: 36
  color: "#FF6600"
  opacity: 0.25
  position: diagonal-band
```

---

## Template Variables

Dynamic values can be inserted into text watermarks:

| Variable | Example Output | Description |
|:---------|:---------------|:------------|
| `{{jwt.sub}}` | `user@example.com` | JWT subject claim |
| `{{jwt.iss}}` | `auth.example.com` | JWT issuer claim |
| `{{jwt.<claim>}}` | `Acme Inc` | Custom JWT claim (e.g., `{{jwt.org}}`) |
| `{{ip}}` | `192.168.1.100` | Client IP address |
| `{{header.X-Name}}` | header value | Request header value |
| `{{path}}` | `/preview/photo.jpg` | Request path |
| `{{bucket}}` | `previews` | Bucket name |
| `{{date}}` | `2025-12-25` | Current date |
| `{{datetime}}` | `2025-12-25T14:30:00Z` | ISO 8601 datetime |
| `{{timestamp}}` | `1735134600` | Unix timestamp |

---

## Pattern Matching

Rules are matched against the **full request path** using glob patterns.

{: .important }
Use `**` to match paths with directories. Single `*` only matches within a single path segment.

| Pattern | Matches |
|:--------|:--------|
| `**/*.jpg` | All JPEG files in any path |
| `**/*.png` | All PNG files in any path |
| `**/previews/*.jpg` | JPEGs in any `/previews/` directory |
| `**` | All files (default fallback) |

Rules are evaluated in order; **first match wins**.

### Example: Multiple Rules by File Type

```yaml
watermark:
  enabled: true
  rules:
    # JPEG files get corner watermark
    - pattern: "**/*.jpg"
      watermarks:
        - type: text
          text: "PREVIEW - {{date}}"
          font_size: 32
          color: "#FF0000"
          opacity: 0.5
          position: bottom-right
          margin: 20

    # PNG files get tiled watermark
    - pattern: "**/*.png"
      watermarks:
        - type: text
          text: "SAMPLE"
          font_size: 48
          color: "#888888"
          opacity: 0.2
          position: tiled
          tile_spacing: 150
          rotation: -30

    # All other files get diagonal band
    - pattern: "**"
      watermarks:
        - type: text
          text: "PREVIEW ONLY"
          font_size: 36
          color: "#FF6600"
          opacity: 0.3
          position: diagonal-band
```

---

## Common Use Cases

### Copyright Protection

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "**"
      watermarks:
        - type: text
          text: "Copyright {{jwt.org}} {{date}}"
          font_size: 24
          color: "#FFFFFF"
          opacity: 0.4
          position: bottom-right
          margin: 15
```

---

### Preview Images with Tiled Watermark

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "**"
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

---

### User Tracking for Documents

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "**"
      watermarks:
        - type: text
          text: "Licensed to: {{jwt.sub}}"
          font_size: 14
          color: "#333333"
          opacity: 0.5
          position: bottom-center
          margin: 10
        - type: text
          text: "Downloaded: {{datetime}} from {{ip}}"
          font_size: 10
          color: "#666666"
          opacity: 0.3
          position: top-right
          margin: 5
```

---

### Logo Watermark

```yaml
watermark:
  enabled: true
  rules:
    - pattern: "**"
      watermarks:
        - type: image
          source: "https://cdn.example.com/logo.png"
          width: 120
          opacity: 0.6
          position: bottom-right
          margin: 20
```

---

## Processing Pipeline

Watermarks are applied after image effects and before encoding:

```
1. Decode image
2. EXIF auto-rotation
3. Crop/Resize/Rotate/Flip
4. Effects (blur, sharpen, brightness, contrast)
5. *** APPLY WATERMARKS ***
6. Encode to output format
```

{: .note }
Watermarks are only applied when image processing is requested (e.g., `?w=800`). Requests without processing parameters serve the original image without watermarks.

---

## Caching

- Watermark images from URLs are cached in memory (LRU)
- Cache TTL is configurable via `cache_ttl_seconds`
- Processed images with watermarks are cached with a key that includes the resolved watermark text

---

## See Also

- [Image Optimization](/yatagarasu/configuration/image-optimization/)
- [Authentication](/yatagarasu/configuration/authentication/)
