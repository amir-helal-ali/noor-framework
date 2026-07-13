// ============================================================
// Server - HTTP Server
// خادم HTTP
// ============================================================
// A real async HTTP server built on tokio + hyper 1.x.
// Converts incoming hyper requests into the framework's `Request` type,
// dispatches them through the `Router`, and converts the framework's
// `Response` back to a hyper response.
//
// خادم HTTP غير متزامن مبني على tokio و hyper.
// ============================================================

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::core::config::Config;
use crate::core::http::{Method, Request, Response, StatusCode};
use crate::core::router::Router;

/// The HTTP server
/// خادم HTTP
pub struct Server {
    config: Arc<Config>,
    router: Arc<Router>,
}

impl Server {
    pub fn new(config: Arc<Config>, router: Arc<Router>) -> Self {
        Self { config, router }
    }

    /// Run the server: bind, accept connections, and serve requests.
    ///
    /// This blocks the calling thread until the server is shut down
    /// (Ctrl+C / SIGTERM). Each connection is served on its own tokio task.
    pub fn run(&self) -> crate::NoorResult<()> {
        // Initialize tracing subscriber (idempotent — safe to call multiple
        // times). This enables the `tracing::info!` request logs emitted by
        // `handle_request`.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_target(false)
            .try_init();

        let host = &self.config.server.host;
        let port = self.config.server.port;
        let addr: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| crate::NoorError::Internal(format!("Invalid address: {}", e)))?;

        println!("\n{}", crate::banner());
        println!("  Server running at http://{}", addr);
        println!("  Environment: {:?}", self.config.app.env);
        println!("  Workers: {}", self.config.server.workers);
        println!("  Press Ctrl+C to stop\n");

        // Build a multi-threaded tokio runtime sized to the configured worker
        // count, defaulting to the number of logical CPUs.
        let worker_threads = if self.config.server.workers > 0 {
            self.config.server.workers
        } else {
            num_cpus_fallback()
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .enable_all()
            .build()
            .map_err(|e| crate::NoorError::Internal(format!("Tokio runtime error: {}", e)))?;

        runtime.block_on(async move {
            let listener = match TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to bind to {}: {}", addr, e);
                    return;
                }
            };

            // Spawn a task per accepted connection, until Ctrl+C / SIGTERM.
            // `tokio::select!` races the accept loop against a shutdown signal
            // so the server stops cleanly without dropping in-flight requests.
            let shutdown = tokio::signal::ctrl_c();

            tokio::pin!(shutdown);

            loop {
                tokio::select! {
                    _ = &mut shutdown => {
                        eprintln!("\n  Shutting down gracefully...");
                        break;
                    }
                    accept_result = listener.accept() => {
                        let (stream, peer) = match accept_result {
                            Ok(conn) => conn,
                            Err(e) => {
                                eprintln!("Accept error: {}", e);
                                continue;
                            }
                        };

                        let io = TokioIo::new(stream);
                        let router = self.router.clone();
                        let config = self.config.clone();
                        let peer_ip = peer.ip().to_string();

                        tokio::task::spawn(async move {
                            if let Err(e) = http1::Builder::new()
                                .keep_alive(true)
                                .serve_connection(
                                    io,
                                    service_fn(move |req| {
                                        let router = router.clone();
                                        let config = config.clone();
                                        let peer_ip = peer_ip.clone();
                                        async move {
                                            handle_request(req, router, config, Some(peer_ip)).await
                                        }
                                    }),
                                )
                                .await
                            {
                                eprintln!("Connection error: {}", e);
                            }
                        });
                    }
                }
            }

            eprintln!("  Server stopped.");
        });

        Ok(())
    }
}

