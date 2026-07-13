// ============================================================
// Static File Serving - خدمة الملفات الثابتة
// ============================================================
// Serves files from a local directory (e.g. `public/`). Supports:
// - automatic Content-Type detection by extension
// - ETag caching headers + If-None-Match → 304
// - Range requests (partial content, HTTP 206) for large files
// - directory index files (index.html)
// - path traversal prevention (canonicalization-based)
//
// يخدم الملفات من مجلد محلي مع دعم Range و caching.
// ============================================================

use std::path::{Path, PathBuf};

use crate::core::http::{Request, Response, StatusCode};

/// Configuration for static file serving.
#[derive(Debug, Clone)]
pub struct StaticFileConfig {
    /// Root directory to serve files from.
    pub root: PathBuf,
    /// Default file to look for when a directory is requested.
    pub index_file: String,
    /// Whether to generate ETag headers.
    pub generate_etag: bool,
    /// Whether to support Range requests.
    pub support_ranges: bool,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("public"),
            index_file: "index.html".to_string(),
            generate_etag: true,
            support_ranges: true,
        }
    }
}

/// A static file handler that serves files from a local directory.
pub struct StaticFileHandler {
    config: StaticFileConfig,
}

impl StaticFileHandler {
    pub fn new(config: StaticFileConfig) -> Self {
        Self { config }
    }

    /// Create a handler with the given root directory and default config.
    pub fn from_root(root: &str) -> Self {
        let mut config = StaticFileConfig::default();
        config.root = PathBuf::from(root);
        Self::new(config)
    }

    /// Handle a request. Returns `None` if no file matches the request path
    /// (so the caller can fall through to a 404 handler), or `Some(response)`
    /// with the file contents.
    pub fn handle(&self, request: &Request) -> Option<Response> {
        // Only GET and HEAD are valid for static files.
        if request.method != crate::core::http::Method::Get
            && request.method != crate::core::http::Method::Head
        {
            return None;
        }

        let file_path = self.resolve_path(&request.path)?;

        let content = std::fs::read(&file_path).ok()?;
        let metadata = std::fs::metadata(&file_path).ok()?;

        let content_type = guess_content_type(&file_path);
        let mut response = Response::ok()
            .header("content-type", &content_type)
            .body(content);

        response = response.header("content-length", &metadata.len().to_string());

        // ETag + conditional GET (If-None-Match → 304).
        if self.config.generate_etag {
            if let Ok(modified) = metadata.modified() {
                if let Ok(epoch) = modified.duration_since(std::time::UNIX_EPOCH) {
                    let etag = format!("\"{}-{}\"", metadata.len(), epoch.as_secs());
                    response = response.header("etag", &etag);

                    if let Some(inm) = request.header("if-none-match") {
                        if inm == etag {
                            let not_modified = Response::new(StatusCode::NOT_MODIFIED)
                                .header("etag", &etag);
                            return Some(not_modified);
                        }
                    }
                }
            }
        }

        // Range support (single byte range → 206 Partial Content).
        if self.config.support_ranges {
            if let Some(range_header) = request.header("range") {
                if let Some((start, end)) = parse_range(range_header, metadata.len()) {
                    let full = response.body.clone();
                    let slice_end = end.min(full.len() as u64) as usize;
                    let slice_start = start as usize;
                    if slice_start < slice_end {
                        let partial = full.slice(slice_start..slice_end);
                        let content_range = format!(
                            "bytes {}-{}/{}",
                            start,
                            slice_end - 1,
                            metadata.len()
                        );
                        let partial_resp = Response::new(StatusCode::PARTIAL_CONTENT)
                            .header("content-type", &content_type)
                            .header("content-length", &(slice_end - slice_start).to_string())
                            .header("content-range", &content_range)
                            .header("accept-ranges", "bytes")
                            .body(partial);
                        return Some(partial_resp);
                    }
                }
            }
            response = response.header("accept-ranges", "bytes");
        }

        // HEAD: strip the body, keep headers (so Content-Length still reflects size).
        if request.method == crate::core::http::Method::Head {
            response.body = bytes::Bytes::new();
        }

        Some(response)
    }

