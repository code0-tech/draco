pub mod queue;
pub mod store;

use std::{future::Future, pin::Pin, sync::Arc};

use code0_flow::{
    flow_queue::service::RabbitmqClient, flow_store::connection::create_flow_store_connection,
};
use config::FromEnv;
use http::{
    request::HttpRequest,
    response::HttpResponse,
    server::{self, AsyncHandler},
};
use queue::queue::handle_connection;
use tucana::shared::{DataType, FlowType, Translation};

pub struct FlowConnectionHandler {
    flow_store: code0_flow::flow_store::connection::FlowStore,
    rabbitmq_client: Arc<RabbitmqClient>,
}

impl FlowConnectionHandler {
    pub async fn new(config: &Config) -> Self {
        let flow_store = create_flow_store_connection(config.redis_url.clone()).await;
        let rabbitmq_client = Arc::new(RabbitmqClient::new(config.rabbitmq_url.as_str()).await);
        FlowConnectionHandler {
            flow_store,
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

#[derive(FromEnv)]
pub struct Config {
    port: u16,
    redis_url: String,
    rabbitmq_url: String,
    aquila_url: String,
    is_static: bool,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting Draco REST server");
    let config = Config::from_file("./.env");

    if !config.is_static {
        let rest_flow_type = FlowType {
            name: vec![Translation {
                code: "en-US".to_string(),
                content: "Rest Endpoint".to_string(),
            }],
            definition: None,
        };

        let data_type = DataType {
            variant: 1,
            identifier: "string".to_string(),
            rules: vec![],
            name: vec![Translation {
                code: "en-US".to_string(),
                content: "String".to_string(),
            }],
            input_types: vec![],
            return_type: None,
            parent_type_identifier: None,
        };

        let update_client =
            code0_flow::flow_definition::FlowUpdateService::from_url(config.aquila_url.clone())
                .with_data_types(vec![data_type])
                .with_flow_types(vec![rest_flow_type]);

        update_client.send().await;
    }

    let mut server = server::Server::new(config.port);

    let handler = FlowConnectionHandler::new(&config).await;
    server.register_handler(handler);
    server.start().await
}
