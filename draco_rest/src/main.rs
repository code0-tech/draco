pub mod http;
pub mod queue;
pub mod store;

use draco_base::FromEnv;
use http::server;
use std::collections::HashMap;
use tucana::shared::value::Kind;
use tucana::shared::{
    DataType, DataTypeRule, Flow, NodeFunctionDefinition, NodeParameter, NodeParameterDefinition,
    Value,
};
use tucana::shared::{FlowSetting, FlowSettingDefinition};

#[derive(FromEnv)]
pub struct Config {
    port: u16,
    redis_url: String,
    rabbitmq_url: String,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting Draco REST server");

    let config = Config::from_file("./.env");
    let server = server::Server::new(config);

    server.start().await
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
                        kind: Some(tucana::shared::value::Kind::StringValue(
                            "first".to_string(),
                        )),
                    })),
                },
                NodeParameter {
                    definition: Some(NodeParameterDefinition {
                        parameter_id: "some_database_id".to_string(),
                        runtime_parameter_id: "standard::keys::math::add::second".to_string(),
                    }),
                    value: Some(tucana::shared::node_parameter::Value::LiteralValue(Value {
                        kind: Some(tucana::shared::value::Kind::StringValue(
                            "second".to_string(),
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

fn to_tucana_value(value: serde_json::Value) -> tucana::shared::Value {
    match value {
        serde_json::Value::Null => tucana::shared::Value {
            kind: Some(Kind::NullValue(0)),
        },
        serde_json::Value::Bool(b) => tucana::shared::Value {
            kind: Some(Kind::BoolValue(b)),
        },
        serde_json::Value::Number(n) => tucana::shared::Value {
            kind: Some(Kind::NumberValue(n.as_f64().unwrap())),
        },
        serde_json::Value::String(s) => tucana::shared::Value {
            kind: Some(Kind::StringValue(s)),
        },
        serde_json::Value::Array(arr) => tucana::shared::Value {
            kind: Some(Kind::ListValue(tucana::shared::ListValue {
                values: arr.into_iter().map(|v| to_tucana_value(v)).collect(),
            })),
        },
        serde_json::Value::Object(obj) => tucana::shared::Value {
            kind: Some(Kind::StructValue(tucana::shared::Struct {
                fields: obj
                    .into_iter()
                    .map(|(k, v)| (k, to_tucana_value(v)))
                    .collect(),
            })),
        },
    }
}
