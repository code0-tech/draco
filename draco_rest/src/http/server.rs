use code0_flow::{
    flow_queue::service::RabbitmqClient, flow_store::connection::create_flow_store_connection,
};

use crate::{queue::queue::handle_connection, Config};

use super::request::convert_to_http_request;
use std::{io::Write, net::TcpListener, sync::Arc};

pub struct Server {
    pub config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Server { config }
    }

    pub async fn start(&self) {
        let url = format!("127.0.0.1:{}", self.config.port);
        let listener = match TcpListener::bind(&url) {
            Ok(listener) => listener,
            Err(err) => panic!("Failed to bind to {}: {}", url, err),
        };

        let flow_store = create_flow_store_connection(self.config.redis_url.clone()).await;
        let rabbitmq_client =
            Arc::new(RabbitmqClient::new(self.config.rabbitmq_url.as_str()).await);

        for stream_result in listener.incoming() {
            let mut stream = match stream_result {
                Ok(stream) => stream,
                Err(err) => {
                    log::error!("Failed to accept incoming connection: {}", err);
                    continue;
                }
            };

            let flow_store_clone = flow_store.clone();
            let rabbitmq_client_clone = rabbitmq_client.clone();

            tokio::spawn(async move {
                match convert_to_http_request(&stream) {
                    Ok(request) => {
                        let response =
                            handle_connection(request, flow_store_clone, rabbitmq_client_clone)
                                .await;

                        stream.write_all(&response.to_bytes()).unwrap();
                    }
                    Err(response) => {
                        stream.write_all(&response.to_bytes()).unwrap();
                    }
                };
            });
        }
    }
}