/// Convert a hyper request into a Noor `Request`, dispatch it through the
/// router, and convert the Noor `Response` into a hyper response.
///
/// After the handler runs, applies post-processing middleware based on the
/// application config:
/// - CORS headers (if `security.cors_origins` is non-empty)
/// - Security headers (X-Content-Type-Options, X-Frame-Options, etc.)
/// - Request logging (via `tracing`)
async fn handle_request(
    req: HyperRequest<Incoming>,
    router: Arc<Router>,
    config: Arc<Config>,
    peer_ip: Option<String>,
) -> Result<HyperResponse<Full<Bytes>>, Infallible> {
    let start = std::time::Instant::now();
    let method_str = req.method().as_str().to_string();
    let uri_str = req.uri().to_string();

    // --- Map hyper request → Noor request ---
    let method = Method::from_str(req.method().as_str()).unwrap_or(Method::Get);

    // Collect headers (case-insensitive lookups later via `header()`).
    let mut headers = std::collections::HashMap::new();
    for (name, value) in req.headers().iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_string(), v.to_string());
        }
    }

    let user_agent = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("user-agent"))
        .map(|(_, v)| v.clone());
    // Prefer X-Forwarded-For (set by reverse proxies); fall back to the
    // TCP peer address.
    let client_ip = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("x-forwarded-for"))
        .map(|(_, v)| v.split(',').next().unwrap_or("").trim().to_string())
        .or(peer_ip);

    // Collect the full body, enforcing the configured body size limit.
    let body_limit = config.server.body_limit;
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => {
            let bytes = collected.to_bytes();
            if bytes.len() > body_limit {
                // Body too large — return 413 immediately.
                let resp = Response::new(StatusCode::PAYLOAD_TOO_LARGE)
                    .text(format!(
                        "Body size {} exceeds limit of {} bytes",
                        bytes.len(),
                        body_limit
                    ));
                return Ok(to_hyper_response(resp));
            }
            bytes
        }
        Err(_) => Bytes::new(),
    };

    let mut request = Request::new(method, uri_str.clone());
    request.headers = headers;
    request.body = body_bytes;
    request.user_agent = user_agent;
    request.client_ip = client_ip;
    request.is_secure = false; // would need TLS info to set this
    request.parse_cookies(); // populate request.cookies from Cookie header

    // --- Dispatch ---
    let mut response: Response = match router.dispatch(request) {
        Ok(resp) => resp,
        Err(e) => {
            let code = e.status_code();
            let status_code = match code {
                400 => StatusCode::BAD_REQUEST,
                401 => StatusCode::UNAUTHORIZED,
                403 => StatusCode::FORBIDDEN,
                404 => StatusCode::NOT_FOUND,
                405 => StatusCode::METHOD_NOT_ALLOWED,
                429 => StatusCode::TOO_MANY_REQUESTS,
                500 => StatusCode::INTERNAL_SERVER_ERROR,
                502 => StatusCode::BAD_GATEWAY,
                503 => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let body = format!("Internal Server Error: {}", e);
            Response::new(status_code).text(body)
        }
    };

    // --- Post-processing: CORS headers ---
    if !config.security.cors_origins.is_empty() {
        let cors = crate::core::middleware::cors::CorsMiddleware::new(
            crate::core::middleware::cors::CorsConfig {
                allowed_origins: config.security.cors_origins.clone(),
                ..Default::default()
            },
        );
        // We need a lightweight Request for `apply_headers`; reconstruct
        // just the method (the only field CORS uses).
        let dummy_req = Request::new(method, uri_str.clone());
        cors.apply_headers(&dummy_req, &mut response);
    }

    // --- Post-processing: security headers ---
    if config.security.secure_headers {
        apply_security_headers(&mut response);
    }

    // --- Post-processing: response compression ---
    // Compress the response body if:
    //   - compression is enabled in config
    //   - the body is large enough (>= min_size)
    //   - the content type is compressible
    //   - the client sent Accept-Encoding
    if config.server.compression {
        let comp_config = crate::core::brotli::CompressionConfig::default();
        // Reconstruct a lightweight Request for compress_response (it only
        // reads the Accept-Encoding header).
        let dummy_req = Request::new(method, uri_str.clone());
        crate::core::brotli::compress_response(&dummy_req, &mut response, &comp_config);
    }

    // --- Request logging ---
    let elapsed = start.elapsed();
    tracing::info!(
        method = %method_str,
        path = %uri_str,
        status = response.status.0,
        elapsed_ms = elapsed.as_millis() as u64,
        "request"
    );

    // --- Map Noor response → hyper response ---
    Ok(to_hyper_response(response))
}

/// Apply standard security hardening headers to a response.
///
/// These protect against common attacks:
/// - `X-Content-Type-Options: nosniff` — prevents MIME-type sniffing
/// - `X-Frame-Options: DENY` — prevents clickjacking
/// - `X-XSS-Protection: 1; mode=block` — legacy XSS filter (older browsers)
/// - `Strict-Transport-Security` — forces HTTPS (respected on next visit)
/// - `Referrer-Policy` — controls how much referrer info is leaked
fn apply_security_headers(response: &mut Response) {
    response
        .headers
        .entry("x-content-type-options".to_string())
        .or_insert_with(|| "nosniff".to_string());
    response
        .headers
        .entry("x-frame-options".to_string())
        .or_insert_with(|| "DENY".to_string());
    response
        .headers
        .entry("x-xss-protection".to_string())
        .or_insert_with(|| "1; mode=block".to_string());
    response
        .headers
        .entry("strict-transport-security".to_string())
        .or_insert_with(|| "max-age=31536000; includeSubDomains".to_string());
    response
        .headers
        .entry("referrer-policy".to_string())
        .or_insert_with(|| "strict-origin-when-cross-origin".to_string());
}

/// Convert a Noor `Response` into a hyper `Response<Full<Bytes>>`.
fn to_hyper_response(resp: Response) -> HyperResponse<Full<Bytes>> {
    let status = hyper::StatusCode::from_u16(resp.status.0).unwrap_or(hyper::StatusCode::OK);

    // Capture the body length before moving the bytes into `Full`.
    let body_len = resp.body.len();
    let mut builder = HyperResponse::builder().status(status);

    for (name, value) in &resp.headers {
        if let Ok(header_name) = hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            if let Ok(header_value) = hyper::header::HeaderValue::from_str(value) {
                builder = builder.header(header_name, header_value);
            }
        }
    }

    // Always set Content-Length to the actual body length (overriding any
    // stale value the handler may have set).
    builder = builder.header(hyper::header::CONTENT_LENGTH, body_len.to_string());

    let body = Full::new(resp.body);
    builder.body(body).unwrap_or_else(|_| {
        HyperResponse::builder()
            .status(500)
            .body(Full::new(Bytes::from_static(b"Internal Server Error")))
            .expect("static fallback response")
    })
}

/// Fallback for the number of CPUs when the configured worker count is 0.
fn num_cpus_fallback() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
