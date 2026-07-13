// ============================================================
// Asset Pipeline - تجميع الأصول
// ============================================================
// CSS/JS bundling, minification, and cache-busting.
// Compiles SCSS/LESS, bundles JS modules, and generates
// cache-manifest with content hashing.
//
// تجميع وضغط CSS/JS مع تجزئة المحتوى.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Asset type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    Css,
    Js,
    Image,
    Font,
    Other,
}

impl AssetType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "css" => Self::Css,
            "js" | "mjs" => Self::Js,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => Self::Image,
            "woff" | "woff2" | "ttf" | "eot" => Self::Font,
            _ => Self::Other,
        }
    }
    
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Css => "text/css",
            Self::Js => "application/javascript",
            Self::Image => "image/*",
            Self::Font => "font/*",
            Self::Other => "application/octet-stream",
        }
    }
}

/// Asset manifest entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub source: String,
    pub output: String,
    pub asset_type: AssetType,
    pub hash: String,
    pub size: u64,
    pub dependencies: Vec<String>,
}

/// Asset pipeline configuration
#[derive(Debug, Clone)]
pub struct AssetConfig {
    pub source_dir: String,
    pub output_dir: String,
    pub public_url: String,
    pub minify: bool,
    pub source_maps: bool,
    pub cache_busting: bool,
    pub auto_version: bool,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            source_dir: "resources/assets".to_string(),
            output_dir: "public/assets".to_string(),
            public_url: "/assets".to_string(),
            minify: true,
            source_maps: true,
            cache_busting: true,
            auto_version: true,
        }
    }
}

/// Asset pipeline
pub struct AssetPipeline {
    config: AssetConfig,
    manifest: Arc<RwLock<HashMap<String, AssetEntry>>>,
}

