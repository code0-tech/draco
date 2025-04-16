pub mod store {
    use crate::http::request::HttpRequest;
    use code0_flow::flow_store::connection::FlowStore;
    use redis::{AsyncCommands, JsonAsyncCommands};
    use std::collections::HashMap;
    use tucana::shared::{FlowSetting, FlowSettingDefinition, Value};

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

    pub async fn check_flow_exists(
        flow_store: &FlowStore,
        request: &HttpRequest,
    ) -> Option<String> {
        let settings = create_flow_settings(&request);

        // Convert settings to JSON
        let settings_json = match serde_json::to_string(&settings) {
            Ok(json) => json,
            Err(_) => return None,
        };

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

        // Check if any stored flow matches our settings
        for item in result {
            if item.contains(&settings_json) {
                return Some(item);
            }
        }

        None
    }
}
