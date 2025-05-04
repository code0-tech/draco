use std::collections::HashMap;

use tucana::shared::{
    node_parameter::Value, Flow, FlowSetting, FlowSettingDefinition, NodeFunction,
    NodeFunctionDefinition, NodeParameter, NodeParameterDefinition, Struct,
};

use crate::typed_data_types::{
    get_http_method_data_type, get_http_request_data_type, get_http_response_data_type,
    get_http_url_data_type,
};

pub fn get_add_rest_flow() -> Flow {
    fn get_string_value(value: &str) -> tucana::shared::Value {
        tucana::shared::Value {
            kind: Some(tucana::shared::value::Kind::StringValue(String::from(
                value,
            ))),
        }
    }

    Flow {
        flow_id: 1,
        project_id: 1,
        r#type: "REST".to_string(),
        data_types: vec![
            get_http_url_data_type(),
            get_http_method_data_type(),
            get_http_request_data_type(),
            get_http_response_data_type(),
        ],
        input_type_identifier: Some(String::from("HTTP_REQUEST")),
        return_type_identifier: Some(String::from("HTTP_RESPONSE")),
        settings: vec![
            FlowSetting {
                definition: Some(FlowSettingDefinition {
                    id: String::from("1424525"),
                    key: String::from("HTTP_URL"),
                }),
                object: Some(Struct {
                    fields: {
                        let mut map = HashMap::new();
                        map.insert(String::from("url"), get_string_value("/add"));
                        map
                    },
                }),
            },
            FlowSetting {
                definition: Some(FlowSettingDefinition {
                    id: String::from("14245252352"),
                    key: String::from("HTTP_METHOD"),
                }),
                object: Some(Struct {
                    fields: {
                        let mut map = HashMap::new();
                        map.insert(String::from("method"), get_string_value("GET"));
                        map
                    },
                }),
            },
        ],
        starting_node: Some(NodeFunction {
            definition: Some(NodeFunctionDefinition {
                function_id: String::from("234567890"),
                runtime_function_id: String::from("std::math::add"),
            }),
            parameters: vec![
                NodeParameter {
                    definition: Some(NodeParameterDefinition {
                        parameter_id: String::from("12345678"),
                        runtime_parameter_id: String::from("first"),
                    }),
                    value: Some(Value::LiteralValue(get_string_value("body.first"))),
                },
                NodeParameter {
                    definition: Some(NodeParameterDefinition {
                        parameter_id: String::from("25346346"),
                        runtime_parameter_id: String::from("second"),
                    }),
                    value: Some(Value::LiteralValue(get_string_value("body.second"))),
                },
            ],
            next_node: None,
        }),
    }
}
