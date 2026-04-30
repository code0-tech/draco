use crate::{
    client::DracoRuntimeStatusService,
    config::AdapterConfig,
    store::AdapterStore,
    traits::{LoadConfig, Server as AdapterServer},
};
use code0_flow::flow_service::FlowUpdateService;
use std::{sync::Arc, time::Duration};
use tokio::{signal, task::JoinHandle, time::sleep};
use tonic::transport::Server;
use tonic_health::pb::health_server::HealthServer;
use tucana::shared::AdapterStatusConfiguration;

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
    pub fn get_server_config(&self) -> Arc<C> {
        self.context.server_config.clone()
    }

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

    pub async fn serve(
        self,
        runtime_config: Vec<AdapterStatusConfiguration>,
    ) -> anyhow::Result<()> {
        let config = self.context.adapter_config.clone();
        let mut runtime_status_service: Option<Arc<DracoRuntimeStatusService>> = None;
        let mut runtime_status_heartbeat_task: Option<JoinHandle<()>> = None;
        log::info!("Starting Draco Variant: {}", config.draco_variant);

        if !config.is_static() {
            runtime_status_service = Some(Arc::new(
                DracoRuntimeStatusService::from_url(
                    config.aquila_url.clone(),
                    config.aquila_token.clone(),
                    config.draco_variant.clone(),
                    runtime_config,
                )
                .await,
            ));

            if let Some(ser) = &runtime_status_service {
                ser.update_runtime_status_by_status(
                    tucana::shared::adapter_runtime_status::Status::NotReady,
                )
                .await;
            };

            let service_name = format!("draco-{}", config.draco_variant.to_lowercase());
            let mut definition_service = FlowUpdateService::from_url(
                config.aquila_url.clone(),
                config.definition_path.as_str(),
                config.aquila_token.clone(),
            )
            .await
            .with_definition_source(service_name);

            let mut success = false;
            let mut count = 1;
            while !success {
                success = definition_service.send_with_status().await;
                if success {
                    break;
                }

                log::warn!(
                    "Updating definitions failed, trying again in 2 secs (retry number {})",
                    count
                );
                count += 1;
                sleep(Duration::from_secs(3)).await;
            }
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

        if let Some(ser) = &runtime_status_service {
            ser.update_runtime_status_by_status(
                tucana::shared::adapter_runtime_status::Status::Running,
            )
            .await;

            if config.adapter_status_update_interval_seconds > 0 {
                let status_service = Arc::clone(ser);
                let update_interval_seconds = config.adapter_status_update_interval_seconds;
                runtime_status_heartbeat_task = Some(tokio::spawn(async move {
                    let mut interval =
                        tokio::time::interval(Duration::from_secs(update_interval_seconds));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                    // First tick is immediate; consume it so heartbeats start after the interval.
                    interval.tick().await;

                    loop {
                        interval.tick().await;
                        status_service
                            .update_runtime_status_by_status(
                                tucana::shared::adapter_runtime_status::Status::Running,
                            )
                            .await;
                    }
                }));

                log::info!(
                    "Runtime status heartbeat started (interval={}s)",
                    update_interval_seconds
                );
            } else {
                log::info!("Runtime status heartbeat is disabled");
            }
        };
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
        if let Some(handle) = runtime_status_heartbeat_task.take() {
            handle.abort();
            if let Err(err) = handle.await {
                if !err.is_cancelled() {
                    log::warn!("Runtime status heartbeat task ended unexpectedly: {}", err);
                }
            }
        }

        if let Some(ser) = &runtime_status_service {
            ser.update_runtime_status_by_status(
                tucana::shared::adapter_runtime_status::Status::Stopped,
            )
            .await;
        };

        log::info!("Draco shutdown complete");
        Ok(())
    }
}
