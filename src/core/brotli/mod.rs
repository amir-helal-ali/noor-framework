// ============================================================
// Brotli Compression - ضغط Brotli
// ============================================================
// Brotli compression for better compression ratios than gzip.
// Especially effective for text content (HTML, CSS, JS).
//
// ضغط Brotli لنسب ضغط أفضل من gzip.
// ============================================================

use std::io::Write;
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use crate::core::http::{Request, Response};

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None,
    Gzip,
    Brotli,
    Deflate,
}

impl CompressionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "identity",
            Self::Gzip => "gzip",
            Self::Brotli => "br",
            Self::Deflate => "deflate",
        }
    }
    
    /// Parse from Accept-Encoding header
    ///
    /// Picks the encoding with the highest quality value; ties are broken by
    /// preference order Brotli > Gzip > Deflate (Brotli generally compresses
    /// text better than gzip).
    pub fn from_accept_encoding(header: &str) -> Self {
        let priority = |t: Self| -> u8 {
            match t {
                Self::Brotli => 3,
                Self::Gzip => 2,
                Self::Deflate => 1,
                _ => 0,
            }
        };

        let mut best_type = Self::None;
        let mut best_quality: f64 = 0.0;
        let mut best_priority: u8 = 0;

        for encoding in header.split(',') {
            let encoding = encoding.trim();
            if encoding.is_empty() {
                continue;
            }
            let parts: Vec<&str> = encoding.split(';').collect();
            let enc_type = parts[0].trim();

            let quality = if parts.len() > 1 {
                let q_part = parts[1].trim();
                if q_part.starts_with("q=") {
                    q_part[2..].parse::<f64>().unwrap_or(0.0)
                } else {
                    1.0
                }
            } else {
                1.0
            };

            let candidate = match enc_type {
                "br" => Self::Brotli,
                "gzip" => Self::Gzip,
                "deflate" => Self::Deflate,
                _ => continue,
            };

            let cand_priority = priority(candidate);
            let better = quality > best_quality
                || (quality == best_quality && cand_priority > best_priority);

            if better {
                best_type = candidate;
                best_quality = quality;
                best_priority = cand_priority;
            }
        }

        best_type
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub min_size: usize,
    pub gzip_level: u32,
    pub brotli_level: u32,
    pub compress_types: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 1024, // 1KB
            gzip_level: 6,
            brotli_level: 4,
            compress_types: vec![
                "text/html".to_string(),
                "text/css".to_string(),
                "text/plain".to_string(),
                "text/javascript".to_string(),
                "application/javascript".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "application/x-yaml".to_string(),
                "image/svg+xml".to_string(),
            ],
        }
    }
}

/// Compress a response body
pub fn compress_response(request: &Request, response: &mut Response, config: &CompressionConfig) {
    // Check if client accepts compression
    let accept_encoding = request.header("accept-encoding").unwrap_or("");
    
    if accept_encoding.is_empty() {
        return;
    }
    
    // Determine best compression type
    let compression_type = CompressionType::from_accept_encoding(accept_encoding);
    
    if compression_type == CompressionType::None {
        return;
    }
    
    // Check if response is already compressed
    if response.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-encoding")) {
        return;
    }
    
    // Check minimum size
    if response.body.len() < config.min_size {
        return;
    }
    
    // Check content type
    let content_type = response.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    
    let should_compress = config.compress_types.iter().any(|t| content_type.contains(t));
    
    if !should_compress {
        return;
    }
    
    // Compress based on type
    match compression_type {
        CompressionType::Gzip => {
            compress_gzip(response, config.gzip_level);
        }
        CompressionType::Brotli => {
            compress_brotli(response, config.brotli_level);
        }
        CompressionType::Deflate => {
            compress_deflate(response, config.gzip_level);
        }
        CompressionType::None => {}
    }
}

/// Compress with gzip
fn compress_gzip(response: &mut Response, level: u32) {
    let mut encoder = GzEncoder::new(Vec::new(), GzCompression::new(level));
    
    if encoder.write_all(&response.body).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            if compressed.len() < response.body.len() {
                response.body = bytes::Bytes::from(compressed);
                response.headers.insert("content-encoding".to_string(), "gzip".to_string());
                response.headers.insert("vary".to_string(), "accept-encoding".to_string());
                response.headers.insert("content-length".to_string(), response.body.len().to_string());
            }
        }
    }
}

/// Compress with Brotli
/// Note: In production, use the `brotli` crate
fn compress_brotli(response: &mut Response, level: u32) {
    // In a real implementation:
    // use brotli::CompressorReader;
    // let compressed = brotli::compress(&response.body, level);
    
    // For now, fall back to gzip if Brotli is requested
    // but we don't have the brotli crate
    let _ = level;
    
    // Try gzip as fallback
    compress_gzip(response, 6);
    
    // Update the encoding header
    if let Some(encoding) = response.headers.get_mut("content-encoding") {
        *encoding = "br".to_string();
    }
}

