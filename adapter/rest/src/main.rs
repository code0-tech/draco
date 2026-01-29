use base::{
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
    let runner = match ServerRunner::new(server).await {
        Ok(runner) => runner,
        Err(err) => panic!("Failed to create server runner: {:?}", err),
    };
    match runner.serve().await {
        Ok(_) => (),
        Err(err) => panic!("Failed to start server runner: {:?}", err),
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
        let regex_str = flow
            .settings
            .iter()
            .find(|s| s.flow_setting_id == "HTTP_URL")
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.kind.as_ref())
            .and_then(|k| match k {
                Kind::StringValue(s) => Some(s.as_str()),
                _ => None,
            });

        let Some(regex_str) = regex_str else {
            return false;
        };

        print!(
            "Comparing regex {} with literal route: {}",
            regex_str, self.url
        );

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
        log::info!("Initializing http server");
        self.http_server = Some(Server::new(
            ctx.server_config.host.clone(),
            ctx.server_config.port,
        ));
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<HttpServerConfig>) -> anyhow::Result<()> {
        if let Some(server) = &mut self.http_server {
            log::info!("Running http server");
            server.register_async_closure({
                let store = Arc::clone(&ctx.adapter_store);
                move |request: HttpRequest| {
                    let store = Arc::clone(&store);
                    async move {
                        //Get slug => host/slug/real_path

                        let splits: Vec<_> = request.path.split("/").collect();
                        let first = splits.first();

                        if let Some(slug) = first {
                            let pattern = format!("REST.{}.*", slug);
                            let route = RequestRoute {
                                url: request.path.clone(),
                            };

                            match store.get_possible_flow_match(pattern, route).await {
                                FlowIdentifyResult::Single(flow) => {
                                    print!("Found flow: {}", flow.flow_id);
                                    execute_flow(flow, request, store).await
                                }
                                _ => Some(HttpResponse::internal_server_error(
                                    format!("No flow found for path: {}", request.path),
                                    HashMap::new(),
                                )),
                            }
                        } else {
                            Some(HttpResponse::internal_server_error(
                                format!("No flow found for path: {}", request.path),
                                HashMap::new(),
                            ))
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
    host: String,
}

impl LoadConfig for HttpServerConfig {
    fn load() -> Self {
        Self {
            port: env_with_default("HTTP_SERVER_PORT", 8082),
            host: env_with_default("HTTP_SERVER_HOST", String::from("0.0.0.0")),
        }
    }
}
