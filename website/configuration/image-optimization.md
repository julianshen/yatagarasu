---
title: Image Optimization
layout: default
parent: Configuration
nav_order: 6
---

# Image Optimization

Configure on-the-fly image processing, resizing, and effects.
{: .fs-6 .fw-300 }

---

## Overview

Yatagarasu can resize, crop, format-convert, apply effects, and optimize images on-the-fly. Processed variants are cached for subsequent requests.

```
Original Image (S3)
       |
       v
+------------------+
| Image Processor  |  Decode -> Transform -> Effects -> Encode
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

## Configuration

Image optimization is configured at the **top level** of the configuration file (not per-bucket):

```yaml
# Top-level configuration
image_optimization:
  enabled: true
  max_width: 4096
  max_height: 4096

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "my-images"
      region: "us-east-1"
```

### Configuration Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `enabled` | boolean | `false` | Enable image optimization |
| `max_width` | integer | `4096` | Maximum output width in pixels |
| `max_height` | integer | `4096` | Maximum output height in pixels |

---

## URL Parameters

Transform images by adding query parameters to the URL.

### Resize Parameters

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `w` | integer or percentage | Target width | `w=800` or `w=50p` |
| `h` | integer or percentage | Target height | `h=600` or `h=50%` |
| `dpr` | float (1-4) | Device pixel ratio multiplier | `dpr=2` |
| `fit` | string | Resize fit mode | `fit=cover` |
| `g` | string | Gravity/anchor for cropping | `g=center` |
| `enlarge` | boolean | Allow upscaling beyond original | `enlarge=1` |

### Fit Modes

| Value | Description |
|:------|:------------|
| `cover` | Crop to fill target dimensions (default) |
| `contain` | Scale to fit within dimensions, preserving aspect ratio |
| `fill` | Stretch to fill exactly (may distort) |
| `inside` | Scale down only, never up |
| `outside` | Scale to cover, may exceed target |
| `pad` | Add padding to fill dimensions |

### Gravity Options

| Value | Description |
|:------|:------------|
| `center`, `c` | Center (default) |
| `north`, `n` | Top center |
| `south`, `s` | Bottom center |
| `east`, `e` | Right center |
| `west`, `w` | Left center |
| `northeast`, `ne` | Top-right |
| `northwest`, `nw` | Top-left |
| `southeast`, `se` | Bottom-right |
| `southwest`, `sw` | Bottom-left |
| `smart`, `sm` | Content-aware smart crop |

---

### Format & Quality

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `fmt` or `f` | string | Output format | `fmt=webp` |
| `q` | integer (1-100) | Output quality | `q=80` |
| `strip` | boolean | Strip metadata (default: true) | `strip=0` |
| `progressive` | boolean | Progressive encoding (default: true) | `progressive=0` |

**Supported Formats:** `jpeg`, `jpg`, `png`, `webp`, `avif`, `auto`

---

### Rotation & Flip

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `r` | integer | Rotate degrees (0, 90, 180, 270) | `r=90` |
| `flip` | string | Flip direction (h, v, hv) | `flip=h` |
| `auto_rotate` | boolean | Auto-rotate from EXIF (default: true) | `auto_rotate=0` |

---

### Manual Crop

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `cx` | integer | Crop X offset | `cx=100` |
| `cy` | integer | Crop Y offset | `cy=50` |
| `cw` | integer | Crop width | `cw=500` |
| `ch` | integer | Crop height | `ch=400` |

---

### Image Effects

| Parameter | Type | Range | Description | Example |
|:----------|:-----|:------|:------------|:--------|
| `blur` | float | 0-100 | Gaussian blur sigma | `blur=5` |
| `sharpen` | float | 0-10 | Unsharp mask intensity | `sharpen=1.5` |
| `brightness` | integer | -100 to 100 | Brightness adjustment | `brightness=20` |
| `contrast` | integer | -100 to 100 | Contrast adjustment | `contrast=10` |
| `saturation` | integer | -100 to 100 | Color saturation | `saturation=30` |
| `grayscale` | boolean | true/1 | Convert to grayscale | `grayscale=1` |

---

### Background

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `bg` | string | Background color (hex RGB) for padding | `bg=ffffff` |

---

## Examples

### Basic Resize

```bash
# Resize to 800px width (height auto-calculated)
curl "http://localhost:8080/img/photo.jpg?w=800"

# Resize to exact dimensions with cover fit
curl "http://localhost:8080/img/photo.jpg?w=800&h=600&fit=cover"

# Resize to 50% of original
curl "http://localhost:8080/img/photo.jpg?w=50p"
```

### Format Conversion

```bash
# Convert to WebP with quality 85
curl "http://localhost:8080/img/photo.jpg?fmt=webp&q=85"

# Convert to AVIF (modern browsers)
curl "http://localhost:8080/img/photo.jpg?fmt=avif&q=80"
```

### Retina/HiDPI Images

```bash
# 2x resolution for retina displays
curl "http://localhost:8080/img/photo.jpg?w=400&dpr=2"
# Results in 800px wide image
```

### Effects

```bash
# Apply blur effect
curl "http://localhost:8080/img/photo.jpg?w=800&blur=3"

# Sharpen the image
curl "http://localhost:8080/img/photo.jpg?w=800&sharpen=1.5"

# Adjust brightness and contrast
curl "http://localhost:8080/img/photo.jpg?brightness=20&contrast=15"

# Grayscale with high contrast
curl "http://localhost:8080/img/photo.jpg?grayscale=1&contrast=30"

# Vintage look: desaturated with slight blur
curl "http://localhost:8080/img/photo.jpg?saturation=-40&blur=0.5"
```

### Rotation and Flip

```bash
# Rotate 90 degrees clockwise
curl "http://localhost:8080/img/photo.jpg?r=90"

# Flip horizontally (mirror)
curl "http://localhost:8080/img/photo.jpg?flip=h"

# Disable auto-rotation from EXIF
curl "http://localhost:8080/img/photo.jpg?auto_rotate=0"
```

### Manual Crop

```bash
# Crop a 500x400 region starting at (100, 50)
curl "http://localhost:8080/img/photo.jpg?cx=100&cy=50&cw=500&ch=400"
```

### Combined Transformations

```bash
# Resize, convert to WebP, and apply effects
curl "http://localhost:8080/img/photo.jpg?w=800&fmt=webp&q=85&sharpen=0.5&contrast=10"
```

---

## Processing Pipeline

Images are processed in this order:

```
1. Decode image
2. EXIF auto-rotation (if enabled)
3. Manual crop (cx, cy, cw, ch)
4. Resize with fit mode
5. Rotate/Flip
6. Effects (blur, sharpen, brightness, contrast, saturation, grayscale)
7. Apply watermarks (if configured)
8. Encode to output format
```

---

## Caching

Processed image variants are cached based on the transformation parameters:

- Each unique combination of parameters creates a separate cache entry
- Cache key includes: width, height, quality, format, fit, effects, etc.
- Original images and processed variants are cached separately

---

## See Also

- [Watermarks Configuration](/yatagarasu/configuration/watermarks/)
- [Cache Configuration](/yatagarasu/configuration/cache/)