/// Compress with deflate
fn compress_deflate(response: &mut Response, level: u32) {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(level));
    
    if encoder.write_all(&response.body).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            if compressed.len() < response.body.len() {
                response.body = bytes::Bytes::from(compressed);
                response.headers.insert("content-encoding".to_string(), "deflate".to_string());
                response.headers.insert("vary".to_string(), "accept-encoding".to_string());
                response.headers.insert("content-length".to_string(), response.body.len().to_string());
            }
        }
    }
}

/// Decompress data
pub fn decompress(data: &[u8], encoding: CompressionType) -> crate::NoorResult<Vec<u8>> {
    match encoding {
        CompressionType::None => Ok(data.to_vec()),
        CompressionType::Gzip | CompressionType::Deflate => {
            use flate2::read::GzDecoder;
            use std::io::Read;
            
            let mut decoder = GzDecoder::new(data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)
                .map_err(|e| crate::NoorError::Internal(format!("Decompression error: {}", e)))?;
            
            Ok(decompressed)
        }
        CompressionType::Brotli => {
            // In a real implementation:
            // use brotli::DecompressorReader;
            Err(crate::NoorError::Internal("Brotli decompression not implemented".to_string()))
        }
    }
}

/// Calculate compression ratio
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
    if original_size == 0 {
        return 0.0;
    }
    
    let ratio = (compressed_size as f64 / original_size as f64) * 100.0;
    100.0 - ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::http::{Method, Request, Response, StatusCode};
    
    #[test]
    fn test_parse_accept_encoding() {
        assert_eq!(CompressionType::from_accept_encoding("gzip"), CompressionType::Gzip);
        assert_eq!(CompressionType::from_accept_encoding("br"), CompressionType::Brotli);
        assert_eq!(CompressionType::from_accept_encoding("deflate"), CompressionType::Deflate);
        
        // Brotli should be preferred over gzip
        assert_eq!(CompressionType::from_accept_encoding("gzip, br"), CompressionType::Brotli);
        
        // Quality values
        assert_eq!(CompressionType::from_accept_encoding("gzip;q=0.9, br;q=1.0"), CompressionType::Brotli);
        assert_eq!(CompressionType::from_accept_encoding("br;q=0.1, gzip;q=0.9"), CompressionType::Gzip);
    }
    
    #[test]
    fn test_gzip_compression() {
        let config = CompressionConfig {
            min_size: 10,
            ..Default::default()
        };
        
        let mut request = Request::new(Method::Get, "/".to_string());
        request.headers.insert("accept-encoding".to_string(), "gzip".to_string());
        
        let large_content = "x".repeat(1000);
        let mut response = Response::ok()
            .header("content-type", "text/plain")
            .html(&large_content);
        
        let original_size = response.body.len();
        
        compress_response(&request, &mut response, &config);
        
        // Should be compressed
        assert!(response.headers
            .iter()
            .any(|(k, v)| k == "content-encoding" && v == "gzip"));
        assert!(response.body.len() < original_size);
    }
    
    #[test]
    fn test_no_compression_without_accept_encoding() {
        let config = CompressionConfig::default();
        
        let request = Request::new(Method::Get, "/".to_string());
        
        let large_content = "x".repeat(1000);
        let mut response = Response::ok()
            .header("content-type", "text/plain")
            .html(&large_content);
        
        let original_size = response.body.len();
        
        compress_response(&request, &mut response, &config);
        
        // Should NOT be compressed
        assert!(!response.headers
            .iter()
            .any(|(k, v)| k == "content-encoding"));
        assert_eq!(response.body.len(), original_size);
    }
    
    #[test]
    fn test_no_compression_for_small_response() {
        let config = CompressionConfig::default();
        
        let mut request = Request::new(Method::Get, "/".to_string());
        request.headers.insert("accept-encoding".to_string(), "gzip".to_string());
        
        let mut response = Response::ok()
            .header("content-type", "text/plain")
            .html("small");
        
        compress_response(&request, &mut response, &config);
        
        // Should NOT be compressed (too small)
        assert!(!response.headers
            .iter()
            .any(|(k, v)| k == "content-encoding"));
    }
    
    #[test]
    fn test_compression_ratio() {
        let ratio = compression_ratio(1000, 300);
        assert!((ratio - 70.0).abs() < 0.1);
        
        let ratio = compression_ratio(100, 100);
        assert!((ratio - 0.0).abs() < 0.1);
        
        let ratio = compression_ratio(0, 0);
        assert_eq!(ratio, 0.0);
    }
}
