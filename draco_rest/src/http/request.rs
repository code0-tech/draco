use super::response::HttpResponse;
use crate::to_tucana_value;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    str::FromStr,
};
use tucana::shared::Value;

#[derive(Debug)]
pub enum HttpOption {
    GET,
    POST,
    PUT,
    DELETE,
}

impl FromStr for HttpOption {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HttpOption::GET),
            "POST" => Ok(HttpOption::POST),
            "PUT" => Ok(HttpOption::PUT),
            "DELETE" => Ok(HttpOption::DELETE),
            _ => Err(()),
        }
    }
}

impl ToString for HttpOption {
    fn to_string(&self) -> String {
        match self {
            HttpOption::GET => "GET".to_string(),
            HttpOption::POST => "POST".to_string(),
            HttpOption::PUT => "PUT".to_string(),
            HttpOption::DELETE => "DELETE".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpOption,
    pub path: String,
    pub version: String,
    pub headers: Vec<String>,
    pub body: Option<Value>,
}

#[derive(Debug)]
pub enum PrimitiveValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Debug)]
pub struct PrimitiveStruct {
    pub fields: HashMap<String, PrimitiveValue>,
}

#[derive(Debug)]
pub struct HttpParameter {
    pub url_query: Option<PrimitiveStruct>,
    pub url_parameters: Option<PrimitiveStruct>,
    pub body: Option<Value>,
}

pub fn convert_to_http_request(stream: &TcpStream) -> Result<HttpRequest, HttpResponse> {
    let mut buf_reader = BufReader::new(stream);
    let mut raw_http_request: Vec<String> = Vec::new();
    let mut line = String::new();

    // Read headers until empty line
    while let Ok(bytes) = buf_reader.read_line(&mut line) {
        if bytes == 0 || line.trim().is_empty() {
            break;
        }

        raw_http_request.push(line.trim().to_string());
        line.clear();
    }

    // Parse headers
    let mut http_request = parse_request(raw_http_request)?;

    // Read body if Content-Length is specified
    for header in &http_request.headers {
        if header.to_lowercase().starts_with("content-length:") {
            let content_length: usize = header
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            if content_length > 0 {
                let mut body = vec![0; content_length];
                if let Ok(_) = buf_reader.read_exact(&mut body) {
                    // Parse JSON body
                    if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&body) {
                        http_request.body = Some(to_tucana_value(json_value));
                    }
                }
            }
            break;
        }
    }

    log::debug!("Received HTTP Request: {:?}", &http_request);

    if http_request.version != "HTTP/1.1" {
        return Err(HttpResponse::not_implemented(
            "The HTTP version is not supported".to_string(),
            HashMap::new(),
        ));
    }

    Ok(http_request)
}

fn parse_request(raw_http_request: Vec<String>) -> Result<HttpRequest, HttpResponse> {
    let params = &raw_http_request[0];

    if params.is_empty() {
        return Err(HttpResponse::bad_request(
            "Empty HTTP request line".to_string(),
            HashMap::new(),
        ));
    }

    let mut header_params = params.split(" ");
    let raw_method = header_params.next().ok_or_else(|| {
        HttpResponse::bad_request("Missing HTTP method".to_string(), HashMap::new())
    })?;
    let path = header_params.next().ok_or_else(|| {
        HttpResponse::bad_request("Missing request path".to_string(), HashMap::new())
    })?;
    let version = header_params.next().ok_or_else(|| {
        HttpResponse::bad_request("Missing HTTP version".to_string(), HashMap::new())
    })?;

    let method = match HttpOption::from_str(raw_method) {
        Ok(method) => method,
        Err(_) => {
            return Err(HttpResponse::method_not_allowed(
                format!("Unsupported HTTP method: {}", raw_method),
                HashMap::new(),
            ))
        }
    };

    Ok(HttpRequest {
        method,
        path: path.to_string(),
        version: version.to_string(),
        headers: raw_http_request.clone(),
        body: None,
    })
}