    /// Resolve a request path to a filesystem path, preventing directory
    /// traversal via canonicalization.
    fn resolve_path(&self, request_path: &str) -> Option<PathBuf> {
        let path = request_path.split('?').next().unwrap_or(request_path);
        let decoded = url_decode(path);
        let relative = decoded.trim_start_matches('/');
        let candidate = self.config.root.join(relative);

        let canonical_root = self.config.root.canonicalize().ok()?;
        let canonical_candidate = candidate.canonicalize().ok()?;

        if !canonical_candidate.starts_with(&canonical_root) {
            return None;
        }

        if canonical_candidate.is_dir() {
            let index = canonical_candidate.join(&self.config.index_file);
            if index.exists() {
                return Some(index);
            }
            return None;
        }

        if canonical_candidate.is_file() {
            Some(canonical_candidate)
        } else {
            None
        }
    }
}

/// Simple URL decoder for path components (handles %XX sequences and `+`).
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(a), Some(b)) = (h1, h2) {
                if let Ok(byte) = u8::from_str_radix(&format!("{}{}", a, b), 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push(c);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse a `Range: bytes=start-end` header. Returns (start, end_inclusive).
fn parse_range(header: &str, file_size: u64) -> Option<(u64, u64)> {
    let header = header.strip_prefix("bytes=")?;
    let parts: Vec<&str> = header.splitn(2, '-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start: u64 = parts[0].trim().parse().ok()?;
    let end = if parts[1].trim().is_empty() {
        file_size.saturating_sub(1)
    } else {
        parts[1].trim().parse().ok()?
    };
    if start > end || start >= file_size {
        return None;
    }
    Some((start, end))
}

/// Guess the MIME content type from a file extension.
fn guess_content_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        Some("md") => "text/markdown; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("gz") => "application/gzip",
        Some("tar") => "application/x-tar",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Extra status code used by static file serving (Range responses).
/// `NOT_MODIFIED` (304) is already defined on `StatusCode`.
impl StatusCode {
    /// 206 Partial Content (Range responses).
    pub const PARTIAL_CONTENT: Self = Self(206);
}

/// Convenience function returning a handler closure suitable for
/// `router.get("/static/*", noor::core::static_files::serve("public"))`.
pub fn serve(root: &str) -> impl Fn(Request) -> crate::NoorResult<Response> + Send + Sync + 'static {
    let handler = std::sync::Arc::new(StaticFileHandler::from_root(root));
    move |request: Request| {
        if let Some(response) = handler.handle(&request) {
            Ok(response)
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND)
                .html("<h1>404 - File Not Found</h1>"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::http::Method;
    use std::io::Write;

    fn setup_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::write(root.join("hello.txt"), b"Hello, World!").unwrap();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub").join("page.html"), b"<h1>Sub Page</h1>").unwrap();
        std::fs::write(root.join("index.html"), b"<h1>Index</h1>").unwrap();

        let mut css = std::fs::File::create(root.join("style.css")).unwrap();
        css.write_all(b"body { margin: 0; }").unwrap();
        drop(css);

        dir
    }

    #[test]
    fn test_serve_text_file() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Get, "/hello.txt".to_string());
        let resp = handler.handle(&req).unwrap();

        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.body, bytes::Bytes::from("Hello, World!"));
        assert!(resp
            .headers
            .iter()
            .any(|(k, v)| k == "content-type" && v.contains("text/plain")));
    }

    #[test]
    fn test_serve_html_file() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Get, "/sub/page.html".to_string());
        let resp = handler.handle(&req).unwrap();

        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.body, bytes::Bytes::from("<h1>Sub Page</h1>"));
    }

    #[test]
    fn test_serve_index_file() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Get, "/".to_string());
        let resp = handler.handle(&req).unwrap();

        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.body, bytes::Bytes::from("<h1>Index</h1>"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Get, "/../../../etc/passwd".to_string());
        let resp = handler.handle(&req);
        assert!(resp.is_none(), "path traversal should be blocked");
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Get, "/nope.txt".to_string());
        assert!(handler.handle(&req).is_none());
    }

    #[test]
    fn test_content_type_detection() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Get, "/style.css".to_string());
        let resp = handler.handle(&req).unwrap();
        assert!(resp
            .headers
            .iter()
            .any(|(k, v)| k == "content-type" && v.contains("text/css")));
    }

    #[test]
    fn test_head_request_strips_body() {
        let dir = setup_dir();
        let handler = StaticFileHandler::from_root(dir.path().to_str().unwrap());

        let req = Request::new(Method::Head, "/hello.txt".to_string());
        let resp = handler.handle(&req).unwrap();

        assert_eq!(resp.status, StatusCode::OK);
        assert!(resp.body.is_empty());
        assert!(resp.headers.iter().any(|(k, _)| k == "content-length"));
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("/hello%20world.txt"), "/hello world.txt");
        assert_eq!(url_decode("/path/file.txt"), "/path/file.txt");
    }
}
