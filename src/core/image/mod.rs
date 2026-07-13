// ============================================================
// Image Processing - معالجة الصور
// ============================================================
// Image manipulation: resize, crop, watermark, format conversion.
// Note: In production, use the `image` crate for actual processing.
//
// معالجة الصور: تغيير الحجم، القص، العلامة المائية.
// ============================================================

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Svg,
    Bmp,
}

impl ImageFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::WebP),
            "svg" => Some(Self::Svg),
            "bmp" => Some(Self::Bmp),
            _ => None,
        }
    }
    
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Svg => "svg",
            Self::Bmp => "bmp",
        }
    }
    
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Svg => "image/svg+xml",
            Self::Bmp => "image/bmp",
        }
    }
}

/// Image dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    
    pub fn aspect_ratio(&self) -> f64 {
        if self.height == 0 {
            0.0
        } else {
            self.width as f64 / self.height as f64
        }
    }
    
    /// Calculate new dimensions maintaining aspect ratio
    pub fn resize_to_fit(&self, max_width: u32, max_height: u32) -> Self {
        let ratio = self.aspect_ratio();
        let max_ratio = max_width as f64 / max_height as f64;
        
        if ratio > max_ratio {
            // Width is the limiting factor
            Self::new(max_width, (max_width as f64 / ratio) as u32)
        } else {
            // Height is the limiting factor
            Self::new((max_height as f64 * ratio) as u32, max_height)
        }
    }
    
    /// Calculate dimensions for a crop
    pub fn resize_to_fill(&self, target_width: u32, target_height: u32) -> Self {
        let ratio = self.aspect_ratio();
        let target_ratio = target_width as f64 / target_height as f64;
        
        if ratio > target_ratio {
            // Crop sides
            Self::new((target_height as f64 * ratio) as u32, target_height)
        } else {
            // Crop top/bottom
            Self::new(target_width, (target_width as f64 / ratio) as u32)
        }
    }
}

/// Image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub path: String,
    pub format: ImageFormat,
    pub dimensions: Dimensions,
    pub size_bytes: u64,
}

/// Resize options
#[derive(Debug, Clone)]
pub struct ResizeOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub maintain_aspect: bool,
    pub fill: bool,  // If true, crop to fill; if false, fit within
    pub quality: u8,  // 1-100 for JPEG/WebP
    pub format: Option<ImageFormat>,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            maintain_aspect: true,
            fill: false,
            quality: 85,
            format: None,
        }
    }
}

/// Watermark options
#[derive(Debug, Clone)]
pub struct WatermarkOptions {
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub position: WatermarkPosition,
    pub opacity: f32,  // 0.0 - 1.0
    pub padding: u32,
    pub font_size: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatermarkPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    Tile,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            text: None,
            image_path: None,
            position: WatermarkPosition::BottomRight,
            opacity: 0.5,
            padding: 10,
            font_size: Some(24),
        }
    }
}

/// Image processor
pub struct ImageProcessor {
    /// Output directory for processed images
    output_dir: PathBuf,
    /// Cache of processed images (path -> info)
    cache: Arc<RwLock<std::collections::HashMap<String, ImageInfo>>>,
}

impl ImageProcessor {
    pub fn new(output_dir: &str) -> crate::NoorResult<Self> {
        let path = PathBuf::from(output_dir);
        std::fs::create_dir_all(&path)?;
        
        Ok(Self {
            output_dir: path,
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }
    
    /// Get image information
    pub fn get_info(&self, path: &str) -> crate::NoorResult<ImageInfo> {
        let file_path = Path::new(path);
        
        if !file_path.exists() {
            return Err(crate::NoorError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Image file not found",
            )));
        }
        
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let format = ImageFormat::from_extension(extension)
            .ok_or_else(|| crate::NoorError::Validation(
                format!("Unsupported image format: {}", extension)
            ))?;
        
        let metadata = std::fs::metadata(file_path)?;
        
        // In a real implementation, we'd read actual dimensions
        // For now, we use placeholder dimensions
        let dimensions = Dimensions::new(1920, 1080);
        
        Ok(ImageInfo {
            path: path.to_string(),
            format,
            dimensions,
            size_bytes: metadata.len(),
        })
    }
    
