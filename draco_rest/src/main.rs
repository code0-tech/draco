pub mod http;

use code0_flow::flow_store::connection::{create_flow_store_connection, FlowStore};
use draco_base::FromEnv;
use draco_body::verify_flow;
use futures_lite::StreamExt;
use http::http::{HttpOption, HttpRequest};
use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;
use lapin::{Channel, Connection};
use redis::{AsyncCommands, JsonAsyncCommands};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tucana::shared::value::Kind;
use tucana::shared::{
    DataType, DataTypeRule, Flow, Flows, NodeFunctionDefinition, NodeParameter,
    NodeParameterDefinition, Value,
};
use tucana::shared::{FlowSetting, FlowSettingDefinition};

// Custom error that wraps lapin::Error or a default Rust error
#[derive(Debug)]
enum RabbitMqError {
    LapinError(lapin::Error),
    ConnectionError(String),
    TimeoutError,
    DeserializationError,
}

impl From<lapin::Error> for RabbitMqError {
    fn from(error: lapin::Error) -> Self {
        RabbitMqError::LapinError(error)
    }
}

impl From<std::io::Error> for RabbitMqError {
    fn from(error: std::io::Error) -> Self {
        RabbitMqError::ConnectionError(error.to_string())
    }
}

impl std::fmt::Display for RabbitMqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RabbitMqError::LapinError(err) => write!(f, "RabbitMQ error: {}", err),
            RabbitMqError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            RabbitMqError::TimeoutError => write!(f, "Operation timed out"),
            RabbitMqError::DeserializationError => write!(f, "Failed to deserialize message"),
        }
    }
}

#[derive(Serialize, Deserialize)]
enum MessageType {
    ExecuteFlow,
    TestExecuteFlow,
}

#[derive(Serialize, Deserialize)]
struct Sender {
    name: String,
    protocol: String,
    version: String,
}

#[derive(Serialize, Deserialize)]
struct Message {
    message_type: MessageType,
    sender: Sender,
    timestamp: i64,
    telegram_id: String,
    body: String,
}

// Configuration struct loaded from environment file
#[derive(FromEnv)]
struct Config {
    port: u16,
    redis_url: String,
    rabbitmq_url: String,
}

async fn build_connection(rabbitmq_url: &str) -> Connection {
    match Connection::connect(rabbitmq_url, lapin::ConnectionProperties::default()).await {
        Ok(env) => env,
        Err(error) => panic!(
            "Cannot connect to FlowQueue (RabbitMQ) instance! Reason: {:?}",
            error
        ),
    }
}

// Thread-safe wrapper for RabbitMQ channel
struct RabbitmqClient {
    channel: Arc<Mutex<Channel>>,
}

impl RabbitmqClient {
    // Create a new RabbitMQ client with channel
    async fn new(rabbitmq_url: &str) -> Self {
        let connection = build_connection(rabbitmq_url).await;
        let channel = connection.create_channel().await.unwrap();

        // Declare the queue once during initialization
        channel
            .queue_declare(
                "send_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        channel
            .queue_declare(
                "recieve_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        RabbitmqClient {
            channel: Arc::new(Mutex::new(channel)),
        }
    }

    // Send message to the queue
    async fn send_message(
        &self,
        message_json: String,
        queue_name: &str,
    ) -> Result<(), lapin::Error> {
        let channel = self.channel.lock().await;

        channel
            .basic_publish(
                "",         // exchange
                queue_name, // routing key (queue name)
                lapin::options::BasicPublishOptions::default(),
                message_json.as_bytes(),
                lapin::BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    // Receive messages from a queue
    async fn receive_messages(
        &self,
        queue_name: &str,
        telegram_id: String,
    ) -> Result<String, RabbitMqError> {
        let mut consumer = {
            let channel = self.channel.lock().await;

            let consumer_res = channel
                .basic_consume(
                    queue_name,
                    "consumer",
                    lapin::options::BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await;

            match consumer_res {
                Ok(consumer) => consumer,
                Err(err) => panic!("{}", err),
            }
        };

        println!("Starting to consume from {}", queue_name);

        // Create a future for the next message
        let receive_future = async {
            while let Some(delivery_result) = consumer.next().await {
                let delivery = match delivery_result {
                    Ok(del) => del,
                    Err(_) => return Err(RabbitMqError::DeserializationError),
                };
                let data = &delivery.data;
                let message_str = match std::str::from_utf8(&data) {
                    Ok(str) => str,
                    Err(_) => {
                        return Err(RabbitMqError::DeserializationError);
                    }
                };
                println!("Received message: {}", message_str);

                // Parse the message
                let message = match serde_json::from_str::<Message>(message_str) {
                    Ok(m) => m,
                    Err(e) => {
                        println!("Failed to parse message: {}", e);
                        return Err(RabbitMqError::DeserializationError);
                    }
                };

                if message.telegram_id == telegram_id {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                        message.body
                    );

                    delivery
                        .ack(lapin::options::BasicAckOptions::default())
                        .await
                        .expect("Failed to acknowledge message");

                    return Ok(response);
                }
            }
            Err(RabbitMqError::DeserializationError)
        };

        // Set a timeout of 10 seconds
        match tokio::time::timeout(std::time::Duration::from_secs(10), receive_future).await {
            Ok(result) => result,
            Err(_) => {
                println!("Timeout waiting for message after 10 seconds");
                Err(RabbitMqError::TimeoutError)
            }
        }
    }
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
        match verify_flow(flow.clone(), http_request.body.unwrap()) {
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

        let message = Message {
            message_type: MessageType::ExecuteFlow,
            sender: Sender {
                name: "draco_rest".to_string(),
                protocol: "rest".to_string(),
                version: "1.0".to_string(),
            },
            telegram_id: "test".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            body: format!("{:?}", flow),
        };

        let message_json = serde_json::to_string(&message).unwrap();

        // Send the message to RabbitMQ queue using thread-safe client

        match rabbitmq_client
            .send_message(message_json, "send_queue")
            .await
        {
            Ok(_) => println!("Message sent to RabbitMQ queue"),
            Err(e) => println!("Failed to send message to RabbitMQ: {:?}", e),
        };

        match rabbitmq_client
            .receive_messages("recieve_queue", "test".to_string())
            .await
        {
            Ok(response) => response,
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
