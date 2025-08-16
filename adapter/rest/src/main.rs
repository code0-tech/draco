use base::{
    extract_flow_setting_field,
    runner::{ServerContext, ServerRunner},
    store::FlowIdenfiyResult,
    traits::{IdentifiableFlow, LoadConfig, Server as ServerTrait},
};
use code0_flow::flow_config::env_with_default;
use http::{request::HttpRequest, response::HttpResponse, server::Server};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::async_trait;
use tucana::shared::ValidationFlow;

#[tokio::main]
async fn main() {
    let server = HttpServer { http_server: None };
    let runner = ServerRunner::new(server).await.unwrap();
    runner.serve().await.unwrap();
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

        match regex::Regex::new(regex_str) {
            Ok(regex) => regex.is_match(&self.url),
            Err(err) => {
                log::error!("Failed to compile regex: {}", err);
                false
            }
        }
    }
}

#[async_trait]
impl ServerTrait<HttpServerConfig> for HttpServer {
    async fn init(&mut self, ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        self.http_server = Some(Server::new(ctx.server_config.port));
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        if let Some(server) = &mut self.http_server {
            server.register_async_closure({
                let store = Arc::clone(&ctx.adapter_store);
                move |request: HttpRequest| {
                    let store = Arc::clone(&store);
                    async move {
                        let pattern = format!("*.*.REST.{}.{:?}", request.host, request.method);
                        let route = RequestRoute {
                            url: request.path.clone(),
                        };

                        match store.get_possible_flow_match(pattern, route).await {
                            FlowIdenfiyResult::Single(flow) => {
                                execute_flow(flow, request, store).await
                            }
                            _ => Some(HttpResponse::internal_server_error(
                                format!("No flow found for path: {}", request.path),
                                HashMap::new(),
                            )),
                        }
                    }
                }
            });

            server.start().await;
        }
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        if let Some(server) = &self.http_server {
            server.shutdown();
        }
        Ok(())
    }
}

async fn execute_flow(
    flow: ValidationFlow,
    request: HttpRequest,
    store: Arc<base::store::AdapterStore>,
) -> Option<HttpResponse> {
    match store.validate_and_execute_flow(flow, request.body).await {
        Some(result) => {
            let json = serde_json::to_vec_pretty(&result).unwrap_or_else(|err| {
                format!(r#"{{"error": "Serialization failed: {}"}}"#, err).into_bytes()
            });
            Some(HttpResponse::ok(json, HashMap::new()))
        }
        None => Some(HttpResponse::internal_server_error(
            "Flow execution failed".to_string(),
            HashMap::new(),
        )),
    }
}

#[derive(Clone)]
struct HttpServerConfig {
    port: u16,
}

impl LoadConfig for HttpServerConfig {
    fn load() -> Self {
        Self {
            port: env_with_default("HTTP_SERVER_PORT", 8082),
        }
    }
}
