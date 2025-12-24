---
title: Image Optimization
layout: default
parent: Configuration
nav_order: 6
---

# Image Optimization Configuration

Configure on-the-fly image processing and optimization.
{: .fs-6 .fw-300 }

---

## Overview

Yatagarasu can resize, crop, format-convert, and optimize images on-the-fly. Processed variants are cached for subsequent requests.

```
Original Image (S3)
       |
       v
+------------------+
| Image Processor  |  Resize, crop, format conversion
+------------------+
       |
       v
+------------------+
| Variant Cache    |  Cache processed versions
+------------------+
       |
       v
    Response
```

---

## Enabling Image Optimization

```yaml
buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "my-images"
      region: "us-east-1"
    
    # Enable image optimization for this bucket
    image_optimization:
      enabled: true
      max_width: 4096
      max_height: 4096
      quality: 85
      allowed_formats:
        - jpeg
        - png
        - webp
        - avif
```

---

## Configuration Options

### Basic Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `enabled` | boolean | false | Enable image optimization |
| `max_width` | integer | 4096 | Maximum output width |
| `max_height` | integer | 4096 | Maximum output height |
| `quality` | integer | 85 | Default JPEG/WebP quality (1-100) |
| `auto_format` | boolean | true | Auto-select format based on Accept header |

### Format Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `allowed_formats` | array | [jpeg, png, webp, avif] | Allowed output formats |
| `default_format` | string | jpeg | Default when format not specified |
| `avif_speed` | integer | 6 | AVIF encoding speed (1=slow/best, 10=fast) |
| `webp_method` | integer | 4 | WebP compression method (0-6) |

### Security Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `require_signature` | boolean | false | Require HMAC signature |
| `signature_key` | string | - | HMAC secret key (env var recommended) |
| `max_file_size` | integer | 52428800 | Max input file size (50MB) |
| `max_pixels` | integer | 100000000 | Max input pixels (100MP) |

---

## Complete Example

```yaml
buckets:
  - name: "media"
    path_prefix: "/media"
    s3:
      bucket: "media-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    
    image_optimization:
      enabled: true
      
      # Size limits
      max_width: 4096
      max_height: 4096
      max_file_size: 52428800      # 50MB
      max_pixels: 100000000        # 100MP
      
      # Quality defaults
      quality: 85
      avif_speed: 6
      webp_method: 4
      
      # Format handling
      auto_format: true
      allowed_formats:
        - jpeg
        - png
        - webp
        - avif
      default_format: jpeg
      
      # Security
      require_signature: true
      signature_key: "${IMAGE_SIGNING_KEY}"
      
      # Auto-rotate based on EXIF
      auto_rotate: true
```

---

## URL Parameters

### Basic Transformations

```
/img/photo.jpg?w=800&h=600&q=80&fmt=webp
```

| Parameter | Description | Example |
|:----------|:------------|:--------|
| `w` | Width | `w=800` |
| `h` | Height | `h=600` |
| `q` | Quality (1-100) | `q=80` |
| `fmt` | Format (jpeg/png/webp/avif) | `fmt=webp` |
| `fit` | Fit mode (cover/contain/fill/inside/outside/pad) | `fit=cover` |
| `g` | Gravity (center/n/s/e/w/ne/nw/se/sw/smart) | `g=center` |
| `dpr` | Device pixel ratio | `dpr=2` |

### Rotation & Flip

| Parameter | Description | Example |
|:----------|:------------|:--------|
| `rotate` | Rotate (90/180/270) | `rotate=90` |
| `flip` | Flip (h/v/hv) | `flip=h` |
| `auto_rotate` | Auto-rotate from EXIF (default: true) | `auto_rotate=false` |

### Image Effects

| Parameter | Description | Range | Example |
|:----------|:------------|:------|:--------|
| `blur` | Gaussian blur sigma | 0-100 | `blur=5` |
| `sharpen` | Unsharp mask intensity | 0-10 | `sharpen=1.5` |
| `brightness` | Brightness adjustment | -100 to 100 | `brightness=20` |
| `contrast` | Contrast adjustment | -100 to 100 | `contrast=10` |
| `saturation` | Color saturation | -100 to 100 | `saturation=30` |
| `grayscale` | Convert to grayscale | true/1 | `grayscale=1` |

### Examples

```bash
# Resize and blur
curl "http://localhost:8080/img/photo.jpg?w=800&blur=3"

# Adjust brightness and contrast
curl "http://localhost:8080/img/photo.jpg?brightness=20&contrast=15"

# Grayscale with high contrast
curl "http://localhost:8080/img/photo.jpg?grayscale=1&contrast=30"

# Vintage look: desaturated with slight blur
curl "http://localhost:8080/img/photo.jpg?saturation=-40&blur=0.5"
```

---

## See Also

- [Image Optimization Tutorial](/yatagarasu/tutorials/image-optimization/)
- [Image Parameters Reference](/yatagarasu/reference/image-parameters/)
- [Cache Configuration](/yatagarasu/configuration/cache/)

