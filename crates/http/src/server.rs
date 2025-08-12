use super::request::convert_to_http_request;
use crate::{request::HttpRequest, response::HttpResponse};
use std::{future::Future, io::Write, net::TcpListener, pin::Pin, sync::Arc};

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
    port: u16,
    handlers: Arc<Vec<Box<dyn AsyncHandler>>>,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Server {
            port,
            handlers: Arc::new(Vec::new()),
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

    pub async fn start(&self) {
        let url = format!("127.0.0.1:{}", self.port);

        let listener = match TcpListener::bind(&url) {
            Ok(listener) => listener,
            Err(err) => panic!("Failed to bind to {}: {}", url, err),
        };

        for stream_result in listener.incoming() {
            let mut stream = match stream_result {
                Ok(stream) => stream,
                Err(err) => {
                    log::error!("Failed to accept incoming connection: {}", err);
                    continue;
                }
            };

            let handlers = self.handlers.clone();

            tokio::spawn(async move {
                match convert_to_http_request(&stream) {
                    Ok(request) => {
                        // Try each handler until one handles the request
                        let mut response = None;
                        for handler in handlers.iter() {
                            let handler_response = handler.handle(request.clone()).await;
                            if handler_response.is_some() {
                                response = Some(handler_response);
                                break;
                            }
                        }

                        // Default response if no handler matched
                        let http_response = match response {
                            Some(Some(resp)) => resp,
                            Some(None) => {
                                let headers = std::collections::HashMap::new();
                                HttpResponse::not_found("No handler found".to_string(), headers)
                            }
                            None => {
                                let headers = std::collections::HashMap::new();
                                HttpResponse::not_found("No handler found".to_string(), headers)
                            }
                        };

                        stream.write_all(&http_response.to_bytes()).unwrap();
                    }
                    Err(response) => {
                        stream.write_all(&response.to_bytes()).unwrap();
                    }
                };
            });
        }
    }
}
