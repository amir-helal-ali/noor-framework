// ============================================================
// Compression Middleware - middleware الضغط
// ============================================================

use std::io::Write;
use flate2::write::GzEncoder;
use flate2::Compression as FlateCompression;
use crate::core::http::{Request, Response};

/// Compression middleware
pub struct CompressionMiddleware {
    min_size: usize,
    level: u32,
}

impl CompressionMiddleware {
    pub fn new() -> Self {
        Self {
            min_size: 1024, // Only compress responses > 1KB
            level: 6,
        }
    }
    
    pub fn with_min_size(mut self, min_size: usize) -> Self {
        self.min_size = min_size;
        self
    }
    
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }
    
    /// Compress response body if client supports gzip
    pub fn compress(&self, request: &Request, response: &mut Response) {
        // Check if client accepts gzip
        let accept_encoding = request.header("accept-encoding").unwrap_or("");
        if !accept_encoding.contains("gzip") {
            return;
        }
        
        // Check if response is already compressed
        if response.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-encoding")) {
            return;
        }
        
        // Check minimum size
        if response.body.len() < self.min_size {
            return;
        }
        
        // Compress with gzip
        let mut encoder = GzEncoder::new(Vec::new(), FlateCompression::new(self.level));
        
        if encoder.write_all(&response.body).is_ok() {
            if let Ok(compressed) = encoder.finish() {
                // Only use compressed if it's actually smaller
                if compressed.len() < response.body.len() {
                    response.body = bytes::Bytes::from(compressed);
                    response.headers.insert(
                        "content-encoding".to_string(),
                        "gzip".to_string(),
                    );
                    response.headers.insert(
                        "vary".to_string(),
                        "accept-encoding".to_string(),
                    );
                    response.headers.insert(
                        "content-length".to_string(),
                        response.body.len().to_string(),
                    );
                }
            }
        }
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression() {
        let middleware = CompressionMiddleware::new().with_min_size(10);
        
        let mut request = Request::new(crate::core::http::Method::Get, "/".to_string());
        request.headers.insert("accept-encoding".to_string(), "gzip".to_string());
        
        let mut response = Response::ok().html(&"x".repeat(1000));
        let original_size = response.body.len();
        
        middleware.compress(&request, &mut response);
        
        // Should be compressed (gzip header set)
        assert!(response.headers
            .iter()
            .any(|(k, v)| k == "content-encoding" && v == "gzip"));
        assert!(response.body.len() < original_size);
    }
    
    #[test]
    fn test_no_compression_without_accept_encoding() {
        let middleware = CompressionMiddleware::new();
        
        let request = Request::new(crate::core::http::Method::Get, "/".to_string());
        let mut response = Response::ok().html(&"x".repeat(1000));
        
        middleware.compress(&request, &mut response);
        
        // Should NOT be compressed
        assert!(!response.headers
            .iter()
            .any(|(k, v)| k == "content-encoding" && v == "gzip"));
    }
}
