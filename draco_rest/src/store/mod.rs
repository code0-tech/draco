pub mod store {
    use crate::http::request::HttpRequest;
    use code0_flow::flow_store::connection::FlowStore;
    use redis::{AsyncCommands, JsonAsyncCommands};
    use regex::Regex;
    use tucana::shared::{value::Kind, Flow, Struct};

    //The regex is required for later purposes --> resolve the parameter of the url
    pub struct FlowExistResult {
        pub flow: Flow,
        pub regex_pattern: Regex,
    }

    pub async fn check_flow_exists(
        flow_store: &FlowStore,
        request: &HttpRequest,
    ) -> Option<FlowExistResult> {
        let mut store = flow_store.lock().await;

        // Get all keys from Redis
        let keys: Vec<String> = store.keys("*").await.unwrap_or_default();
        let mut result: Vec<Flow> = Vec::new();

        // Retrieve JSON values for each key
        for key in keys {
            if let Ok(json_value) = store.json_get::<&String, &str, String>(&key, "$").await {
                let flow = match serde_json::from_str::<Vec<Flow>>(json_value.as_str()) {
                    Ok(flow) => flow[0].clone(),
                    Err(_) => continue,
                };

                result.push(flow);
            }
        }

        for flow in result {
            let mut correct_url = false;
            let mut correct_method = false;
            let mut flow_regex: Option<Regex> = None;

            for setting in flow.settings.clone() {
                let definition = match setting.definition {
                    Some(definition) => definition,
                    None => continue,
                };

                if definition.key == "HTTP_METHOD" {
                    let object: Struct = match setting.object {
                        Some(object) => object,
                        None => continue,
                    };

                    for field in object.fields {
                        if field.0 == "method" {
                            if let Some(Kind::StringValue(method)) = field.1.kind {
                                if method == request.method.to_string() {
                                    correct_method = true;
                                }
                            }
                        }
                    }

                    continue;
                }

                if definition.key == "URL" {
                    let object: Struct = match setting.object {
                        Some(object) => object,
                        None => continue,
                    };

                    for field in object.fields {
                        if field.0 == "url" {
                            if let Some(Kind::StringValue(regex_str)) = field.1.kind {
                                let regex = match regex::Regex::new(&regex_str) {
                                    Ok(regex) => regex,
                                    Err(err) => {
                                        log::error!("Failed to compile regex: {}", err);
                                        continue;
                                    }
                                };

                                if regex.is_match(&request.path) {
                                    correct_url = true;
                                    flow_regex = Some(regex);
                                }
                            }
                        }
                    }

                    continue;
                }
            }

            if correct_method && correct_url {
                let regex_pattern = match flow_regex {
                    Some(regex) => regex.clone(),
                    None => continue,
                };

                return Some(FlowExistResult {
                    flow,
                    regex_pattern,
                });
            }
        }

        None
    }
}
