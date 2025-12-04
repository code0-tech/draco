use crate::{
    request::{HeaderMap, HttpRequest},
    response::HttpResponse,
};
use std::{future::Future, net::TcpListener, pin::Pin, sync::Arc};

// Handler trait for asynchronous request handling only
pub trait AsyncHandler: Send + Sync + 'static {
    fn handle(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Option<HttpResponse>> + Send + 'static>>;
}

// Implement AsyncHandler for async closures
impl<F, Fut> AsyncHandler for F
where
    F: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<HttpResponse>> + Send + 'static,
{
    fn handle(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Option<HttpResponse>> + Send + 'static>> {
        Box::pin(self(request))
    }
}

pub struct Server {
    host: String,
    port: u16,
    handlers: Arc<Vec<Box<dyn AsyncHandler>>>,
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl Server {
    pub fn new(host: String, port: u16) -> Self {
        Server {
            host,
            port,
            handlers: Arc::new(Vec::new()),
            shutdown_tx: None,
        }
    }

    /// Register an async handler
    pub fn register_handler<H>(&mut self, handler: H)
    where
        H: AsyncHandler,
    {
        let handlers =
            Arc::get_mut(&mut self.handlers).expect("Cannot register handler after server start");
        handlers.push(Box::new(handler));
    }

    /// Register an async closure as a handler
    pub fn register_async_closure<F, Fut>(&mut self, closure: F)
    where
        F: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<HttpResponse>> + Send + 'static,
    {
        self.register_handler(closure);
    }

    pub async fn start(&mut self) {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        self.run_server(&mut shutdown_rx).await;
    }

    pub fn shutdown(&self) {
        if let Some(ref tx) = self.shutdown_tx {
            let _ = tx.send(());
        }
    }

    async fn run_server(&self, shutdown_rx: &mut tokio::sync::broadcast::Receiver<()>) {
        let url = format!("{}:{}", self.host, self.port);
        log::info!("Starting http server on {}", &url);
        let listener = match TcpListener::bind(&url) {
            Ok(listener) => {
                listener
                    .set_nonblocking(true)
                    .expect("Failed to set non-blocking");
                listener
            }
            Err(err) => panic!("Failed to bind to {}: {}", url, err),
        };

        let async_listener =
            tokio::net::TcpListener::from_std(listener).expect("Failed to create async listener");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    log::info!("Shutdown signal received, stopping server");
                    break;
                }
                stream_result = async_listener.accept() => {
                    let (stream, _) = match stream_result {
                        Ok(connection) => connection,
                        Err(err) => {
                            log::error!("Failed to accept incoming connection: {}", err);
                            continue;
                        }
                    };

                    let handlers = self.handlers.clone();

                    tokio::spawn(async move {
                        println!("New connection accepted, starting to read request...");

                        // Read HTTP request data using tokio's async methods
                        use tokio::io::{AsyncBufReadExt, BufReader};

                        let mut buf_reader = BufReader::new(stream);
                        let mut raw_http_request: Vec<String> = Vec::new();
                        let mut line = String::new();

                        // Read headers until empty line
                        while let Ok(bytes) = buf_reader.read_line(&mut line).await {
                            println!("Read {} bytes: '{}'", bytes, line.trim());
                            if bytes == 0 || line.trim().is_empty() {
                                break;
                            }
                            raw_http_request.push(line.trim().to_string());
                            line.clear();
                        }

                        println!("Finished reading request. Raw data: {:?}", raw_http_request);

                        // Parse the HTTP request manually here since we can't use convert_to_http_request with tokio stream
                        let request_result = if let Some(first_line) = raw_http_request.first() {
                            println!("Parsing first line: '{}'", first_line);
                            let parts: Vec<&str> = first_line.split_whitespace().collect();
                            println!("Split into parts: {:?}", parts);

                            if parts.len() >= 3 {
                                // Extract host from headers or use default
                                let mut host = "localhost".to_string();
                                let header_lines = raw_http_request[1..].to_vec();
                                for header_line in &header_lines {
                                    if header_line.to_lowercase().starts_with("host:") {
                                        if let Some(host_value) = header_line.split(':').nth(1) {
                                            host = host_value.trim().to_string();
                                        }
                                        break;
                                    }
                                }

                                let request = HttpRequest {
                                    method: match parts[0] {
                                        "GET" => crate::request::HttpOption::GET,
                                        "POST" => crate::request::HttpOption::POST,
                                        "PUT" => crate::request::HttpOption::PUT,
                                        "DELETE" => crate::request::HttpOption::DELETE,
                                        _ => crate::request::HttpOption::GET,
                                    },
                                    path: parts[1].to_string(),
                                    version: parts[2].to_string(),
                                    host,
                                    headers: HeaderMap::from_vec(header_lines),
                                    body: None,
                                };

                                println!("Successfully parsed request: {:?}", request);
                                Ok(request)
                            } else {
                                println!("Invalid HTTP request - not enough parts");
                                let headers = std::collections::HashMap::new();
                                Err(HttpResponse::bad_request("Invalid HTTP request".to_string(), headers))
                            }
                        } else {
                            println!("Empty HTTP request - no first line");
                            let headers = std::collections::HashMap::new();
                            Err(HttpResponse::bad_request("Empty HTTP request".to_string(), headers))
                        };

                        match request_result {
                            Ok(request) => {
                                println!("About to call handlers for request: {:?}", request);

                                // Try each handler until one handles the request
                                let mut response = None;
                                for (i, handler) in handlers.iter().enumerate() {
                                    println!("Calling handler {}", i);
                                    let handler_response = handler.handle(request.clone()).await;
                                    println!("Handler {} returned: {:?}", i, handler_response.is_some());
                                    if handler_response.is_some() {
                                        response = Some(handler_response);
                                        break;
                                    }
                                }

                                println!("Final response from handlers: {:?}", response.is_some());

                                // Default response if no handler matched
                                let http_response = match response {
                                    Some(Some(resp)) => {
                                        println!("Using handler response");
                                        resp
                                    },
                                    Some(None) => {
                                        println!("Handler returned None, using not found");
                                        let headers = std::collections::HashMap::new();
                                        HttpResponse::not_found("No handler found".to_string(), headers)
                                    }
                                    None => {
                                        println!("No handlers matched, using not found");
                                        let headers = std::collections::HashMap::new();
                                        HttpResponse::not_found("No handler found".to_string(), headers)
                                    }
                                };

                                println!("About to write response: {} bytes", http_response.to_bytes().len());

                                use tokio::io::AsyncWriteExt;
                                let mut stream = buf_reader.into_inner();
                                if let Err(e) = stream.write_all(&http_response.to_bytes()).await {
                                    println!("Failed to write response: {}", e);
                                    log::error!("Failed to write response: {}", e);
                                } else if let Err(e) = stream.flush().await {
                                    println!("Failed to flush response: {}", e);
                                    log::error!("Failed to flush response: {}", e);
                                } else {
                                    println!("Response written and flushed successfully");
                                }
                            }
                            Err(response) => {
                                println!("Request parsing failed, sending error response");
                                use tokio::io::AsyncWriteExt;
                                let mut stream = buf_reader.into_inner();
                                if let Err(e) = stream.write_all(&response.to_bytes()).await {
                                    println!("Failed to write error response: {}", e);
                                    log::error!("Failed to write error response: {}", e);
                                } else if let Err(e) = stream.flush().await {
                                    println!("Failed to flush error response: {}", e);
                                    log::error!("Failed to flush error response: {}", e);
                                } else {
                                    println!("Error response written and flushed successfully");
                                }
                            }
                        }

                        // Connection will be closed when stream goes out of scope
                        println!("Request processing completed");
                    });
                }
            }
        }
    }
}
