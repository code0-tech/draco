use base::{
    extract_flow_setting_field,
    runner::{ServerContext, ServerRunner},
    store::FlowIdenfiyResult,
    traits::{IdentifiableFlow, LoadConfig, Server as ServerTrait},
};
use http::{request::HttpRequest, response::HttpResponse, server::Server};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::async_trait;
use tucana::shared::ValidationFlow;

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

struct RequestRoute {
    url: String,
}

impl IdentifiableFlow for RequestRoute {
    fn identify(&self, flow: &ValidationFlow) -> bool {
        let url = extract_flow_setting_field(&flow.settings, "HTTP_URL", "url");

        let regex_str = match url.as_deref() {
            Some(s) => s,
            None => return false,
        };

        let regex = match regex::Regex::new(regex_str) {
            Ok(regex) => regex,
            Err(err) => {
                log::error!("Failed to compile regex: {}", err);
                return false;
            }
        };

        regex.is_match(&self.url)
    }
}

#[async_trait]
impl ServerTrait<HttpServerConfig> for HttpServer {
    async fn init(&mut self, ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        self.http_server = Some(Server::new(ctx.server_config.port));
        Ok(())
    }

    /// The "serve forever" loop.
    async fn run(&mut self, ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        if let Some(server) = &mut self.http_server {
            println!("Registering async closure handler...");

            server.register_async_closure({
                let store = Arc::clone(&ctx.adapter_store);
                move |request: HttpRequest| {
                    let store = Arc::clone(&store);
                    async move {
                        println!("Handler called with request: {:?}", &request);

                        let pattern =
                            format!("*::*::{}::{}", request.host, request.method.to_string());
                        println!("Pattern created: {}", pattern);

                        let route = RequestRoute {
                            url: request.path.clone(),
                        };

                        println!("About to call get_possible_flow_match...");
                        let identification_result =
                            store.get_possible_flow_match(pattern, route).await;
                        println!("Flow identification completed");

                        match identification_result {
                            FlowIdenfiyResult::Single(_flow) => {
                                println!("Single flow found, returning success response");
                                //TODO: Implement flow execution logic
                                //let execution_result = ctx
                                //    .adapter_store
                                //        .validate_and_execute_flow(flow, None)
                                //        .await;

                                let headers = HashMap::new();
                                let response = Some(HttpResponse::ok(
                                    String::from("Flow executed successfully!").into_bytes(),
                                    headers,
                                ));
                                println!("Returning response: {:?}", response.is_some());
                                return response;
                            }
                            _ => {
                                println!("No single flow found, returning default response");
                                let headers = HashMap::new();
                                let response = Some(HttpResponse::internal_server_error(
                                    format!("No Flow found for path: {}", request.path),
                                    headers,
                                ));
                                println!("Returning response: {:?}", response.is_some());
                                return response;
                            }
                        }
                    }
                }
            });

            println!("Starting HTTP server...");
            server.start().await;
        };

        Ok(())
    }

    /// Called on shutdown signal.
    async fn shutdown(&mut self, _ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        if let Some(server) = &self.http_server {
            server.shutdown();
        }
        Ok(())
    }
}

#[derive(Clone)]
struct HttpServerConfig {
    port: u16,
}

impl LoadConfig for HttpServerConfig {
    fn load() -> Self {
        HttpServerConfig { port: 8081 }
    }
}
