use base::{
    extract_flow_setting_field,
    runner::{ServerContext, ServerRunner},
    store::FlowIdentifyResult,
    traits::{IdentifiableFlow, LoadConfig, Server as ServerTrait},
};
use code0_flow::flow_config::env_with_default;
use http::{request::HttpRequest, response::HttpResponse, server::Server};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::async_trait;
use tucana::shared::value::Kind;
use tucana::shared::value::Kind::StructValue;
use tucana::shared::{Struct, ValidationFlow, Value};

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
                            FlowIdentifyResult::Single(flow) => {
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
            let Value {
                kind: Some(StructValue(Struct { fields })),
            } = result
            else {
                return None;
            };

            let Some(headers) = fields.get("headers") else {
                return None;
            };

            let Some(status_code) = fields.get("status_code") else {
                return None;
            };

            let Some(payload) = fields.get("payload") else {
                return None;
            };

            let Value {
                kind:
                    Some(StructValue(Struct {
                        fields: header_fields,
                    })),
            } = headers
            else {
                return None;
            };
            let http_headers: HashMap<String, String> = header_fields
                .iter()
                .filter_map(|(k, v)| {
                    let value = match &v.kind {
                        Some(Kind::StringValue(s)) if !s.is_empty() => s.clone(),
                        _ => return None,
                    };

                    Some((k.clone(), value))
                })
                .collect();

            let json = serde_json::to_vec_pretty(&payload).unwrap_or_else(|err| {
                format!(r#"{{"error": "Serialization failed: {}"}}"#, err).into_bytes()
            });

            let Some(Kind::NumberValue(code)) = status_code.kind else {
                return None;
            };
            Some(HttpResponse::new(code as u16, http_headers.clone(), json))
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
