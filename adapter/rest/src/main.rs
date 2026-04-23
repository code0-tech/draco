use base::{
    runner::{ServerContext, ServerRunner},
    store::{FlowExecutionResult, FlowIdentifyResult},
    traits::Server as ServerTrait,
};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::{Request, Response};
use hyper::{
    StatusCode,
    body::{Bytes, Incoming},
};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::async_trait;
use tucana::shared::{
    AdapterConfiguration, RuntimeFeature, Struct, Translation, ValidationFlow, Value,
    helper::value::ToValue, value::Kind,
};

use crate::response::{error_to_http_response, value_to_http_response};

mod config;
mod content_type;
mod response;
mod route;

#[tokio::main]
async fn main() {
    let server = HttpServer {
        shutdown_tx: None,
        addr: None,
    };
    let runner = match ServerRunner::new(server).await {
        Ok(runner) => runner,
        Err(err) => panic!("Failed to create server runner: {:?}", err),
    };
    log::info!("Successfully created runner for http service");

    let addr = runner.get_server_config().port;
    let host = runner.get_server_config().host.clone();

    let featues = vec![RuntimeFeature {
                name: vec![Translation {
                    code: "en-US".to_string(),
                    content: "Rest Adapter".to_string(),
                }],
                description: vec![Translation {
                    code: "en-US".to_string(),
                    content: "A Rest-Adapter is a server that exposes resources through HTTP URLs (endpoints). Clients use methods like GET, POST, PUT, and DELETE to retrieve or modify data, typically exchanged as JSON.".to_string(),
                }],
            }];

    let configs = vec![AdapterConfiguration {
        data: Some(tucana::shared::adapter_configuration::Data::Endpoint(
            format!(
                r"{}:{}/${{project_slug}}/${{flow_setting_identifier}}",
                host, addr
            ),
        )),
    }];
    match runner.serve(featues, configs).await {
        Ok(_) => (),
        Err(err) => panic!("Failed to start server runner: {:?}", err),
    };
}

struct HttpServer {
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
    addr: Option<SocketAddr>,
}

async fn execute_flow_to_hyper_response(
    flow: ValidationFlow,
    body: Value,
    store: Arc<base::store::AdapterStore>,
) -> Response<Full<Bytes>> {
    match store.execute_flow_with_emitter(flow, Some(body)).await {
        FlowExecutionResult::Ongoing(result) => {
            log::debug!("Received first ongoing response from emitter");
            value_to_http_response(result)
        }
        FlowExecutionResult::Failed => {
            log::error!("Flow execution failed event received from emitter");
            error_to_http_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
        FlowExecutionResult::FinishedWithoutOngoing => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Full::new(Bytes::new()))
            .unwrap(),
        FlowExecutionResult::TransportError => {
            log::error!("Flow execution transport error");
            error_to_http_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

pub async fn handle_request(
    req: Request<Incoming>,
    store: Arc<base::store::AdapterStore>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let headers = req.headers().clone();

    // Read full body
    let body_bytes = match BodyExt::collect(req.into_body()).await {
        Ok(collected) => collected.to_bytes().to_vec(),
        Err(err) => {
            log::error!("Failed to read request body: {}", err);
            return Ok(error_to_http_response(
                StatusCode::BAD_REQUEST,
                "Failed to read request body",
            ));
        }
    };

    let request_body_value = match content_type::parse_body_from_headers(&headers, &body_bytes) {
        Ok(value) => value,
        Err(err) => {
            log::warn!("Failed to parse request body: {}", err);
            let status_code = match err {
                content_type::BodyParseError::UnsupportedContentType { .. } => {
                    StatusCode::UNSUPPORTED_MEDIA_TYPE
                }
                _ => StatusCode::BAD_REQUEST,
            };

            return Ok(error_to_http_response(status_code, &err.to_string()));
        }
    };

    // slug matching
    let Some(slug) = route::extract_slug_from_path(&path) else {
        return Ok(error_to_http_response(
            StatusCode::BAD_REQUEST,
            "Missing slug in path",
        ));
    };

    let pattern = format!("REST.{}.*", slug);
    let route = route::RequestRoute {
        url: path.clone(),
        method,
    };

    let resp = match store.get_possible_flow_match(pattern, route).await {
        FlowIdentifyResult::Single(flow) => {
            let mut header_fields = std::collections::HashMap::new();
            let mut fields = std::collections::HashMap::new();

            for (name, value) in headers.iter() {
                let key = name.as_str().to_owned();
                let value_str = value
                    .to_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|_| String::from_utf8_lossy(value.as_bytes()).into_owned());

                header_fields.insert(key, value_str.to_value());
            }

            if let Some(v) = request_body_value {
                fields.insert(String::from("payload"), v);
            };

            fields.insert(
                String::from("headers"),
                Value {
                    kind: Some(Kind::StructValue(Struct {
                        fields: header_fields,
                    })),
                },
            );

            let input = Value {
                kind: Some(Kind::StructValue(Struct { fields })),
            };

            execute_flow_to_hyper_response(flow, input, store).await
        }
        _ => error_to_http_response(StatusCode::NOT_FOUND, "No flow found for path"),
    };

    Ok(resp)
}

#[async_trait]
impl ServerTrait<config::HttpServerConfig> for HttpServer {
    async fn init(&mut self, ctx: &ServerContext<config::HttpServerConfig>) -> anyhow::Result<()> {
        log::info!("Initializing http server");

        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let bind = format!("{}:{}", ctx.server_config.host, ctx.server_config.port);

        self.addr = Some(
            bind.parse::<SocketAddr>()
                .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", bind, e))?,
        );

        log::debug!("Initialized with Address: {:?}", self.addr);
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<config::HttpServerConfig>) -> anyhow::Result<()> {
        let addr = self
            .addr
            .expect("cannot start tcp listener with empty address");

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("failed to bind {addr}: {e}"))?;

        // Create a receiver for this run loop
        let shutdown_tx = self
            .shutdown_tx
            .as_ref()
            .expect("shutdown_tx not initialized; init() must run first")
            .clone();
        let mut shutdown_rx = shutdown_tx.subscribe();

        loop {
            let (stream, _) = tokio::select! {
                _ = shutdown_rx.recv() => {
                    log::info!("HTTP server: shutdown received, stopping accept loop");
                    break;
                }
                res = listener.accept() => {
                    res.map_err(|e| anyhow::anyhow!("accept failed: {e}"))?
                }
            };

            let io = TokioIo::new(stream);
            let store = Arc::clone(&ctx.adapter_store);

            let mut conn_shutdown_rx = shutdown_tx.subscribe();

            tokio::spawn(async move {
                let svc = hyper::service::service_fn(move |req| {
                    let store = Arc::clone(&store);
                    async move { handle_request(req, store).await }
                });

                let conn = http1::Builder::new().serve_connection(io, svc);

                tokio::pin!(conn);

                tokio::select! {
                    res = conn.as_mut() => {
                        if let Err(err) = res {
                            log::error!("Error serving connection: {:?}", err);
                        }
                    }
                    _ = conn_shutdown_rx.recv() => {
                        conn.as_mut().graceful_shutdown();
                    }
                }
            });
        }

        Ok(())
    }
    async fn shutdown(
        &mut self,
        _ctx: &ServerContext<config::HttpServerConfig>,
    ) -> anyhow::Result<()> {
        if let Some(ref tx) = self.shutdown_tx {
            log::info!("Received a shutdown signal for Adapter Server");
            let _ = tx.send(());
        }

        Ok(())
    }
}
