use base::{
    runner::{ServerContext, ServerRunner},
    store::FlowIdentifyResult,
    traits::Server as ServerTrait,
};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response};
use hyper::{
    StatusCode,
    body::{Bytes, Incoming},
};
use hyper::{
    header::{HeaderName, HeaderValue},
    server::conn::http1,
};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::async_trait;
use tucana::shared::value::Kind::StructValue;
use tucana::shared::{ListValue, value::Kind};
use tucana::shared::{Struct, ValidationFlow, Value};

mod config;
mod route;

#[tokio::main]
async fn main() {
    let server = HttpServer { addr: None };
    let runner = match ServerRunner::new(server).await {
        Ok(runner) => runner,
        Err(err) => panic!("Failed to create server runner: {:?}", err),
    };
    log::info!("Successfully created runner for http service");
    match runner.serve().await {
        Ok(_) => (),
        Err(err) => panic!("Failed to start server runner: {:?}", err),
    };
}

struct HttpServer {
    addr: Option<SocketAddr>,
}

fn json_error(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
    let body = format!(r#"{{"error": "{}"}}"#, msg);
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

fn build_response(
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> Response<Full<Bytes>> {
    let mut builder = Response::builder().status(status);

    {
        let h = builder.headers_mut().unwrap();
        for (k, v) in headers {
            let name = match HeaderName::from_bytes(k.as_bytes()) {
                Ok(n) => n,
                Err(_) => {
                    log::warn!("Dropping invalid header name: {}", k);
                    continue;
                }
            };

            let value = match HeaderValue::from_str(&v) {
                Ok(v) => v,
                Err(_) => {
                    log::warn!("Dropping invalid header value for {}: {:?}", k, v);
                    continue;
                }
            };

            h.insert(name, value);
        }
    }

    builder.body(Full::new(Bytes::from(body))).unwrap()
}

async fn execute_flow_to_hyper_response(
    flow: ValidationFlow,
    body: Vec<u8>,
    store: Arc<base::store::AdapterStore>,
) -> Response<Full<Bytes>> {
    let value: Option<Value> = if body.is_empty() {
        None
    } else {
        match prost::Message::decode(body.as_slice()) {
            Ok(v) => Some(v),
            Err(e) => {
                log::warn!("Failed to decode request body as protobuf Value: {}", e);
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "Failed to decode request body as protobuf Value",
                );
            }
        }
    };

    match store.validate_and_execute_flow(flow, value).await {
        Some(result) => {
            log::debug!("Recieved Result: {:?}", result);
            let Value {
                kind: Some(StructValue(Struct { fields })),
            } = result
            else {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Flow result was not a struct",
                );
            };

            let Some(headers_val) = fields.get("headers") else {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Flow result missing headers",
                );
            };
            let Some(status_code_val) = fields.get("status_code") else {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Flow result missing status_code",
                );
            };
            let Some(payload_val) = fields.get("payload") else {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Flow result missing payload",
                );
            };

            // headers struct
            let Value {
                kind:
                    Some(Kind::ListValue(ListValue {
                        values: header_fields,
                    })),
            } = headers_val
            else {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "headers was not a list of header entries",
                );
            };

            let http_headers: HashMap<String, String> = header_fields
                .iter()
                .filter_map(|x| {
                    if let Value {
                        kind: Some(StructValue(Struct { fields: f })),
                    } = x
                    {
                        Some(f)
                    } else {
                        None
                    }
                })
                .filter_map(|f| {
                    let key = match f.get("key") {
                        Some(value) => {
                            if let Value {
                                kind: Some(Kind::StringValue(x)),
                            } = value
                            {
                                x
                            } else {
                                return None;
                            }
                        }
                        None => return None,
                    };
                    let value = match f.get("value") {
                        Some(value) => {
                            if let Value {
                                kind: Some(Kind::StringValue(x)),
                            } = value
                            {
                                x
                            } else {
                                return None;
                            }
                        }
                        None => return None,
                    };

                    Some((key.clone(), value.clone()))
                })
                .collect();

            // status_code number
            let Some(Kind::NumberValue(code)) = status_code_val.kind else {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "status_code was not a number",
                );
            };

            // payload -> json bytes
            let json = serde_json::to_vec_pretty(payload_val).unwrap_or_else(|err| {
                format!(r#"{{"error":"Serialization failed: {}"}}"#, err).into_bytes()
            });

            let status =
                StatusCode::from_u16(code as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            build_response(status, http_headers, json)
        }
        None => {
            log::error!("flow execution failed");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Flow execution failed")
        }
    }
}

pub async fn handle_request(
    req: Request<Incoming>,
    store: Arc<base::store::AdapterStore>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Read full body
    let body_bytes = match BodyExt::collect(req.into_body()).await {
        Ok(collected) => collected.to_bytes().to_vec(),
        Err(err) => {
            log::error!("Failed to read request body: {}", err);
            return Ok(json_error(
                StatusCode::BAD_REQUEST,
                "Failed to read request body",
            ));
        }
    };

    // slug matching
    let Some(slug) = route::extract_slug_from_path(&path) else {
        return Ok(json_error(StatusCode::BAD_REQUEST, "Missing slug in path"));
    };

    let pattern = format!("REST.{}.*", slug);
    let route = route::RequestRoute {
        url: path.clone(),
        method,
    };

    let resp = match store.get_possible_flow_match(pattern, route).await {
        FlowIdentifyResult::Single(flow) => {
            execute_flow_to_hyper_response(flow, body_bytes, store).await
        }
        _ => json_error(StatusCode::NOT_FOUND, "No flow found for path"),
    };

    Ok(resp)
}

#[async_trait]
impl ServerTrait<config::HttpServerConfig> for HttpServer {
    async fn init(&mut self, ctx: &ServerContext<config::HttpServerConfig>) -> anyhow::Result<()> {
        log::info!("Initializing http server");
        let bind = format!("{}:{}", ctx.server_config.host, ctx.server_config.port);

        self.addr = Some(
            bind.parse::<SocketAddr>()
                .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", bind, e))?,
        );

        log::debug!("Initizalized with Address: {:?}", self.addr);
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<config::HttpServerConfig>) -> anyhow::Result<()> {
        let addr = match self.addr {
            Some(addr) => addr,
            None => panic!("cannot start tcp listener with empty address"),
        };

        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(err) => {
                panic!("failed to register tcp listener on address: {:?}", err);
            }
        };

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(res) => res,
                Err(e) => {
                    panic!("listener failed to accept requests: {:?}", e);
                }
            };

            let io = TokioIo::new(stream);

            let store = Arc::clone(&ctx.adapter_store);

            tokio::task::spawn(async move {
                let store = Arc::clone(&store);
                let svc = hyper::service::service_fn(move |req| {
                    let store = Arc::clone(&store);
                    async move { handle_request(req, store).await }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                    log::error!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    async fn shutdown(
        &mut self,
        _ctx: &ServerContext<config::HttpServerConfig>,
    ) -> anyhow::Result<()> {
        todo!("Implement shutdown!");
        Ok(())
    }
}
