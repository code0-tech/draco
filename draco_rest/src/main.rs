pub mod http;

use code0_flow::flow_queue::service::{Message, MessageType, RabbitmqClient, Sender};
use code0_flow::flow_store::connection::{create_flow_store_connection, FlowStore};
use draco_base::FromEnv;
use draco_validator::resolver::flow_resolver::resolve_flow;
use draco_validator::verify_flow;
use http::http::{HttpOption, HttpRequest};
use redis::{AsyncCommands, JsonAsyncCommands};
use std::collections::HashMap;
use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tucana::shared::{
    DataType, DataTypeRule, Flow, NodeFunctionDefinition, NodeParameter, NodeParameterDefinition,
    Value,
};
use tucana::shared::{FlowSetting, FlowSettingDefinition};

#[derive(FromEnv)]
struct Config {
    port: u16,
    redis_url: String,
    rabbitmq_url: String,
}

async fn handle_connection(
    mut stream: TcpStream,
    flow_store: FlowStore,
    rabbitmq_client: Arc<RabbitmqClient>,
) {
    // Parse the HTTP request
    let http_request = match parse_http_stream(&stream) {
        Ok(request) => request,
        Err(response) => {
            stream.write_all(response.as_bytes()).unwrap();
            return;
        }
    };

    // Create flow settings based on HTTP request
    let settings = create_flow_settings(&http_request);

    // Convert settings to JSON
    let settings_json = match serde_json::to_string(&settings) {
        Ok(json) => json,
        Err(err) => {
            let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
            println!("JSON serialization error: {}", err);
            stream.write_all(response.as_bytes()).unwrap();
            return;
        }
    };

    // Check if a flow exists for the given settings
    let flow_exists = check_flow_exists(&flow_store, &settings_json).await;

    // Send appropriate response
    let response = if flow_exists.0 {
        let flow = match flow_exists.1 {
            Some(flow) => match serde_json::from_str::<Vec<Flow>>(flow.as_str()) {
                Ok(c) => c[0].clone(),
                Err(err) => {
                    println!("Problems parsing: {}", flow);
                    let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                    println!("Flow cant be parsed {}", err);
                    stream.write_all(response.as_bytes()).unwrap();
                    return;
                }
            },
            None => {
                let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                println!("Flow not found");
                stream.write_all(response.as_bytes()).unwrap();
                return;
            }
        };

        //TODO: Body verification of the incomming request (only json for now)
        match verify_flow(flow.clone(), http_request.body.clone().unwrap()) {
            Ok(_) => {
                print!("Body is correct")
            }
            Err(err) => {
                let reason = err.to_string();
                let json_response =
                    format!("{{\"error\":\"Invalid body\",\"reason\":\"{}\"}}", reason);
                let response = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    json_response.len(),
                    json_response);
                stream.write_all(response.as_bytes()).unwrap();
                return;
            }
        };

        // Resolve the flow by replacing parameters with actual values
        let mut flow_to_execute = flow.clone();
        // Convert http_request.body from Option<Value> to Struct
        let resolved_flow = match resolve_flow(&mut flow_to_execute, http_request.body.unwrap()) {
            Ok(flow) => flow,
            Err(_) => {
                panic!("Failed to resolve flow")
            }
        };

        let message = Message {
            message_type: MessageType::ExecuteFlow,
            sender: Sender {
                name: "draco_rest".to_string(),
                protocol: "rest".to_string(),
                version: "1.0".to_string(),
            },
            message_id: "test".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            body: format!("{:?}", resolved_flow),
        };

        let message_json = serde_json::to_string(&message).unwrap();

        // Send the message to RabbitMQ queue using thread-safe client

        match rabbitmq_client
            .send_message(message_json.clone(), "send_queue")
            .await
        {
            Ok(_) => println!("Message sent to RabbitMQ queue {}", message_json),
            Err(e) => println!("Failed to send message to RabbitMQ: {:?}", e),
        };

        match rabbitmq_client
            .await_message(
                "recieve_queue",
                "test".to_string(),
                Duration::from_secs(10),
                true,
            )
            .await
        {
            Ok(response) => {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                    response.body
                )
            }
            Err(_) => "HTTP/1.1 500 Internal Server Error\r\n\r\n".to_string(),
        }
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nFlow does not exist".to_string()
    };

    stream.write_all(response.as_bytes()).unwrap();
}

