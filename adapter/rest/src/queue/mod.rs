pub mod queue {

    use code0_flow::{
        flow_queue::service::{Message, RabbitmqClient},
        flow_store::service::FlowStoreService,
    };
    use http::{request::HttpRequest, response::HttpResponse};
    use std::{collections::HashMap, sync::Arc, time::Duration};
    use tokio::sync::Mutex;
    use tucana::shared::{Struct, Value};
    use validator::verify_flow;

    use crate::store::store::check_flow_exists;

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
        mut request: HttpRequest,
        flow_store: Arc<Mutex<FlowStoreService>>,
        rabbitmq_client: Arc<RabbitmqClient>,
    ) -> Option<HttpResponse> {
        // Check if a flow exists for the given settings, return none if not exsist for http handler
        let flow_exists = check_flow_exists(flow_store, &request).await;

        let flow_result = match flow_exists {
            Some(flow) => flow,
            None => return None,
        };

        let flow = flow_result.flow;
        let regex_pattern = flow_result.regex_pattern;
        let mut url_params: HashMap<String, Value> = HashMap::new();

        //Resolve url params
        let capture_keys = regex_pattern.capture_names();
        if let Some(captures) = regex_pattern.captures(&request.path) {
            for key_option in capture_keys {
                let key = match key_option {
                    Some(key) => key,
                    None => continue,
                };

                let value = match captures.name(key) {
                    Some(value) => value.as_str().to_string(),
                    None => continue,
                };

                let string_value = Value {
                    kind: Some(tucana::shared::value::Kind::StringValue(value)),
                };

                url_params.insert(key.to_string(), string_value);
            }
        };

        //Will add the url params to the request body
        if !url_params.is_empty() {
            if let Some(body) = &mut request.body {
                if let Some(kind) = &mut body.kind {
                    match kind {
                        tucana::shared::value::Kind::StructValue(struct_value) => {
                            struct_value.fields.insert(
                                "url".to_string(),
                                Value {
                                    kind: Some(tucana::shared::value::Kind::StructValue(Struct {
                                        fields: url_params,
                                    })),
                                },
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(body) = &request.body {
            // Verify the rules of a Flow - this is only possible if a body exists
            if let Err(err) = verify_flow(flow.clone(), body.clone()) {
                return Some(HttpResponse::bad_request(err.to_string(), HashMap::new()));
            }
        }

        // Serialize flow
        let json_flow = match serde_json::to_string(&flow) {
            Ok(string) => string,
            Err(err) => {
                return Some(HttpResponse::internal_server_error(
                    err.to_string(),
                    HashMap::new(),
                ))
            }
        };

        // Create and serialize message
        let message = create_rest_message(json_flow);
        let message_json = match serde_json::to_string(&message) {
            Ok(string) => string,
            Err(err) => {
                return Some(HttpResponse::internal_server_error(
                    err.to_string(),
                    HashMap::new(),
                ))
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
            Ok(response) => Some(HttpResponse::ok(
                response.body.as_bytes().to_vec(),
                HashMap::new(),
            )),
            Err(_) => Some(HttpResponse::internal_server_error(
                "Failed to receive message from RabbitMQ".to_string(),
                HashMap::new(),
            )),
        }
    }
}
