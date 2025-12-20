//! Request decompression middleware (Phase 40.3)
//!
//! This module will implement:
//! - Request body decompression (gzip, brotli, deflate)
//! - Content-Encoding header parsing
//! - Error handling for invalid compressed data
