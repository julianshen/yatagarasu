# Watermarking

Yatagarasu supports server-side watermarking of images, allowing you to apply text or image watermarks during image processing. Watermarks are enforced at the proxy level and cannot be bypassed by clients.

## Features

- **Text watermarks** with configurable font, size, color, and opacity
- **Image watermarks** from HTTPS URLs
- **Template variables** for dynamic text (JWT claims, IP, dates)
- **11 positioning modes**: 9-grid corners/centers, tiled, diagonal band
- **Per-bucket configuration** with glob pattern path matching
- **LRU caching** for watermark images

## Configuration

Watermarks are configured per-bucket using the `watermark` section:

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

## Text Watermarks

Text watermarks render text directly onto images.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `text` | string | required | Text to render (supports template variables) |
| `font_size` | integer | 24 | Font size in pixels |
| `color` | string | "#FFFFFF" | Hex color (#RGB or #RRGGBB) |
| `opacity` | float | 0.5 | Opacity from 0.0 (transparent) to 1.0 (opaque) |
| `position` | string | required | Positioning mode (see below) |
| `margin` | integer | 10 | Margin from edges in pixels |
| `rotation` | integer | 0 | Rotation angle in degrees (for tiled/diagonal) |
| `tile_spacing` | integer | 100 | Spacing between tiles (for tiled position) |

### Example

```yaml
- type: text
  text: "Copyright {{jwt.org}} - {{date}}"
  font_size: 18
  color: "#FFFFFF"
  opacity: 0.5
  position: bottom-right
  margin: 15
```

## Image Watermarks

Image watermarks overlay an image onto the target.

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `source` | string | required | URL to watermark image (HTTPS only) |
| `width` | integer | - | Resize width (aspect ratio preserved) |
| `height` | integer | - | Resize height (aspect ratio preserved) |
| `opacity` | float | 0.5 | Opacity from 0.0 to 1.0 |
| `position` | string | required | Positioning mode |
| `margin` | integer | 10 | Margin from edges in pixels |

### Example

```yaml
- type: image
  source: "https://cdn.example.com/logo.png"
  width: 100
  opacity: 0.7
  position: top-left
  margin: 15
```

## Positioning Modes

### 9-Grid Positions

Standard corner and center positions:

```
+------------+------------+------------+
| top-left   | top-center | top-right  |
+------------+------------+------------+
|center-left |   center   |center-right|
+------------+------------+------------+
|bottom-left |bottom-center|bottom-right|
+------------+------------+------------+
```

### Tiled

Repeats the watermark in a grid pattern across the entire image:

```yaml
position: tiled
tile_spacing: 150   # pixels between tiles
rotation: -30       # rotate each tile
```

### Diagonal Band

Renders watermarks in a diagonal stripe across the image:

```yaml
position: diagonal-band
rotation: -45
```

## Template Variables

Text watermarks support dynamic variable substitution:

| Variable | Description | Example |
|----------|-------------|---------|
| `{{jwt.sub}}` | JWT subject claim | `user@example.com` |
| `{{jwt.iss}}` | JWT issuer claim | `auth.example.com` |
| `{{jwt.<claim>}}` | Any custom JWT claim | `{{jwt.org}}` → `Acme Inc` |
| `{{ip}}` | Client IP address | `192.168.1.100` |
| `{{header.X-Name}}` | Request header value | `{{header.X-User-Id}}` |
| `{{path}}` | Request path | `/preview/photo.jpg` |
| `{{bucket}}` | Bucket name | `previews` |
| `{{date}}` | Current date (YYYY-MM-DD) | `2025-12-25` |
| `{{datetime}}` | ISO 8601 datetime | `2025-12-25T14:30:00Z` |
| `{{timestamp}}` | Unix timestamp | `1735134600` |

### Example with Variables

```yaml
- type: text
  text: "Licensed to: {{jwt.sub}} - Downloaded {{datetime}}"
  font_size: 12
  color: "#333333"
  opacity: 0.6
  position: bottom-center
```

## Pattern Matching

Rules are matched against the request path using glob patterns:

| Pattern | Matches |
|---------|---------|
| `*.jpg` | All JPEG files |
| `*.png` | All PNG files |
| `/products/*` | All files under /products/ |
| `/preview/**/*.jpg` | JPEGs in any subdirectory |
| `*` | All files (default fallback) |

Rules are evaluated in order; first match wins.

### Example with Multiple Rules

```yaml
watermark:
  enabled: true
  rules:
    # High-res images get subtle watermark
    - pattern: "/hires/*.jpg"
      watermarks:
        - type: text
          text: "{{jwt.org}}"
          font_size: 14
          opacity: 0.2
          position: bottom-right

    # Thumbnails get no watermark
    - pattern: "/thumbs/*"
      watermarks: []  # Empty = no watermarks

    # Default: tiled watermark
    - pattern: "*"
      watermarks:
        - type: text
          text: "PREVIEW"
          font_size: 36
          opacity: 0.15
          position: tiled
          tile_spacing: 200
          rotation: -45
```

## Use Cases

### Copyright Protection

```yaml
- type: text
  text: "Copyright {{jwt.org}} {{date}}"
  position: bottom-right
  opacity: 0.4
```

### Preview Watermarks

```yaml
- type: text
  text: "PREVIEW - Not for distribution"
  position: tiled
  tile_spacing: 200
  rotation: -30
  opacity: 0.1
```

### User Tracking

```yaml
- type: text
  text: "Downloaded by {{jwt.sub}} on {{datetime}}"
  position: bottom-center
  font_size: 10
  opacity: 0.5
```

### Brand Overlay

```yaml
- type: image
  source: "https://cdn.example.com/logo.png"
  width: 80
  position: top-left
  opacity: 0.7
```

## Notes

- Watermarks are applied **after** image optimization (resize, effects) and **before** encoding
- Watermarks only apply when image processing is requested (e.g., `?w=800`)
- Original images without processing parameters are served without watermarks
- Image watermarks from S3 (`s3://`) require additional AWS SDK configuration
- Watermark images are cached in memory (LRU) for performance
