use crate::traits::IdentifiableFlow;
use async_nats::jetstream::kv::Config;
use futures_lite::StreamExt;
use prost::Message;
use tucana::shared::{ExecutionFlow, ValidationFlow, Value};
use validator::verify_flow;

pub struct AdapterStore {
    client: async_nats::Client,
    kv: async_nats::jetstream::kv::Store,
}

pub enum FlowIdenfiyResult {
    None,
    Single(ValidationFlow),
    Multiple(Vec<ValidationFlow>),
}

impl AdapterStore {
    pub async fn from_url(url: String, bucket: String) -> Self {
        let client = match async_nats::connect(url).await {
            Ok(client) => client,
            Err(err) => panic!("Failed to connect to NATS server: {}", err),
        };

        let jetstream = async_nats::jetstream::new(client.clone());

        let _ = jetstream
            .create_key_value(Config {
                bucket: bucket.clone(),
                ..Default::default()
            })
            .await;

        let kv = match jetstream.get_key_value(bucket).await {
            Ok(kv) => kv,
            Err(err) => panic!("Failed to get key-value store: {}", err),
        };

        Self { client, kv }
    }

    /// get_possible_flow_matches
    ///
    /// This function will take a key that one or more keys of a flow.
    /// It will then loop over every value received from the key and return all flows that matched the IdentifiedFlow trait.
    ///
    /// Arguments:
    /// - pattern: The key to get possible flow matches. For example, a REST Flow is never completely identifiable through a single key because the URL is dynamic and wherefore a regex is needed to be applied to the url making it impossible to include the entire URL in the key. In this case the key just reduces the amount of flows that can be a possible match.
    /// - id: The identifier to use for identifying the possible matches. Its just a fine grain identifier that can be used to identify the possible matches. For a REST Flow this will be the regex matcher, for a CRON Flow the trait just return true every time.
    ///
    /// Returns:
    /// - FlowIdenfiyResult: The result of the flow identification process. This can be one of the following:
    ///     - None: No flows matched the identifier.
    ///     - Single(ValidationFlow): A single flow matched the identifier.
    ///     - Multiple(Vec<ValidationFlow>): Multiple flows matched the identifier.
    ///
    /// None is always bad, but as always this depends on the Adapter type.
    /// For example:
    /// REST will have only one match, if multiple matches are found it means the regex is not correct.
    /// CRON can have multiple matches, because multiple flows can have the same CRON expression.
    pub async fn get_possible_flow_match<I: IdentifiableFlow>(
        &self,
        pattern: String,
        id: I,
    ) -> FlowIdenfiyResult {
        let mut collector = Vec::new();
        let mut keys = match self.kv.keys().await {
            Ok(keys) => keys.boxed(),
            Err(err) => {
                eprintln!("Failed to get keys: {}", err);
                return FlowIdenfiyResult::None;
            }
        };

        while let Ok(Some(key)) = keys.try_next().await {
            println!("comparing: key: {} pattern {:?}", key, pattern);

            if !Self::is_matching_key(&pattern, &key) {
                println!("Key does not match pattern: {}", key);
                continue;
            }

            if let Ok(Some(bytes)) = self.kv.get(key).await {
                let decoded_flow = ValidationFlow::decode(bytes);
                if let Ok(flow) = decoded_flow {
                    if id.identify(&flow) {
                        collector.push(flow);
                    }
                };
            }
        }

        match collector.len() {
            0 => FlowIdenfiyResult::None,
            1 => FlowIdenfiyResult::Single(collector[0].clone()),
            _ => FlowIdenfiyResult::Multiple(collector),
        }
    }

    /// validate_and_execute_flow
    ///
    /// This function will validate the flow. If the flow is valid, it will execute (send the flow to the execution and wait for a/multiple result/s) the flow.
    ///
    /// Arguments:
    /// - flow: The flow to be validated and executed.
    /// - input_value: The input value to be used for the flow execution.
    pub async fn validate_and_execute_flow(
        &self,
        flow: ValidationFlow,
        input_value: Option<Value>,
    ) -> Option<Value> {
        if let Some(body) = input_value.clone() {
            let verify_result = verify_flow(flow.clone(), body);

            match verify_result {
                Ok(()) => {}
                Err(_err) => {
                    return None;
                }
            };
        }

        let uuid = uuid::Uuid::new_v4().to_string();
        let execution_flow: ExecutionFlow = Self::convert_validation_flow(flow, input_value);
        let bytes = execution_flow.encode_to_vec();
        let topic = format!("execution.{}", uuid);
        let result = self.client.request(topic, bytes.into()).await;

        match result {
            Ok(message) => match Value::decode(message.payload) {
                Ok(value) => {
                    println!("Response: {:?}", &value);
                    Some(value)
                }
                Err(err) => {
                    eprintln!("Failed to decode response from NATS server: {}", err);
                    return None;
                }
            },
            Err(err) => {
                eprintln!("Failed to send request to NATS server: {}", err);
                None
            }
        }
    }

    fn convert_validation_flow(flow: ValidationFlow, input_value: Option<Value>) -> ExecutionFlow {
        ExecutionFlow {
            flow_id: flow.flow_id,
            starting_node: flow.starting_node,
            input_value: input_value,
        }
    }

    fn is_matching_key(pattern: &String, key: &String) -> bool {
        let splitted_pattern = pattern.split(".");
        let splitted_key = key.split(".").collect::<Vec<&str>>();

        let zip = splitted_pattern.into_iter().zip(splitted_key);

        for (pattern_part, key_part) in zip {
            if pattern_part == "*" {
                continue;
            }

            if pattern_part != key_part {
                println!("matching: pattern: {} key: {}", pattern_part, key_part);
                return false;
            }
        }
        println!("pattern was correct");
        true
    }
}
