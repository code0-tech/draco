mod config;
pub mod queue;
pub mod store;
mod types;

use code0_flow::{
    flow_config::mode::Mode,
    flow_queue::service::RabbitmqClient,
    flow_store::{
        connection::create_flow_store_connection,
        service::{FlowStoreService, FlowStoreServiceBase},
    },
};
use http::{
    request::HttpRequest,
    response::HttpResponse,
    server::{self, AsyncHandler},
};
use queue::queue::handle_connection;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use types::{get_data_types, get_flow_types};

use crate::config::Config;

pub struct FlowConnectionHandler {
    flow_store: Arc<Mutex<FlowStoreService>>,
    rabbitmq_client: Arc<RabbitmqClient>,
}

impl FlowConnectionHandler {
    pub async fn new(config: &Config) -> Self {
        let flow_store = create_flow_store_connection(config.redis_url.clone()).await;
        let flow_store_service = Arc::new(Mutex::new(FlowStoreServiceBase::new(flow_store).await));

        let rabbitmq_client = Arc::new(RabbitmqClient::new(config.rabbitmq_url.as_str()).await);
        FlowConnectionHandler {
            flow_store: flow_store_service,
            rabbitmq_client,
        }
    }
}

impl AsyncHandler for FlowConnectionHandler {
    fn handle(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Option<HttpResponse>> + Send + 'static>> {
        let flow_store = self.flow_store.clone();
        let rabbitmq_client = self.rabbitmq_client.clone();
        Box::pin(async move { handle_connection(request, flow_store, rabbitmq_client).await })
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    code0_flow::flow_config::load_env_file();

    log::info!("Starting Draco REST server");
    let config = Config::new();

    if config.mode != Mode::STATIC {
        let update_client =
            code0_flow::flow_definition::FlowUpdateService::from_url(config.aquila_url.clone())
                .with_data_types(get_data_types())
                .with_flow_types(get_flow_types());

        update_client.send().await;
    }

    let mut server = server::Server::new(config.port);

    let handler = FlowConnectionHandler::new(&config).await;
    server.register_handler(handler);
    server.start().await
}
