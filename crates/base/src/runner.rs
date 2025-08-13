use crate::{
    config::AdapterConfig,
    store::AdapterStore,
    traits::{LoadConfig, Server as AdapterServer},
};
use code0_flow::flow_definition::FlowUpdateService;
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::transport::Server;
use tonic_health::pb::health_server::HealthServer;

/*
 * The ServerRunner is intended to be used as a wrapper around a server implementation.
 *  - It will load all environment variables.
 *  - It will load the general configuration and connect to NATS.
 *  - It will expose the HealthStatus endpoint via grpc.
 *  - It will gracefully shutdown the server on Ctrl+C.
 */

pub struct ServerContext<C: LoadConfig> {
    pub server_config: Arc<C>,
    pub adapter_config: Arc<AdapterConfig>,
    pub adapter_store: Arc<AdapterStore>,
}

pub struct ServerRunner<C: LoadConfig> {
    context: ServerContext<C>,
    server: Box<dyn AdapterServer<C>>,
    shutdown_sender: broadcast::Sender<()>,
}

impl<C: LoadConfig> ServerRunner<C> {
    /// Load config via `C::load()`, box your server impl.
    pub async fn new<S: AdapterServer<C>>(server: S) -> anyhow::Result<Self> {
        code0_flow::flow_config::load_env_file();

        let adapter_config = AdapterConfig::from_env();
        let server_config = C::load();
        let adapter_store = AdapterStore::from_url(
            adapter_config.nats_url.clone(),
            adapter_config.nats_bucket.clone(),
        )
        .await;
        let context = ServerContext {
            adapter_store: Arc::new(adapter_store),
            adapter_config: Arc::new(adapter_config),
            server_config: Arc::new(server_config),
        };

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            context,
            server: Box::new(server),
            shutdown_sender: shutdown_tx,
        })
    }

    /// Run init, spawn `run` with cancel, catch Ctrl+C, then shutdown.
    pub async fn serve(mut self) -> anyhow::Result<()> {
        let config = self.context.adapter_config.clone();
        if !config.is_static() {
            let definition_service = FlowUpdateService::from_url(
                config.aquila_url.clone(),
                config.definition_path.as_str(),
            );

            definition_service.send().await;
        }

        if config.is_monitored {
            let health_service =
                code0_flow::flow_health::HealthService::new(config.nats_url.clone());

            if let Ok(address) = format!("127.0.0.1:{}", config.grpc_port).parse() {
                println!("Health server started at {}", address);
                let _ = Server::builder()
                    .add_service(HealthServer::new(health_service))
                    .serve(address)
                    .await;
            } else {
                println!("Failed to parse address, starting without health server");
            }

            todo!("Start the HealthServer");
        }

        // 1) init
        self.server.init(&self.context).await?;

        // 2) run + listen for shutdown
        let mut rx = self.shutdown_sender.subscribe();
        let mut srv = self.server;
        let handle = tokio::spawn(async move {
            tokio::select! {
                res = srv.run(&self.context) => res,
                _ = rx.recv() => srv.shutdown(&self.context).await,
            }
        });

        // 3) wait Ctrl+C
        tokio::signal::ctrl_c().await?;
        let _ = self.shutdown_sender.send(());

        // 4) wait task
        handle.await??;
        Ok(())
    }
}
