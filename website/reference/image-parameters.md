---
title: Image Parameters
layout: default
parent: Reference
nav_order: 3
---

# Image Parameters Reference

Complete URL parameter reference for image optimization.
{: .fs-6 .fw-300 }

---

## Basic Parameters

### Dimensions

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `w` | integer | Width in pixels | `w=800` |
| `h` | integer | Height in pixels | `h=600` |
| `dpr` | float | Device pixel ratio (1.0-3.0) | `dpr=2` |

**Examples:**
```
# Fixed width, auto height (preserve aspect ratio)
/img/photo.jpg?w=800

# Fixed dimensions
/img/photo.jpg?w=800&h=600

# Retina display (2x)
/img/photo.jpg?w=400&dpr=2
# Results in 800px wide image
```

### Percentage Dimensions

```
# 50% of original width
/img/photo.jpg?w=50p

# 75% of original dimensions
/img/photo.jpg?w=75p&h=75p
```

---

## Quality & Format

| Parameter | Type | Description | Example |
|:----------|:-----|:------------|:--------|
| `q` | integer | Quality 1-100 | `q=80` |
| `fmt` | string | Output format | `fmt=webp` |

**Supported Formats:**
- `jpeg` / `jpg` - JPEG format
- `png` - PNG format
- `webp` - WebP format
- `avif` - AVIF format (best compression)
- `auto` - Auto-select based on Accept header

**Examples:**
```
# Convert to WebP at 80% quality
/img/photo.jpg?fmt=webp&q=80

# Auto-format (uses Accept header)
/img/photo.jpg?fmt=auto&w=800
```

---

## Fit Modes

| Parameter | Values | Description |
|:----------|:-------|:------------|
| `fit` | `contain` | Fit within bounds, preserve aspect ratio (default) |
| `fit` | `cover` | Fill bounds, crop excess |
| `fit` | `fill` | Stretch to exact dimensions |
| `fit` | `inside` | Same as contain |
| `fit` | `outside` | Scale to cover, no crop |
| `fit` | `pad` | Fit within bounds, add padding |

**Examples:**
```
# Contain (letterbox if needed)
/img/photo.jpg?w=800&h=600&fit=contain

# Cover (crop to fill)
/img/photo.jpg?w=800&h=600&fit=cover

# Pad with background color
/img/photo.jpg?w=800&h=600&fit=pad&bg=ffffff
```

---

## Gravity (Crop Position)

| Parameter | Values | Description |
|:----------|:-------|:------------|
| `g` | `center` | Center (default) |
| `g` | `north` | Top center |
| `g` | `south` | Bottom center |
| `g` | `east` | Right center |
| `g` | `west` | Left center |
| `g` | `northeast` | Top right |
| `g` | `northwest` | Top left |
| `g` | `southeast` | Bottom right |
| `g` | `southwest` | Bottom left |
| `g` | `smart` | Entropy-based smart crop |

**Examples:**
```
# Crop from top
/img/photo.jpg?w=800&h=600&fit=cover&g=north

# Smart crop (focus on interesting area)
/img/photo.jpg?w=800&h=600&fit=cover&g=smart
```

---

## Manual Crop

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `cx` | integer | Crop X offset |
| `cy` | integer | Crop Y offset |
| `cw` | integer | Crop width |
| `ch` | integer | Crop height |

**Example:**
```
# Crop 400x300 region starting at (100, 50)
/img/photo.jpg?cx=100&cy=50&cw=400&ch=300
```

---

## Transformations

### Rotation

| Parameter | Values | Description |
|:----------|:-------|:------------|
| `rot` | `90`, `180`, `270` | Rotate clockwise |
| `auto_rotate` | `0`, `1` | EXIF auto-rotate (default: 1) |

### Flip

| Parameter | Values | Description |
|:----------|:-------|:------------|
| `flip` | `h` | Flip horizontal |
| `flip` | `v` | Flip vertical |
| `flip` | `hv` | Flip both |

**Examples:**
```
# Rotate 90° clockwise
/img/photo.jpg?rot=90

# Flip horizontally
/img/photo.jpg?flip=h

# Disable EXIF auto-rotation
/img/photo.jpg?auto_rotate=0
```

---

## Background Color

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `bg` | hex | Background color for padding |

**Examples:**
```
# White background
/img/photo.png?w=800&h=600&fit=pad&bg=ffffff

# Transparent (PNG only)
/img/photo.png?w=800&h=600&fit=pad&bg=transparent
```

---

## URL Signing

When `require_signature` is enabled:

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `sig` | string | HMAC-SHA256 signature |
| `exp` | integer | Expiration timestamp (Unix) |

**Example:**
```
/img/photo.jpg?w=800&sig=abc123def456&exp=1735689600
```

---

## See Also

- [Image Optimization Configuration](/yatagarasu/configuration/image-optimization/)
- [Image Optimization Tutorial](/yatagarasu/tutorials/image-optimization/)

