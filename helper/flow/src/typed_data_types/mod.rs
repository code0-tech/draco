use tucana::shared::{
    data_type_rule::Config, value::Kind, DataType, DataTypeContainsKeyRuleConfig,
    DataTypeItemOfCollectionRuleConfig, DataTypeRegexRuleConfig, DataTypeRule, Translation, Value,
};

pub fn get_http_url_data_type() -> DataType {
    DataType {
        variant: 2,
        identifier: String::from("HTTP_URL"),
        name: vec![Translation {
            code: String::from("en-US"),
            content: String::from("Http Url Route"),
        }],
        rules: vec![DataTypeRule {
            config: Some(Config::Regex(DataTypeRegexRuleConfig {
                pattern: String::from(r"/^\/\w+(?:[.:~-]\w+)*(?:\/\w+(?:[.:~-]\w+)*)*$/"),
            })),
        }],
        parent_type_identifier: Some(String::from("STRING")),
        input_types: vec![],
        return_type: None,
    }
}

pub fn get_http_method_data_type() -> DataType {
    fn get_string_value(value: &str) -> Value {
        Value {
            kind: Some(Kind::StringValue(String::from(value))),
        }
    }

    DataType {
        variant: 2,
        identifier: String::from("HTTP_METHOD"),
        name: vec![Translation {
            code: String::from("en-US"),
            content: String::from("Http Method"),
        }],
        rules: vec![DataTypeRule {
            config: Some(Config::ItemOfCollection(
                DataTypeItemOfCollectionRuleConfig {
                    items: vec![
                        get_string_value("GET"),
                        get_string_value("POST"),
                        get_string_value("PUT"),
                        get_string_value("DELETE"),
                        get_string_value("PATCH"),
                    ],
                },
            )),
        }],
        parent_type_identifier: Some(String::from("ARRAY")),
        input_types: vec![],
        return_type: None,
    }
}

pub fn get_http_request_data_type() -> DataType {
    DataType {
        variant: 3,
        identifier: String::from("HTTP_REQUEST"),
        name: vec![Translation {
            code: String::from("en-US"),
            content: String::from("Http Request"),
        }],
        return_type: None,
        input_types: vec![],
        rules: vec![
            DataTypeRule {
                config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                    key: String::from("method"),
                    data_type_identifier: String::from("HTTP_METHOD"),
                })),
            },
            DataTypeRule {
                config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                    key: String::from("url"),
                    data_type_identifier: String::from("HTTP_URL"),
                })),
            },
            DataTypeRule {
                config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                    key: String::from("body"),
                    data_type_identifier: String::from("OBJECT"),
                })),
            },
            DataTypeRule {
                config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                    key: String::from("header"),
                    data_type_identifier: String::from("OBJECT"),
                })),
            },
        ],
        parent_type_identifier: Some(String::from("OBJECT")),
    }
}

pub fn get_http_response_data_type() -> DataType {
    DataType {
        variant: 3,
        identifier: String::from("HTTP_RESPONSE"),
        name: vec![Translation {
            code: String::from("en-US"),
            content: String::from("Http Response"),
        }],
        return_type: None,
        input_types: vec![],
        rules: vec![
            DataTypeRule {
                config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                    key: String::from("body"),
                    data_type_identifier: String::from("OBJECT"),
                })),
            },
            DataTypeRule {
                config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                    key: String::from("header"),
                    data_type_identifier: String::from("OBJECT"),
                })),
            },
        ],
        parent_type_identifier: Some(String::from("OBJECT")),
    }
}
