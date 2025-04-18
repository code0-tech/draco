pub mod http;
pub mod queue;
pub mod store;

use draco_base::FromEnv;
use http::server;
use tucana::shared::value::Kind;

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