impl AssetPipeline {
    pub fn new(config: AssetConfig) -> crate::NoorResult<Self> {
        std::fs::create_dir_all(&config.output_dir)?;
        
        Ok(Self {
            config,
            manifest: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Add a CSS asset
    pub fn css(&self, source: &str) -> crate::NoorResult<String> {
        self.process_asset(source, AssetType::Css)
    }
    
    /// Add a JS asset
    pub fn js(&self, source: &str) -> crate::NoorResult<String> {
        self.process_asset(source, AssetType::Js)
    }
    
    /// Process an asset
    fn process_asset(&self, source: &str, asset_type: AssetType) -> crate::NoorResult<String> {
        let source_path = PathBuf::from(&self.config.source_dir).join(source);
        
        if !source_path.exists() {
            return Err(crate::NoorError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Asset not found: {}", source_path.display()),
            )));
        }
        
        let content = std::fs::read(&source_path)?;
        let hash = crate::core::security::Encryption::sha256_hex(&content);
        let short_hash = &hash[..8];
        
        // Generate output filename with hash
        let filename = Path::new(source)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("asset");
        
        let extension = match asset_type {
            AssetType::Css => "css",
            AssetType::Js => "js",
            _ => Path::new(source)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("bin"),
        };
        
        let output_filename = if self.config.cache_busting {
            format!("{}.{}.{}", filename, short_hash, extension)
        } else {
            format!("{}.{}", filename, extension)
        };
        
        let output_path = PathBuf::from(&self.config.output_dir).join(&output_filename);
        let public_url = format!("{}/{}", self.config.public_url, output_filename);
        
        // Process content (minify if enabled)
        let processed_content = if self.config.minify {
            match asset_type {
                AssetType::Css => Self::minify_css(&content),
                AssetType::Js => Self::minify_js(&content),
                _ => content.clone(),
            }
        } else {
            content.clone()
        };
        
        // Write output
        std::fs::write(&output_path, &processed_content)?;
        
        // Add to manifest
        let entry = AssetEntry {
            source: source.to_string(),
            output: output_filename,
            asset_type,
            hash: hash.clone(),
            size: processed_content.len() as u64,
            dependencies: vec![],
        };
        
        self.manifest.write().insert(source.to_string(), entry);
        
        Ok(public_url)
    }
    
    /// Minify CSS (simplified)
    fn minify_css(content: &[u8]) -> Vec<u8> {
        let css = String::from_utf8_lossy(content);

        // `str::replace` treats its first argument as a *literal* substring,
        // so the previous `r"/*.*?*/"` matched nothing. Use a real regex to
        // strip CSS comments (including multi-line ones), then collapse
        // whitespace.
        let comment_re = regex::Regex::new(r"/\*[\s\S]*?\*/").expect("valid regex");
        let mut css = comment_re.replace_all(&css, "").to_string();

        // Collapse runs of whitespace into a single space.
        let ws_re = regex::Regex::new(r"\s+").expect("valid regex");
        css = ws_re.replace_all(&css, " ").to_string();

        css = css
            .replace(": ", ":")
            .replace(" {", "{")
            .replace("{ ", "{")
            .replace(";}", "}")
            .replace("; ", ";")
            .replace(", ", ",")
            .trim()
            .to_string();

        css.into_bytes()
    }

    /// Minify JavaScript (simplified)
    fn minify_js(content: &[u8]) -> Vec<u8> {
        let js = String::from_utf8_lossy(content);

        // Use real regexes instead of literal `str::replace` patterns.
        let block_re = regex::Regex::new(r"/\*[\s\S]*?\*/").expect("valid regex");
        let line_re = regex::Regex::new(r"//[^\n]*").expect("valid regex");
        let ws_re = regex::Regex::new(r"\s+").expect("valid regex");

        let js = block_re.replace_all(&js, "").to_string();
        let js = line_re.replace_all(&js, "").to_string();
        let js = ws_re.replace_all(&js, " ").trim().to_string();

        js.into_bytes()
    }
    
    /// Get the URL for an asset
    pub fn url(&self, source: &str) -> Option<String> {
        let manifest = self.manifest.read();
        
        manifest.get(source).map(|entry| {
            format!("{}/{}", self.config.public_url, entry.output)
        })
    }
    
    /// Get all assets of a specific type
    pub fn get_by_type(&self, asset_type: AssetType) -> Vec<AssetEntry> {
        self.manifest
            .read()
            .values()
            .filter(|e| e.asset_type == asset_type)
            .cloned()
            .collect()
    }
    
    /// Generate HTML tags for CSS assets
    pub fn css_tags(&self) -> String {
        self.get_by_type(AssetType::Css)
            .iter()
            .map(|entry| {
                format!(
                    r#"<link rel="stylesheet" href="{}/{}">"#,
                    self.config.public_url, entry.output
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Generate HTML tags for JS assets
    pub fn js_tags(&self) -> String {
        self.get_by_type(AssetType::Js)
            .iter()
            .map(|entry| {
                format!(
                    r#"<script src="{}/{}"></script>"#,
                    self.config.public_url, entry.output
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Get the manifest as JSON
    pub fn manifest_json(&self) -> serde_json::Value {
        let manifest = self.manifest.read();
        
        let map: HashMap<String, serde_json::Value> = manifest
            .iter()
            .map(|(k, v)| {
                (k.clone(), serde_json::json!({
                    "output": v.output,
                    "hash": v.hash,
                    "size": v.size,
                }))
            })
            .collect();
        
        serde_json::Value::Object(map.into_iter().map(|(k, v)| (k, v)).collect())
    }
    
    /// Clean up old assets not in the manifest
    pub fn cleanup(&self) -> crate::NoorResult<usize> {
        let manifest = self.manifest.read();
        let active_files: std::collections::HashSet<String> = manifest.values().map(|e| e.output.clone()).collect();
        
        let mut cleaned = 0;
        
        if let Ok(entries) = std::fs::read_dir(&self.config.output_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if !active_files.contains(filename) {
                        std::fs::remove_file(entry.path()).ok();
                        cleaned += 1;
                    }
                }
            }
        }
        
        Ok(cleaned)
    }
    
    /// Get the total size of all assets
    pub fn total_size(&self) -> u64 {
        self.manifest.read().values().map(|e| e.size).sum()
    }
    
    /// Get the number of assets
    pub fn count(&self) -> usize {
        self.manifest.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("css"), AssetType::Css);
        assert_eq!(AssetType::from_extension("js"), AssetType::Js);
        assert_eq!(AssetType::from_extension("png"), AssetType::Image);
        assert_eq!(AssetType::from_extension("woff2"), AssetType::Font);
    }
    
    #[test]
    fn test_css_minification() {
        let css = b"/* comment */\nbody {\n  margin: 0;\n  padding: 0;\n}";
        let minified = AssetPipeline::minify_css(css);
        
        let result = String::from_utf8_lossy(&minified);
        assert!(!result.contains("/* comment */"));
        assert!(!result.contains("\n  "));
    }
    
    #[test]
    fn test_asset_pipeline() {
        // Create temp directory
        let source_dir = "/tmp/noor_assets_test_source";
        let output_dir = "/tmp/noor_assets_test_output";
        
        std::fs::create_dir_all(source_dir).ok();
        std::fs::create_dir_all(output_dir).ok();
        
        // Create test CSS file
        std::fs::write(
            format!("{}/test.css", source_dir),
            "body { margin: 0; padding: 0; }"
        ).unwrap();
        
        let config = AssetConfig {
            source_dir: source_dir.to_string(),
            output_dir: output_dir.to_string(),
            minify: true,
            cache_busting: true,
            ..Default::default()
        };
        
        let pipeline = AssetPipeline::new(config).unwrap();
        
        let url = pipeline.css("test.css").unwrap();
        
        assert!(url.contains("/assets/test."));
        assert!(url.contains(".css"));
        
        assert_eq!(pipeline.count(), 1);
        
        // Clean up
        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(output_dir).ok();
    }
}