    /// Resize an image
    pub fn resize(
        &self,
        input_path: &str,
        options: &ResizeOptions,
    ) -> crate::NoorResult<String> {
        let info = self.get_info(input_path)?;
        
        let target_dims = if let (Some(w), Some(h)) = (options.width, options.height) {
            if options.fill {
                info.dimensions.resize_to_fill(w, h)
            } else {
                info.dimensions.resize_to_fit(w, h)
            }
        } else if let Some(w) = options.width {
            let ratio = info.dimensions.aspect_ratio();
            Dimensions::new(w, (w as f64 / ratio) as u32)
        } else if let Some(h) = options.height {
            let ratio = info.dimensions.aspect_ratio();
            Dimensions::new((h as f64 * ratio) as u32, h)
        } else {
            info.dimensions
        };
        
        // Generate output path
        let output_filename = self.generate_output_filename(
            input_path,
            "resized",
            options.format.unwrap_or(info.format),
        );
        let output_path = self.output_dir.join(&output_filename);
        
        // In a real implementation, we'd use the `image` crate:
        // let img = image::open(input_path)?;
        // let resized = img.resize_exact(target_dims.width, target_dims.height, FilterType::Lanczos3);
        // resized.save(&output_path)?;
        
        // For now, copy the file as a placeholder
        std::fs::copy(input_path, &output_path)?;
        
        let result_path = output_path.to_string_lossy().to_string();
        
        // Cache the result
        self.cache.write().insert(result_path.clone(), ImageInfo {
            path: result_path.clone(),
            format: options.format.unwrap_or(info.format),
            dimensions: target_dims,
            size_bytes: std::fs::metadata(&output_path)?.len(),
        });
        
        Ok(result_path)
    }
    
    /// Create a thumbnail
    pub fn thumbnail(&self, input_path: &str, size: u32) -> crate::NoorResult<String> {
        self.resize(input_path, &ResizeOptions {
            width: Some(size),
            height: Some(size),
            maintain_aspect: true,
            fill: true,
            quality: 80,
            format: Some(ImageFormat::Jpeg),
        })
    }
    
    /// Add watermark to an image
    pub fn add_watermark(
        &self,
        input_path: &str,
        options: &WatermarkOptions,
    ) -> crate::NoorResult<String> {
        let info = self.get_info(input_path)?;
        
        let output_filename = self.generate_output_filename(
            input_path,
            "watermarked",
            info.format,
        );
        let output_path = self.output_dir.join(&output_filename);
        
        // In a real implementation:
        // 1. Open the image
        // 2. Create watermark (text or image)
        // 3. Composite watermark onto image at specified position
        // 4. Save the result
        
        std::fs::copy(input_path, &output_path)?;
        
        Ok(output_path.to_string_lossy().to_string())
    }
    
    /// Convert image format
    pub fn convert(
        &self,
        input_path: &str,
        target_format: ImageFormat,
        quality: u8,
    ) -> crate::NoorResult<String> {
        self.resize(input_path, &ResizeOptions {
            format: Some(target_format),
            quality,
            ..Default::default()
        })
    }
    
    /// Optimize image (reduce file size)
    pub fn optimize(&self, input_path: &str, quality: u8) -> crate::NoorResult<String> {
        let info = self.get_info(input_path)?;
        
        self.resize(input_path, &ResizeOptions {
            quality,
            format: Some(info.format),
            ..Default::default()
        })
    }
    
    /// Generate multiple sizes for responsive images
    pub fn generate_responsive_sizes(
        &self,
        input_path: &str,
        sizes: &[u32],
    ) -> crate::NoorResult<Vec<String>> {
        let mut results = Vec::new();
        
        for &size in sizes {
            let path = self.thumbnail(input_path, size)?;
            results.push(path);
        }
        
        Ok(results)
    }
    
    /// Generate a unique output filename
    fn generate_output_filename(
        &self,
        input_path: &str,
        suffix: &str,
        format: ImageFormat,
    ) -> String {
        let stem = Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random: u32 = rand::random();
        
        format!("{}_{}_{}_{}.{}", stem, suffix, timestamp, random, format.extension())
    }
    
    /// Clean up old processed images
    pub fn cleanup(&self, older_than_days: u32) -> crate::NoorResult<usize> {
        let mut cleaned = 0;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
        let cutoff_ts = cutoff.timestamp();
        
        if let Ok(entries) = std::fs::read_dir(&self.output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                            if (duration.as_secs() as i64) < cutoff_ts {
                                std::fs::remove_file(&path).ok();
                                cleaned += 1;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_image_format_from_extension() {
        assert_eq!(ImageFormat::from_extension("jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("PNG"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_extension("webp"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("xyz"), None);
    }
    
    #[test]
    fn test_dimensions_resize_to_fit() {
        let dims = Dimensions::new(1920, 1080);
        let resized = dims.resize_to_fit(800, 600);
        
        assert!(resized.width <= 800);
        assert!(resized.height <= 600);
    }
    
    #[test]
    fn test_dimensions_resize_to_fill() {
        let dims = Dimensions::new(1920, 1080);
        let resized = dims.resize_to_fill(800, 600);
        
        assert!(resized.width >= 800);
        assert!(resized.height >= 600);
    }
    
    #[test]
    fn test_aspect_ratio() {
        let dims = Dimensions::new(1920, 1080);
        let ratio = dims.aspect_ratio();
        
        assert!((ratio - 1.7778).abs() < 0.01);
    }
}
