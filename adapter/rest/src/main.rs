use base::{
    runner::{ServerContext, ServerRunner},
    traits::Server as ServerTrait,
};
use code0_flow::flow_service::ModuleDefinitionAppendix;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::async_trait;
use tucana::shared::{Endpoint, ModuleDefinition};

mod auth;
mod config;
mod content_type;
mod request;
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

    let external_addr = runner.get_server_config().external_port;
    let external_host = runner.get_server_config().external_host.clone();

    let configs = vec![ModuleDefinitionAppendix {
        module_identifier: String::from("draco-rest"),
        definitions: vec![ModuleDefinition {
            flow_type_identifier: vec![String::from("REST")],
            value: Some(tucana::shared::module_definition::Value::Endpoint(
                Endpoint {
                    host: external_host,
                    port: external_addr as i64,
                    endpoint: String::from(r"/${{project_slug}}${{httpURL}}"),
                },
            )),
        }],
    }];
    match runner.serve(configs).await {
        Ok(_) => (),
        Err(err) => panic!("Failed to start server runner: {:?}", err),
    };
}

struct HttpServer {
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
    addr: Option<SocketAddr>,
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
                    async move { request::handle(req, store).await }
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
