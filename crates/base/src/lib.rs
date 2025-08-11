pub mod runner;
pub mod traits;

use prost::Message;
use std::sync::Arc;
use tokio::sync::broadcast;
use tucana::shared::{ExecutionFlow, ValidationFlow, Value};
use validator::verify_flow;

use crate::traits::{IdentifiableFlow, LoadConfig};

pub struct Context<C: LoadConfig> {
    pub adapter_config: Arc<C>,
    pub adapter_store: IdentifiableAdapterStore,
    pub shutdown_rx: broadcast::Receiver<()>,
}

pub struct IdentifiableAdapterStore {
    flows: Vec<ValidationFlow>,
}

impl IdentifiableAdapterStore {
    pub fn new() -> Self {
        Self { flows: Vec::new() }
    }

    pub fn get<I: IdentifiableFlow>(&self, id: I) -> Option<ValidationFlow> {
        id.identify(&self.flows)
    }

    pub async fn validate_and_execute(
        &self,
        flow: ValidationFlow,
        input_value: Option<Value>,
    ) -> Option<Value> {
        if let Some(body) = input_value.clone() {
            let verify_result = verify_flow(flow.clone(), body);

            match verify_result {
                Ok(()) => {}
                Err(_err) => {
                    return None;
                }
            };
        }

        let client = match async_nats::connect("addrs").await {
            Ok(client) => client,
            Err(err) => {
                eprintln!("Failed to connect to NATS server: {}", err);
                return None;
            }
        };

        let uuid = uuid::Uuid::new_v4().to_string();
        let execution_flow: ExecutionFlow = Self::convert_validation_flow(flow, input_value);
        let bytes = execution_flow.encode_to_vec();
        let result = client.request(uuid, bytes.into()).await;

        match result {
            Ok(message) => {
                let value = Value::decode(message.payload);
                match value {
                    Ok(value_result) => Some(value_result),
                    Err(err) => {
                        eprintln!("Failed to decode response from NATS server: {}", err);
                        None
                    }
                }
            }
            Err(err) => {
                eprintln!("Failed to send request to NATS server: {}", err);
                None
            }
        }
    }

    fn convert_validation_flow(flow: ValidationFlow, input_value: Option<Value>) -> ExecutionFlow {
        ExecutionFlow {
            flow_id: flow.flow_id,
            starting_node: flow.starting_node,
            input_value: input_value,
        }
    }
}
