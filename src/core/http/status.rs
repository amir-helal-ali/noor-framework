// ============================================================
// HTTP Status Codes - رموز حالة HTTP
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(pub u16);

impl StatusCode {
    // 2xx Success
    pub const OK: Self = Self(200);
    pub const CREATED: Self = Self(201);
    pub const ACCEPTED: Self = Self(202);
    pub const NO_CONTENT: Self = Self(204);
    
    // 3xx Redirection
    pub const MOVED_PERMANENTLY: Self = Self(301);
    pub const FOUND: Self = Self(302);
    pub const NOT_MODIFIED: Self = Self(304);
    pub const TEMPORARY_REDIRECT: Self = Self(307);
    pub const PERMANENT_REDIRECT: Self = Self(308);
    
    // 4xx Client Errors
    pub const BAD_REQUEST: Self = Self(400);
    pub const UNAUTHORIZED: Self = Self(401);
    pub const PAYMENT_REQUIRED: Self = Self(402);
    pub const FORBIDDEN: Self = Self(403);
    pub const NOT_FOUND: Self = Self(404);
    pub const METHOD_NOT_ALLOWED: Self = Self(405);
    pub const NOT_ACCEPTABLE: Self = Self(406);
    pub const CONFLICT: Self = Self(409);
    pub const GONE: Self = Self(410);
    pub const LENGTH_REQUIRED: Self = Self(411);
    pub const PRECONDITION_FAILED: Self = Self(412);
    pub const PAYLOAD_TOO_LARGE: Self = Self(413);
    pub const URI_TOO_LONG: Self = Self(414);
    pub const UNSUPPORTED_MEDIA_TYPE: Self = Self(415);
    pub const TOO_MANY_REQUESTS: Self = Self(429);
    
    // 5xx Server Errors
    pub const INTERNAL_SERVER_ERROR: Self = Self(500);
    pub const NOT_IMPLEMENTED: Self = Self(501);
    pub const BAD_GATEWAY: Self = Self(502);
    pub const SERVICE_UNAVAILABLE: Self = Self(503);
    pub const GATEWAY_TIMEOUT: Self = Self(504);
}

impl StatusCode {
    pub fn is_success(&self) -> bool {
        self.0 >= 200 && self.0 < 300
    }
    
    pub fn is_redirection(&self) -> bool {
        self.0 >= 300 && self.0 < 400
    }
    
    pub fn is_client_error(&self) -> bool {
        self.0 >= 400 && self.0 < 500
    }
    
    pub fn is_server_error(&self) -> bool {
        self.0 >= 500 && self.0 < 600
    }
    
    pub fn reason_phrase(&self) -> &'static str {
        match self.0 {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.0, self.reason_phrase())
    }
}
