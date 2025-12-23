use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Whether image optimization is enabled (global switch)
    #[serde(default)]
    pub enabled: bool,

    /// Maximum allowed width for resized images (to prevent abuse)
    #[serde(default = "default_max_width")]
    pub max_width: u32,

    /// Maximum allowed height for resized images
    #[serde(default = "default_max_height")]
    pub max_height: u32,

    /// Default quality for lossy formats (JPEG, WebP, AVIF)
    #[serde(default = "default_quality")]
    pub default_quality: u8,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_width: 4096,
            max_height: 4096,
            default_quality: 80,
        }
    }
}

fn default_max_width() -> u32 {
    4096
}

fn default_max_height() -> u32 {
    4096
}

fn default_quality() -> u8 {
    80
}
