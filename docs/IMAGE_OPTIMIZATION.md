# Image Optimization

Yatagarasu provides on-the-fly image optimization with resize, crop, format conversion, quality adjustment, and image effects (blur, sharpen, brightness, contrast, saturation). All transformations are applied at request time via URL query parameters.

## Quick Start

```bash
# Resize to 400x300
curl "http://localhost:8080/images/photo.jpg?w=400&h=300"

# Convert to WebP with 80% quality
curl "http://localhost:8080/images/photo.jpg?fmt=webp&q=80"

# Thumbnail with smart crop
curl "http://localhost:8080/images/photo.jpg?w=150&h=150&fit=cover&g=smart"
```

## Configuration

Enable image optimization per bucket in your config:

```yaml
buckets:
  - name: "images"
    path_prefix: "/images"
    s3:
      bucket: "my-images"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    image_optimization:
      enabled: true
      max_width: 4096 # Maximum output width
      max_height: 4096 # Maximum output height
      default_quality: 85 # Default quality (1-100)
      allowed_formats: # Allowed output formats
        - jpeg
        - webp
        - png
        - avif
```

## URL Parameters

### Resize

| Parameter | Description              | Example                |
| --------- | ------------------------ | ---------------------- |
| `w`       | Width in pixels          | `?w=400`               |
| `h`       | Height in pixels         | `?h=300`               |
| `dpr`     | Device pixel ratio (1-4) | `?w=200&dpr=2` → 400px |

### Fit Mode

| Value     | Description                     |
| --------- | ------------------------------- |
| `contain` | Fit inside dimensions (default) |
| `cover`   | Fill dimensions, crop excess    |
| `fill`    | Stretch to exact dimensions     |
| `inside`  | Same as contain                 |
| `outside` | Minimum dimensions              |
| `pad`     | Pad to exact dimensions         |

```bash
# Contain (fit inside, preserve aspect ratio)
curl "http://localhost:8080/images/photo.jpg?w=400&h=300&fit=contain"

# Cover (fill and crop)
curl "http://localhost:8080/images/photo.jpg?w=400&h=300&fit=cover"
```

### Format Conversion

| Parameter | Values                        | Description    |
| --------- | ----------------------------- | -------------- |
| `fmt`     | `jpeg`, `webp`, `png`, `avif` | Output format  |
| `q`       | 1-100                         | Output quality |

```bash
# Convert to WebP at 80% quality
curl "http://localhost:8080/images/photo.jpg?fmt=webp&q=80"

# Convert to AVIF (best compression)
curl "http://localhost:8080/images/photo.jpg?fmt=avif&q=75"
```

### Gravity (Crop Anchor)

| Value                                              | Description                         |
| -------------------------------------------------- | ----------------------------------- |
| `center`                                           | Center (default)                    |
| `north`, `south`, `east`, `west`                   | Edge positions                      |
| `northeast`, `northwest`, `southeast`, `southwest` | Corner positions                    |
| `smart`                                            | Smart crop (face/content detection) |

```bash
# Crop from top-left
curl "http://localhost:8080/images/photo.jpg?w=200&h=200&fit=cover&g=northwest"

# Smart crop (focus on faces/content)
curl "http://localhost:8080/images/photo.jpg?w=200&h=200&fit=cover&g=smart"
```

### Transformations

| Parameter     | Values                  | Description              |
| ------------- | ----------------------- | ------------------------ |
| `rot`         | `0`, `90`, `180`, `270` | Rotation in degrees      |
| `flip`        | `h`, `v`, `hv`          | Flip horizontal/vertical |
| `auto_rotate` | `0`, `1`                | EXIF-based auto-rotation |

```bash
# Rotate 90 degrees
curl "http://localhost:8080/images/photo.jpg?rot=90"

# Flip horizontal
curl "http://localhost:8080/images/photo.jpg?flip=h"
```

### Image Effects

| Parameter    | Range       | Description                |
| ------------ | ----------- | -------------------------- |
| `blur`       | 0-100       | Gaussian blur sigma        |
| `sharpen`    | 0-10        | Unsharp mask intensity     |
| `brightness` | -100 to 100 | Brightness adjustment      |
| `contrast`   | -100 to 100 | Contrast adjustment        |
| `saturation` | -100 to 100 | Color saturation           |
| `grayscale`  | 0, 1        | Convert to black and white |

```bash
# Apply gaussian blur
curl "http://localhost:8080/images/photo.jpg?blur=5"

# Sharpen image
curl "http://localhost:8080/images/photo.jpg?sharpen=1.5"

# Adjust brightness and contrast
curl "http://localhost:8080/images/photo.jpg?brightness=20&contrast=10"

# Desaturate for vintage look
curl "http://localhost:8080/images/photo.jpg?saturation=-50"

# Convert to grayscale
curl "http://localhost:8080/images/photo.jpg?grayscale=1"

# Combine multiple effects
curl "http://localhost:8080/images/photo.jpg?w=800&brightness=10&contrast=15&saturation=20"
```

## Caching

Optimized images are cached based on the full URL including query parameters. Each unique combination of parameters creates a separate cache entry.

```yaml
cache:
  memory:
    max_capacity: 1073741824 # 1GB
    ttl_seconds: 3600
  disk:
    path: "/var/cache/yatagarasu"
    max_size: 10737418240 # 10GB
```

## URL Signing

For secure access, enable URL signing to prevent parameter tampering:

```yaml
image_optimization:
  enabled: true
  require_signature: true
  signature_key: "${IMAGE_SIGNATURE_KEY}"
```

Signed URL format:

```
/images/photo.jpg?w=400&h=300&sig=<hmac-sha256-signature>
```

Generate signatures using HMAC-SHA256 of the path and sorted query parameters.

## Performance

| Operation              | Typical Time |
| ---------------------- | ------------ |
| Decode JPEG (1MP)      | ~10ms        |
| Resize 1080p → 400x300 | ~15ms        |
| Encode WebP (q=80)     | ~20ms        |
| Encode AVIF (q=75)     | ~100ms       |

Use lower quality for thumbnails and WebP/AVIF for better compression.

## Migration Guide

Image optimization (including effects) is a new feature in v1.5.0. There are no breaking changes to existing functionality.

### Enabling Image Optimization

Add `image_optimization` to any existing bucket configuration:

```yaml
buckets:
  - name: "existing-bucket"
    path_prefix: "/existing"
    s3:
      # ... existing S3 config ...
    image_optimization: # Add this section
      enabled: true
      max_width: 4096
      max_height: 4096
      default_quality: 85
```

### Notes

- Existing requests without image parameters continue to work unchanged
- Non-image files pass through without modification
- Image parameters only apply to requests with `?w=`, `?h=`, `?fmt=`, etc.
- No additional dependencies required (all encoders bundled)

## See Also

- [Configuration Reference](../website/configuration/image-optimization.md)
- [URL Parameters Reference](../website/reference/image-parameters.md)
- [Image Optimization Tutorial](../website/tutorials/image-optimization.md)