// Create flow settings from an HTTP request
fn create_flow_settings(http_request: &HttpRequest) -> Vec<FlowSetting> {
    vec![
        FlowSetting {
            definition: Some(FlowSettingDefinition {
                id: "some_database_id".to_string(),
                key: "HTTP_METHOD".to_string(),
            }),
            object: Some(tucana::shared::Struct {
                fields: HashMap::from([(
                    String::from("method"),
                    Value {
                        kind: Some(tucana::shared::value::Kind::StringValue(
                            http_request.method.to_string(),
                        )),
                    },
                )]),
            }),
        },
        FlowSetting {
            definition: Some(FlowSettingDefinition {
                id: "some_database_id".to_string(),
                key: "URL".to_string(),
            }),
            object: Some(tucana::shared::Struct {
                fields: HashMap::from([(
                    String::from("url"),
                    Value {
                        kind: Some(tucana::shared::value::Kind::StringValue(
                            http_request.path.clone(),
                        )),
                    },
                )]),
            }),
        },
    ]
}

// Check if a flow exists for the given settings JSON
async fn check_flow_exists(flow_store: &FlowStore, settings_json: &str) -> (bool, Option<String>) {
    //TODO: Use a more efficient approach to check if a flow exists
    let mut store = flow_store.lock().await;

    // Get all keys from Redis
    let keys: Vec<String> = store.keys("*").await.unwrap_or_default();
    let mut result: Vec<String> = Vec::new();

    // Retrieve JSON values for each key
    for key in keys {
        if let Ok(json_value) = store.json_get(&key, "$").await {
            result.push(json_value);
        }
    }

    println!("Number of items: {}", result.len());
    println!("Settings JSON: {}", settings_json);

    // Check if any stored flow matches our settings
    for item in result {
        println!("{}", item);

        if item.contains(settings_json) {
            return (true, Some(item));
        } else {
            println!("Item does not contain settings JSON");
            println!("Pair: {}", item);
        }
    }

    (false, None)
}
// Parse an HTTP stream into a structured request
fn parse_http_stream(stream: &TcpStream) -> Result<HttpRequest, String> {
    let mut buf_reader = BufReader::new(stream);

    // Read headers
    let mut raw_http_request: Vec<String> = Vec::new();
    let mut line = String::new();

    // Read headers until empty line
    while let Ok(bytes) = buf_reader.read_line(&mut line) {
        if bytes == 0 || line.trim().is_empty() {
            break;
        }
        raw_http_request.push(line.trim().to_string());
        line.clear();
    }

    // Parse headers
    let mut http_request = parse_request(raw_http_request);

    // Read body if Content-Length is specified
    for header in &http_request.headers {
        if header.to_lowercase().starts_with("content-length:") {
            let content_length: usize = header
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            if content_length > 0 {
                let mut body = vec![0; content_length];
                if let Ok(_) = buf_reader.read_exact(&mut body) {
                    // Parse JSON body
                    if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&body) {
                        http_request.body = Some(json_value);
                    }
                }
            }
            break;
        }
    }

    println!("Request: {:#?}", http_request);

    // Validate HTTP version
    if http_request.version != "HTTP/1.1" {
        return Err("HTTP/1.1 400 Bad Request\r\n\r\n".to_string());
    }

    Ok(http_request)
}

// Parse raw HTTP request strings into structured HttpRequest
fn parse_request(raw_http_request: Vec<String>) -> HttpRequest {
    let params = &raw_http_request[0];

    if params.is_empty() {
        // Better error handling needed
        panic!("Empty HTTP request line")
    }

    let mut header_params = params.split(" ");
    let raw_method = header_params.next().unwrap();
    let path = header_params.next().unwrap();
    let version = header_params.next().unwrap();

    let method = HttpOption::from_str(raw_method).unwrap();

    HttpRequest {
        method,
        path: path.to_string(),
        version: version.to_string(),
        headers: raw_http_request.clone(),
        body: None,
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting Draco REST server");

    let config = Config::from_file("./.env");
    let url = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(url).unwrap();

    let flow_store = create_flow_store_connection(config.redis_url).await;
    let rabbitmq_client = Arc::new(RabbitmqClient::new(config.rabbitmq_url.as_str()).await);

    // Listen for incoming TCP connections
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let flow_store_clone = flow_store.clone();
        let rabbitmq_client_clone = rabbitmq_client.clone();

        tokio::spawn(async move {
            handle_connection(stream, flow_store_clone, rabbitmq_client_clone).await;
        });
    }
}

