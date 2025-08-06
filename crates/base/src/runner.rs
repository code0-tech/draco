use crate::{Context, LoadConfig, traits::Server};
use code0_flow::flow_config::{environment::Environment, mode::Mode};
use std::sync::Arc;
use tokio::sync::broadcast;

/*
 * The ServerRunner is intended to be used as a wrapper around a server implementation.
 *  - It will load all environment variables.
 *  - It will load the general configuration and connect to NATS.
 *  - It will expose the HealthStatus endpoint via grpc.
 *  - It will gracefully shutdown the server on Ctrl+C.
 */

pub struct ServerConfig {
    pub environment: code0_flow::flow_config::environment::Environment,
    pub mode: code0_flow::flow_config::mode::Mode,
    pub nats_url: String,
    pub grpc_port: u16,
    pub aquila_url: String,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let nats_url = code0_flow::flow_config::env_with_default(
            "NATS_URL",
            String::from("nats://localhost:4222"),
        );
        let grpc_port = code0_flow::flow_config::env_with_default("GRPC_PORT", 50051);
        let aquila_url = code0_flow::flow_config::env_with_default(
            "AQUILA_URL",
            String::from("grpc://localhost:50051"),
        );

        let environment =
            code0_flow::flow_config::env_with_default("ENVIRONMENT", Environment::Development);
        let mode = code0_flow::flow_config::env_with_default("MODE", Mode::STATIC);

        Self {
            environment,
            mode,
            nats_url,
            grpc_port,
            aquila_url,
        }
    }
}

pub struct ServerRunner<C: LoadConfig> {
    config: C,
    server_config: ServerConfig,
    server: Box<dyn Server<C>>,
}

impl<C: LoadConfig> ServerRunner<C> {
    /// Load config via `C::load()`, box your server impl.
    pub fn new<S: Server<C>>(server: S) -> anyhow::Result<Self> {
        code0_flow::flow_config::load_env_file();
        let server_config = ServerConfig::from_env();
        let config = C::load()?;
        Ok(Self {
            config,
            server_config,
            server: Box::new(server),
        })
    }

    /// Run init, spawn `run` with cancel, catch Ctrl+C, then shutdown.
    pub async fn serve(mut self) -> anyhow::Result<()> {
        todo!("Load Definitions --> Send to aquila");
        todo!("Start HEalthServer");
        let (shutdown_tx, _) = broadcast::channel(1);
        let ctx = Context {
            adapter_config: Arc::new(self.config),
            shutdown_rx: shutdown_tx.subscribe(),
        };

        // 1) init
        self.server.init(&ctx).await?;

        // 2) run + listen for shutdown
        let mut rx = shutdown_tx.subscribe();
        let mut srv = self.server;
        let handle = tokio::spawn(async move {
            tokio::select! {
                res = srv.run(&ctx) => res,
                _ = rx.recv() => srv.shutdown(&ctx).await,
            }
        });

        // 3) wait Ctrl+C
        tokio::signal::ctrl_c().await?;
        let _ = shutdown_tx.send(());

        // 4) wait task
        handle.await??;
        Ok(())
    }
}
