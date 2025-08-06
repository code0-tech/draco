pub mod runner;
pub mod traits;

use std::sync::Arc;
use tokio::sync::broadcast;
use tucana::shared::{ExecutionFlow, ValidationFlow, Value};

use crate::traits::{IdentifiableFlow, LoadConfig};

pub struct Context<C: LoadConfig> {
    pub adapter_config: Arc<C>,
    pub shutdown_rx: broadcast::Receiver<()>,
}

pub struct IdentifiableAdapterStore {
    flows: Vec<ValidationFlow>,
}

pub trait AdapterServer {
    fn new() -> Self;
    fn start(&self);
}

impl IdentifiableAdapterStore {
    pub fn new() -> Self {
        Self { flows: Vec::new() }
    }

    pub fn get<I: IdentifiableFlow>(&self, id: I) -> Option<ValidationFlow> {
        id.identify(&self.flows)
    }
}

pub fn convert_validation_flow(flow: ValidationFlow, input_value: Option<Value>) -> ExecutionFlow {
    ExecutionFlow {
        flow_id: flow.flow_id,
        starting_node: flow.starting_node,
        input_value: input_value,
    }
}
