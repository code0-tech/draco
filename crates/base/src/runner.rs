use crate::{
    config::AdapterConfig,
    store::AdapterStore,
    traits::{LoadConfig, Server as AdapterServer},
};
use code0_flow::flow_service::FlowUpdateService;
use std::sync::Arc;
use tokio::signal;
use tonic::transport::Server;
use tonic_health::pb::health_server::HealthServer;

/// Context passed to adapter server implementations containing all shared resources
pub struct ServerContext<C: LoadConfig> {
    pub server_config: Arc<C>,
    pub adapter_config: Arc<AdapterConfig>,
    pub adapter_store: Arc<AdapterStore>,
}

/// Main server runner that manages the complete adapter lifecycle
pub struct ServerRunner<C: LoadConfig> {
    context: ServerContext<C>,
    server: Box<dyn AdapterServer<C>>,
}

impl<C: LoadConfig> ServerRunner<C> {
    pub async fn new<S: AdapterServer<C>>(server: S) -> anyhow::Result<Self> {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();

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

        Ok(Self {
            context,
            server: Box::new(server),
        })
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        let config = self.context.adapter_config.clone();
        log::info!("Starting Draco Variant: {}", config.draco_variant);

        if !config.is_static() {
            let definition_service = FlowUpdateService::from_url(
                config.aquila_url.clone(),
                config.definition_path.as_str(),
            )
            .await;
            definition_service.send().await;
        }

        let health_task = if config.with_health_service {
            let health_service =
                code0_flow::flow_health::HealthService::new(config.nats_url.clone());
            let address = format!("{}:{}", config.grpc_host, config.grpc_port).parse()?;

            log::info!(
                "Health server starting at {}:{}",
                config.grpc_host,
                config.grpc_port
            );

            Some(tokio::spawn(async move {
                if let Err(err) = Server::builder()
                    .add_service(HealthServer::new(health_service))
                    .serve(address)
                    .await
                {
                    log::error!("Health server error: {:?}", err);
                } else {
                    log::info!("Health server stopped gracefully");
                }
            }))
        } else {
            None
        };

        let ServerRunner {
            mut server,
            context,
        } = self;
        // Init the adapter server (e.g. create underlying HTTP server)
        server.init(&context).await?;
        log::info!("Draco successfully initialized.");

        #[cfg(unix)]
        let sigterm = async {
            use tokio::signal::unix::{SignalKind, signal};

            let mut term =
                signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
            term.recv().await;
        };

        #[cfg(not(unix))]
        let sigterm = std::future::pending::<()>();

        match health_task {
            Some(mut ht) => {
                tokio::select! {
                    // Main adapter server loop finished on its own
                    res = server.run(&context) => {
                        log::warn!("Adapter server finished, shutting down");
                        ht.abort();
                        res?;
                    }

                    // Health server ended first
                    _ = &mut ht => {
                        log::warn!("Health server task finished, shutting down adapter");
                        server.shutdown(&context).await?;
                    }

                    // Ctrl+C
                    _ = signal::ctrl_c() => {
                        log::info!("Ctrl+C/Exit signal received, shutting down adapter");
                        server.shutdown(&context).await?;
                        ht.abort();
                    }
                    _ = sigterm => {
                        log::info!("SIGTERM received, shutting down adapter");
                        server.shutdown(&context).await?;
                        ht.abort();
                    }
                }
            }
            None => {
                tokio::select! {
                   // Adapter server loop ends on its own
                   res = server.run(&context) => {
                       log::warn!("Adapter server finished");
                       res?;
                   }

                   // Ctrl+C
                   _ = signal::ctrl_c() => {
                       log::info!("Ctrl+C/Exit signal received, shutting down adapter");
                       server.shutdown(&context).await?;
                   }
                   _ = sigterm => {
                       log::info!("SIGTERM received, shutting down adapter");
                       server.shutdown(&context).await?;
                   }
                }
            }
        }

        log::info!("Draco shutdown complete");
        Ok(())
    }
}
