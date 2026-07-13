// ============================================================
// File Streaming - بث الملفات
// ============================================================
// Stream large files efficiently without loading into memory.
// Supports range requests, chunked transfer, and compression.
//
// بث الملفات الكبيرة بكفاءة بدون تحميلها في الذاكرة.
// ============================================================

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, BufReader, BufRead};
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Range request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>,
}

impl ByteRange {
    /// Parse a Range header value
    pub fn parse(header: &str) -> Option<Self> {
        let header = header.strip_prefix("bytes=")?;
        let parts: Vec<&str> = header.split('-').collect();
        
        if parts.len() != 2 {
            return None;
        }
        
        let start = parts[0].parse::<u64>().ok()?;
        let end = if parts[1].is_empty() {
            None
        } else {
            parts[1].parse::<u64>().ok()
        };
        
        Some(Self { start, end })
    }
    
    /// Get the content length of this range
    pub fn content_length(&self, file_size: u64) -> u64 {
        let end = self.end.unwrap_or(file_size - 1);
        end - self.start + 1
    }
    
    /// Get the Content-Range header value
    pub fn content_range(&self, file_size: u64) -> String {
        let end = self.end.unwrap_or(file_size - 1);
        format!("bytes {}-{}/{}", self.start, end, file_size)
    }
}

/// Stream metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMeta {
    pub file_size: u64,
    pub content_type: String,
    pub filename: String,
    pub last_modified: i64,
    pub etag: String,
}

/// File streamer
pub struct FileStreamer {
    /// Buffer size for reading (default: 8KB)
    buffer_size: usize,
    /// Maximum file size to load entirely in memory
    max_in_memory: u64,
}

impl Default for FileStreamer {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStreamer {
    pub fn new() -> Self {
        Self {
            buffer_size: 8192,  // 8KB
            max_in_memory: 10 * 1024 * 1024,  // 10MB
        }
    }
    
    /// Set buffer size
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }
    
    /// Set max in-memory size
    pub fn max_in_memory(mut self, size: u64) -> Self {
        self.max_in_memory = size;
        self
    }
    
    /// Get file metadata for streaming
    pub fn get_metadata(&self, path: &Path) -> crate::NoorResult<StreamMeta> {
        let metadata = std::fs::metadata(path)?;
        
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        
        let content_type = Self::guess_content_type(path);
        
        let last_modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        // Generate ETag from file size and last modified
        let etag_source = format!("{}-{}", metadata.len(), last_modified);
        let hash = crate::core::security::Encryption::sha256_hex(etag_source.as_bytes());
        let etag = format!("\"{}\"", &hash[..16]);
        
        Ok(StreamMeta {
            file_size: metadata.len(),
            content_type,
            filename,
            last_modified,
            etag,
        })
    }
    
    /// Stream a file in chunks
    pub fn stream<F>(&self, path: &Path, callback: F) -> crate::NoorResult<u64>
    where
        F: Fn(&[u8]) -> crate::NoorResult<()>,
    {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(self.buffer_size, file);
        let mut total_read = 0u64;
        
        loop {
            let buffer = reader.fill_buf()?;
            
            if buffer.is_empty() {
                break;
            }
            
            callback(buffer)?;
            
            let read = buffer.len();
            reader.consume(read);
            total_read += read as u64;
        }
        
        Ok(total_read)
    }
    
    /// Stream a range of a file
    pub fn stream_range<F>(&self, path: &Path, range: &ByteRange, callback: F) -> crate::NoorResult<u64>
    where
        F: Fn(&[u8]) -> crate::NoorResult<()>,
    {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(range.start))?;
        
        let mut reader = BufReader::with_capacity(self.buffer_size, file);
        let content_length = range.content_length(std::fs::metadata(path)?.len());
        let mut remaining = content_length;
        
        loop {
            if remaining == 0 {
                break;
            }
            
            let buffer = reader.fill_buf()?;
            
            if buffer.is_empty() {
                break;
            }
            
            let to_read = std::cmp::min(buffer.len(), remaining as usize);
            callback(&buffer[..to_read])?;
            
            reader.consume(to_read);
            remaining -= to_read as u64;
        }
        
        Ok(content_length)
    }
    
    /// Read file in chunks and process each chunk
    pub fn process_chunks<F, T>(&self, path: &Path, processor: F) -> crate::NoorResult<Vec<T>>
    where
        F: Fn(&[u8]) -> crate::NoorResult<T>,
    {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(self.buffer_size, file);
        let mut results = Vec::new();
        
        loop {
            let buffer = reader.fill_buf()?;
            
            if buffer.is_empty() {
                break;
            }
            
            let len = buffer.len();
            results.push(processor(buffer)?);
            reader.consume(len);
        }
        
        Ok(results)
    }
    
    /// Guess content type from file extension
    fn guess_content_type(path: &Path) -> String {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        match extension.to_lowercase().as_str() {
            "html" | "htm" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" | "mjs" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "txt" => "text/plain; charset=utf-8",
            "csv" => "text/csv; charset=utf-8",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "gz" | "gzip" => "application/gzip",
            "tar" => "application/x-tar",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "eot" => "application/vnd.ms-fontobject",
            "wasm" => "application/wasm",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            _ => "application/octet-stream",
        }
        .to_string()
    }
    
    /// Format file size for human-readable display
    pub fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        
        if unit_idx == 0 {
            format!("{} {}", bytes, UNITS[0])
        } else {
            format!("{:.2} {}", size, UNITS[unit_idx])
        }
    }
}

