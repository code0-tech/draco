pub struct HttpResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(
        status_code: u16,
        headers: std::collections::HashMap<String, String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            status_code,
            headers,
            body,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status_code,
            status_text(self.status_code)
        );

        let mut headers_str = String::new();
        for (key, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", key, value));
        }

        let mut response = Vec::new();
        response.extend_from_slice(status_line.as_bytes());
        response.extend_from_slice(headers_str.as_bytes());
        response.extend_from_slice(b"\r\n");
        response.extend_from_slice(&self.body);

        response
    }

    // 2xx Success responses
    pub fn ok(body: Vec<u8>, mut headers: std::collections::HashMap<String, String>) -> Self {
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(200, headers, body)
    }

    pub fn created(body: Vec<u8>, mut headers: std::collections::HashMap<String, String>) -> Self {
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(201, headers, body)
    }

    pub fn accepted(body: Vec<u8>, mut headers: std::collections::HashMap<String, String>) -> Self {
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(202, headers, body)
    }

    pub fn no_content(headers: std::collections::HashMap<String, String>) -> Self {
        Self::new(204, headers, Vec::new())
    }

    // 4xx Client Errors
    pub fn bad_request(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(400, headers, body)
    }

    pub fn unauthorized(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(401, headers, body)
    }

    pub fn forbidden(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(403, headers, body)
    }

    pub fn not_found(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(404, headers, body)
    }

    pub fn method_not_allowed(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(405, headers, body)
    }

    pub fn conflict(error: String, mut headers: std::collections::HashMap<String, String>) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(409, headers, body)
    }

    pub fn gone(error: String, mut headers: std::collections::HashMap<String, String>) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(410, headers, body)
    }

    pub fn unprocessable_entity(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(422, headers, body)
    }

    pub fn too_many_requests(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(429, headers, body)
    }

    // 5xx Server Errors
    pub fn internal_server_error(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(500, headers, body)
    }

    pub fn not_implemented(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(501, headers, body)
    }

    pub fn bad_gateway(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(502, headers, body)
    }

    pub fn service_unavailable(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(503, headers, body)
    }

    pub fn gateway_timeout(
        error: String,
        mut headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let body = format!("{{\"error\":\"{}\"}}", error).into_bytes();
        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self::new(504, headers, body)
    }

    // Check if response is successful (2xx status code)
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    // Check if response is client error (4xx status code)
    pub fn is_client_error(&self) -> bool {
        self.status_code >= 400 && self.status_code < 500
    }

    // Check if response is server error (5xx status code)
    pub fn is_server_error(&self) -> bool {
        self.status_code >= 500 && self.status_code < 600
    }
}

// Helper function to get status text from status code
fn status_text(status_code: u16) -> &'static str {
    match status_code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        410 => "Gone",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown Status",
    }
}