fn mock_flow() {
    let flow = Flow {
        flow_id: 6,
        r#type: "REST".to_string(),
        data_types: vec![DataType {
            variant: 1,
            identifier: "1".to_string(),
            name: vec![],
            rules: vec![DataTypeRule {
                variant: 1,
                config: Some(tucana::shared::Struct {
                    fields: HashMap::from([(
                        "pattern".to_string(),
                        Value {
                            kind: Some(tucana::shared::value::Kind::StringValue(
                                "^[0-9]".to_string(),
                            )),
                        },
                    )]),
                }),
            }],
            input_types: vec![],
            parent_type_identifier: None,
            return_type: None,
        }],
        input_type: Some(DataType {
            variant: 3,
            identifier: "2".to_string(),
            name: vec![],
            rules: vec![
                DataTypeRule {
                    variant: 5,
                    config: Some(tucana::shared::Struct {
                        fields: HashMap::from([
                            (
                                "key".to_string(),
                                Value {
                                    kind: Some(tucana::shared::value::Kind::StringValue(
                                        "first".to_string(),
                                    )),
                                },
                            ),
                            (
                                "type".to_string(),
                                Value {
                                    kind: Some(tucana::shared::value::Kind::StringValue(
                                        "1".to_string(),
                                    )),
                                },
                            ),
                        ]),
                    }),
                },
                DataTypeRule {
                    variant: 5,
                    config: Some(tucana::shared::Struct {
                        fields: HashMap::from([
                            (
                                "key".to_string(),
                                Value {
                                    kind: Some(tucana::shared::value::Kind::StringValue(
                                        "second".to_string(),
                                    )),
                                },
                            ),
                            (
                                "type".to_string(),
                                Value {
                                    kind: Some(tucana::shared::value::Kind::StringValue(
                                        "1".to_string(),
                                    )),
                                },
                            ),
                        ]),
                    }),
                },
            ],
            input_types: vec![],
            parent_type_identifier: None,
            return_type: None,
        }),
        settings: vec![
            FlowSetting {
                definition: Some(FlowSettingDefinition {
                    id: "some_database_id".to_string(),
                    key: "HTTP_METHOD".to_string(),
                }),
                object: Some(tucana::shared::Struct {
                    fields: HashMap::from([(
                        String::from("method"),
                        Value {
                            kind: Some(tucana::shared::value::Kind::StringValue(
                                "POST".to_string(),
                            )),
                        },
                    )]),
                }),
            },
            FlowSetting {
                definition: Some(FlowSettingDefinition {
                    id: "some_database_id".to_string(),
                    key: "URL".to_string(),
                }),
                object: Some(tucana::shared::Struct {
                    fields: HashMap::from([(
                        String::from("url"),
                        Value {
                            kind: Some(tucana::shared::value::Kind::StringValue(
                                "/add".to_string(),
                            )),
                        },
                    )]),
                }),
            },
        ],
        starting_node: Some(tucana::shared::NodeFunction {
            definition: Some(NodeFunctionDefinition {
                function_id: "some_database_id".to_string(),
                runtime_function_id: "standard::function::math::add".to_string(),
            }),
            parameters: vec![
                NodeParameter {
                    definition: Some(NodeParameterDefinition {
                        parameter_id: "some_database_id".to_string(),
                        runtime_parameter_id: "standard::keys::math::add::first".to_string(),
                    }),
                    value: Some(tucana::shared::node_parameter::Value::LiteralValue(Value {
                        kind: Some(tucana::shared::value::Kind::StructValue(
                            tucana::shared::Struct {
                                fields: HashMap::from([(
                                    String::from("first"),
                                    Value {
                                        kind: Some(tucana::shared::value::Kind::StringValue(
                                            "$first$".to_string(),
                                        )),
                                    },
                                )]),
                            },
                        )),
                    })),
                },
                NodeParameter {
                    definition: Some(NodeParameterDefinition {
                        parameter_id: "some_database_id".to_string(),
                        runtime_parameter_id: "standard::keys::math::add::second".to_string(),
                    }),
                    value: Some(tucana::shared::node_parameter::Value::LiteralValue(Value {
                        kind: Some(tucana::shared::value::Kind::StructValue(
                            tucana::shared::Struct {
                                fields: HashMap::from([(
                                    String::from("second"),
                                    Value {
                                        kind: Some(tucana::shared::value::Kind::StringValue(
                                            "$second$".to_string(),
                                        )),
                                    },
                                )]),
                            },
                        )),
                    })),
                },
            ],
            next_node: None,
        }),
    };

    let json = serde_json::to_string(&flow).unwrap();
    println!("{}", json);
}
