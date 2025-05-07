use super::response::HttpResponse;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    str::FromStr,
    usize,
};
use tucana::shared::{helper::value::from_json_value, value::Kind, Struct, Value};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct HeaderMap {
    pub fields: HashMap<String, String>,
}

impl HeaderMap {
    pub fn new() -> Self {
        HeaderMap {
            fields: HashMap::new(),
        }
    }

    /// Create a new HeaderMap from a vector of strings.
    ///
    /// Each string should be in the format "key: value".
    ///
    /// # Examples
    ///
    /// ```
    /// use http::request::HeaderMap;
    ///
    /// let header = vec![
    ///     "Content-Type: application/json".to_string(),
    ///     "User-Agent: Mozilla/5.0".to_string(),
    /// ];
    /// let header_map = HeaderMap::from_vec(header);
    /// assert_eq!(header_map.get("content-type"), Some(&"application/json".to_string()));
    /// assert_eq!(header_map.get("user-agent"), Some(&"mozilla/5.0".to_string()));
    /// ```
    pub fn from_vec(header: Vec<String>) -> Self {
        let mut header_map = HeaderMap::new();

        for param in header {
            let mut parts = param.split(": ");
            let key = match parts.next() {
                Some(key) => key.to_lowercase(),
                None => continue,
            };
            let value = match parts.next() {
                Some(value) => value.to_lowercase(),
                None => continue,
            };

            header_map.add(key, value);
        }

        header_map
    }

    #[inline]
    pub fn add(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.fields.get(key)
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpOption,
    pub path: String,
    pub version: String,
    pub host: String,
    pub headers: HeaderMap,

    /// The body of the request.
    ///
    /// # Example
    /// If the url was called:
    ///
    /// url: .../api/users/123/posts/456?filter=recent&sort=asc
    ///
    /// from the regex: "^/api/users/(?P<user_id>\d+)/posts/(?P<post_id>\d+)(\?.*)?$"
    ///
    /// With the request body:
    ///
    /// ```json
    /// {
    ///    "first": 2,
    ///    "second": 300
    /// }
    /// ```
    /// The equivalent HTTP request body will look like:
    /// ```json
    /// {
    /// "url": {
    ///     "user_id": "123",
    ///     "post_id": "456",
    /// },
    /// "query": {
    ///     "filter": "recent",
    ///     "sort": "asc"
    /// },
    ///     "body": {
    ///         "first": "1",
    ///         "second": "2"
    ///     }
    /// }
    /// ```
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
    let http_request = parse_request(raw_http_request, buf_reader)?;

    log::debug!("Received HTTP Request: {:?}", &http_request);

    if http_request.version != "HTTP/1.1" {
        return Err(HttpResponse::not_implemented(
            "The HTTP version is not supported".to_string(),
            HashMap::new(),
        ));
    }

    Ok(http_request)
}

#[inline]
fn parse_request(
    raw_http_request: Vec<String>,
    mut buf_reader: BufReader<&TcpStream>,
) -> Result<HttpRequest, HttpResponse> {
    let params = &raw_http_request[0];
    let headers = raw_http_request[1..raw_http_request.len()].to_vec();
    let header_map = HeaderMap::from_vec(headers);

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

    let mut body_values: HashMap<String, Value> = HashMap::new();

    if let Some(content_length) = header_map.get("content-length") {
        let size: usize = match content_length.parse() {
            Ok(len) => len,
            Err(_) => {
                return Err(HttpResponse::bad_request(
                    "Invalid content-length header".to_string(),
                    HashMap::new(),
                ))
            }
        };

        let mut body = vec![0; size];
        if let Ok(_) = buf_reader.read_exact(&mut body) {
            if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&body) {
                body_values.insert("body".to_string(), from_json_value(json_value));
            }
        }
    };

    if path.contains("?") {
        let mut fields: HashMap<String, Value> = HashMap::new();
        if let Some((_, query)) = path.split_once("?") {
            let values = query.split("&");

            for value in values {
                let mut parts = value.split("=");
                let key = match parts.next() {
                    Some(key) => key.to_string(),
                    None => continue,
                };

                let value = match parts.next() {
                    Some(value) => value.to_string(),
                    None => continue,
                };

                fields.insert(
                    key,
                    Value {
                        kind: Some(Kind::StringValue(value)),
                    },
                );
            }
        };

        if !fields.is_empty() {
            let value = Value {
                kind: Some(Kind::StructValue(Struct { fields })),
            };

            body_values.insert("query".to_string(), value);
        }
    };

    let body = if !body_values.is_empty() {
        Some(Value {
            kind: Some(Kind::StructValue(Struct {
                fields: body_values,
            })),
        })
    } else {
        None
    };

    let host = {
        match header_map.get("host") {
            Some(host) => host.clone(),
            None => {
                return Err(HttpResponse::bad_request(
                    "Missing Host in Headers!".to_string(),
                    HashMap::new(),
                ));
            }
        }
    };

    Ok(HttpRequest {
        method,
        path: path.to_string(),
        version: version.to_string(),
        host,
        headers: header_map,
        body,
    })
}
