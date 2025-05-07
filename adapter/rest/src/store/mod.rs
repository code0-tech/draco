pub mod store {
    use std::sync::Arc;

    use code0_flow::flow_store::service::{FlowStoreService, FlowStoreServiceBase};
    use http::request::HttpRequest;
    use regex::Regex;
    use tokio::sync::Mutex;
    use tucana::shared::{value::Kind, Flow, FlowSetting};

    //The regex is required for later purposes --> resolve the parameter of the url
    pub struct FlowExistResult {
        pub flow: Flow,
        pub regex_pattern: Regex,
    }

    fn extract_field(settings: &[FlowSetting], def_key: &str, field_name: &str) -> Option<String> {
        settings.iter().find_map(|setting| {
            let def = setting.definition.as_ref()?;
            if def.key != def_key {
                return None;
            }

            let obj = setting.object.as_ref()?;
            obj.fields.iter().find_map(|(k, v)| {
                if k == field_name {
                    if let Some(Kind::StringValue(s)) = &v.kind {
                        return Some(s.clone());
                    }
                }
                None
            })
        })
    }

    pub async fn check_flow_exists(
        flow_store: Arc<Mutex<FlowStoreService>>,
        request: &HttpRequest,
    ) -> Option<FlowExistResult> {
        let flows = {
            let mut store = flow_store.lock().await;
            let pattern = format!("*::*::{}::{}", request.host, request.method.to_string());
            let result = store.query_flows(pattern).await;

            match result {
                Ok(flows) => flows.flows,
                Err(_) => return None,
            }
        };

        for flow in flows {
            let url = extract_field(&flow.settings, "HTTP_URL", "url");

            let regex_str = match url {
                Some(string) => string,
                None => continue,
            };

            let regex = match regex::Regex::new(&regex_str) {
                Ok(regex) => regex,
                Err(err) => {
                    log::error!("Failed to compile regex: {}", err);
                    continue;
                }
            };

            if regex.is_match(&request.path) {
                return Some(FlowExistResult {
                    flow,
                    regex_pattern: regex,
                });
            }
        }
        None
    }
}
