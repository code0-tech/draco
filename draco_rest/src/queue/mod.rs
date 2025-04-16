pub mod queue {
    use crate::{
        http::{request::HttpRequest, response::HttpResponse},
        store::store::check_flow_exists,
    };
    use code0_flow::{
        flow_queue::service::{Message, RabbitmqClient},
        flow_store::connection::FlowStore,
    };
    use draco_validator::{resolver::flow_resolver::resolve_flow, verify_flow};
    use std::{collections::HashMap, sync::Arc, time::Duration};
    use tucana::shared::Flow;

    fn create_rest_message(message_content: String) -> Message {
        Message {
            message_type: code0_flow::flow_queue::service::MessageType::ExecuteFlow,
            sender: code0_flow::flow_queue::service::Sender {
                name: "draco_rest".to_string(),
                protocol: "HTTP".to_string(),
                version: "1.1".to_string(),
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            message_id: uuid::Uuid::new_v4().to_string(),
            body: message_content,
        }
    }

    pub async fn handle_connection(
        request: HttpRequest,
        flow_store: FlowStore,
        rabbitmq_client: Arc<RabbitmqClient>,
    ) -> HttpResponse {
        // Check if a flow exists for the given settings
        let flow_exists = check_flow_exists(&flow_store, &request).await;

        let flow_string = match flow_exists {
            Some(flow) => flow,
            None => {
                return HttpResponse::not_found(
                    "The given route does not exist".to_string(),
                    HashMap::new(),
                )
            }
        };

        let flow = match serde_json::from_str::<Vec<Flow>>(flow_string.as_str()) {
            Ok(flow) => flow[0].clone(),
            Err(_) => {
                return HttpResponse::internal_server_error(
                    "Internal Server Error".to_string(),
                    HashMap::new(),
                )
            }
        };

        // Determine which flow to use based on request body
        let flow_to_use = if let Some(body) = &request.body {
            // Verify flow
            if let Err(err) = verify_flow(flow.clone(), body.clone()) {
                return HttpResponse::bad_request(err.to_string(), HashMap::new());
            }

            // Resolve flow
            let mut resolvable_flow = flow.clone();
            match resolve_flow(&mut resolvable_flow, body.clone()) {
                Ok(resolved_flow) => resolved_flow,
                Err(_) => {
                    return HttpResponse::internal_server_error(
                        "Internal Server Error".to_string(),
                        HashMap::new(),
                    )
                }
            }
        } else {
            // Use original flow if no body
            flow
        };

        // Serialize flow
        let json_flow = match serde_json::to_string(&flow_to_use) {
            Ok(string) => string,
            Err(err) => {
                return HttpResponse::internal_server_error(err.to_string(), HashMap::new())
            }
        };

        // Create and serialize message
        let message = create_rest_message(json_flow);
        let message_json = match serde_json::to_string(&message) {
            Ok(string) => string,
            Err(err) => {
                return HttpResponse::internal_server_error(err.to_string(), HashMap::new())
            }
        };

        // Send message to RabbitMQ
        match rabbitmq_client
            .send_message(message_json.clone(), "send_queue")
            .await
        {
            Ok(_) => println!("Message sent to RabbitMQ queue {}", message_json),
            Err(e) => println!("Failed to send message to RabbitMQ: {:?}", e),
        };

        // Wait for response
        match rabbitmq_client
            .await_message(
                "recieve_queue",
                message.message_id,
                Duration::from_secs(10),
                true,
            )
            .await
        {
            Ok(response) => HttpResponse::ok(response.body.as_bytes().to_vec(), HashMap::new()),
            Err(_) => HttpResponse::internal_server_error(
                "Failed to receive message from RabbitMQ".to_string(),
                HashMap::new(),
            ),
        }
    }
}