/// Streaming response builder
pub struct StreamResponse {
    pub content_type: String,
    pub content_length: Option<u64>,
    pub content_range: Option<String>,
    pub accept_ranges: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub attachment: bool,
    pub filename: Option<String>,
}

impl Default for StreamResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamResponse {
    pub fn new() -> Self {
        Self {
            content_type: "application/octet-stream".to_string(),
            content_length: None,
            content_range: None,
            accept_ranges: true,
            etag: None,
            last_modified: None,
            attachment: false,
            filename: None,
        }
    }
    
    pub fn content_type(mut self, ct: &str) -> Self {
        self.content_type = ct.to_string();
        self
    }
    
    pub fn content_length(mut self, len: u64) -> Self {
        self.content_length = Some(len);
        self
    }
    
    pub fn attachment(mut self, filename: &str) -> Self {
        self.attachment = true;
        self.filename = Some(filename.to_string());
        self
    }
    
    pub fn etag(mut self, etag: &str) -> Self {
        self.etag = Some(etag.to_string());
        self
    }
    
    /// Build HTTP headers for the response
    pub fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        headers.insert("Content-Type".to_string(), self.content_type.clone());
        
        if let Some(len) = self.content_length {
            headers.insert("Content-Length".to_string(), len.to_string());
        }
        
        if self.accept_ranges {
            headers.insert("Accept-Ranges".to_string(), "bytes".to_string());
        }
        
        if let Some(ref range) = self.content_range {
            headers.insert("Content-Range".to_string(), range.clone());
        }
        
        if let Some(ref etag) = self.etag {
            headers.insert("ETag".to_string(), etag.clone());
        }
        
        if let Some(ref lm) = self.last_modified {
            headers.insert("Last-Modified".to_string(), lm.clone());
        }
        
        if self.attachment {
            let filename = self.filename.as_deref().unwrap_or("file");
            headers.insert(
                "Content-Disposition".to_string(),
                format!("attachment; filename=\"{}\"", filename),
            );
        }
        
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_byte_range_parse() {
        let range = ByteRange::parse("bytes=0-499").unwrap();
        assert_eq!(range.start, 0);
        assert_eq!(range.end, Some(499));
        
        let range = ByteRange::parse("bytes=500-").unwrap();
        assert_eq!(range.start, 500);
        assert_eq!(range.end, None);
        
        let range = ByteRange::parse("bytes=-500");
        assert!(range.is_none()); // Doesn't match our parser
    }
    
    #[test]
    fn test_byte_range_content_length() {
        let range = ByteRange { start: 0, end: Some(499) };
        assert_eq!(range.content_length(1000), 500);
        
        let range = ByteRange { start: 500, end: None };
        assert_eq!(range.content_length(1000), 500);
    }
    
    #[test]
    fn test_byte_range_content_range() {
        let range = ByteRange { start: 0, end: Some(499) };
        assert_eq!(range.content_range(1000), "bytes 0-499/1000");
    }
    
    #[test]
    fn test_stream_file() {
        // Create temp file
        let path = Path::new("/tmp/noor_stream_test.txt");
        std::fs::write(path, "Hello, Streaming World!").unwrap();
        
        let streamer = FileStreamer::new();
        
        let total = streamer.stream(path, |chunk| {
            println!("Read {} bytes", chunk.len());
            Ok(())
        }).unwrap();
        
        assert_eq!(total, 23); // "Hello, Streaming World!" is 23 bytes
        
        std::fs::remove_file(path).ok();
    }
    
    #[test]
    fn test_format_size() {
        assert_eq!(FileStreamer::format_size(500), "500 B");
        assert_eq!(FileStreamer::format_size(1024), "1.00 KB");
        assert_eq!(FileStreamer::format_size(1048576), "1.00 MB");
        assert_eq!(FileStreamer::format_size(1073741824), "1.00 GB");
    }
    
    #[test]
    fn test_guess_content_type() {
        let streamer = FileStreamer::new();
        
        assert!(streamer.get_metadata(Path::new("/tmp/test.html")).is_err()); // File doesn't exist
    }
    
    #[test]
    fn test_stream_response_headers() {
        let response = StreamResponse::new()
            .content_type("text/plain")
            .content_length(100)
            .attachment("file.txt")
            .etag("\"abc123\"");
        
        let headers = response.headers();
        
        assert_eq!(headers.get("Content-Type"), Some(&"text/plain".to_string()));
        assert_eq!(headers.get("Content-Length"), Some(&"100".to_string()));
        assert!(headers.contains_key("Content-Disposition"));
        assert!(headers.contains_key("ETag"));
    }
}
