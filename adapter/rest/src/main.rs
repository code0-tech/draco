use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use base::{
    runner::ServerContext,
    runner::ServerRunner,
    traits::{LoadConfig, Server as ServerTrait},
};
use http::{request::HttpRequest, response::HttpResponse, server::Server};
use tonic::async_trait;

#[tokio::main]
async fn main() {
    print!("Starting server!");

    let server = HttpServer { http_server: None };
    let runner = ServerRunner::new(server).await;

    let _ = match runner {
        Ok(s) => s.serve().await,
        Err(e) => Err(e),
    };
}

struct HttpServer {
    http_server: Option<Server>,
}

#[async_trait]
impl ServerTrait<HttpServerConfig> for HttpServer {
    async fn init(&mut self, ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        self.http_server = Some(Server::new(ctx.server_config.port));
        Ok(())
    }

    /// The "serve forever" loop.
    async fn run(&mut self, _ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        if let Some(server) = &mut self.http_server {
            let counter = Arc::new(Mutex::new(0));

            server.register_async_closure({
                let counter = Arc::clone(&counter);
                move |request: HttpRequest| {
                    let counter = Arc::clone(&counter);
                    async move {
                        let mut number = counter.lock().await;
                        *number += 1;

                        println!("Received request: {:?}", request);

                        let headers = HashMap::new();
                        Some(HttpResponse::ok(
                            format!("Hello from REST server! {}", number).into_bytes(),
                            headers,
                        ))
                    }
                }
            });
            server.start().await;
        };

        Ok(())
    }

    /// Called on shutdown signal.
    async fn shutdown(&mut self, _ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        todo!("shutdown http server");
    }
}

#[derive(Clone)]
struct HttpServerConfig {
    port: u16,
}

impl LoadConfig for HttpServerConfig {
    fn load() -> Self {
        HttpServerConfig { port: 8080 }
    }
}
