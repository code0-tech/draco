use std::sync::Arc;

use code0_flow::flow_store::{
    connection::create_flow_store_connection,
    service::{FlowStoreService, FlowStoreServiceBase},
};
use tokio::sync::Mutex;
use tucana::shared::Flows;
use typed_flows::{add_flow::get_add_rest_flow, mutiply_flow::get_multiply_rest_flow};

pub mod typed_data_types;
pub mod typed_flows;

/*
    Helper Service to insert typed flows into the FlowStore.
*/

#[tokio::main]
async fn main() {
    let redis_url = String::from("redis://localhost:6379");
    let flow_store = create_flow_store_connection(redis_url).await;
    let flow_store_client = Arc::new(Mutex::new(FlowStoreService::new(flow_store).await));

    let mut client = flow_store_client.lock().await;
    let _ = client
        .insert_flows(Flows {
            flows: vec![get_add_rest_flow(), get_multiply_rest_flow()],
        })
        .await;
}
