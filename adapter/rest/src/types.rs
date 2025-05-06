use tucana::shared::{
    data_type_rule::Config, value::Kind, DataType, DataTypeContainsKeyRuleConfig,
    DataTypeContainsTypeRuleConfig, DataTypeRegexRuleConfig, DataTypeRule, FlowType, Translation,
    Value,
};

pub fn get_flow_types() -> Vec<FlowType> {
    vec![FlowType {
        identifier: String::from("REST"),
        settings: vec![],
        input_type_identifier: Some(String::from("HTTP_REQUEST_OBJECT")),
        return_type_identifier: Some(String::from("HTTP_RESPONSE_OBJECT")),
        editable: true,
        name: vec![Translation {
            code: String::from("en-US"),
            content: String::from("Rest Endpoint"),
        }],
        description: vec![Translation {
            code: String::from("en-US"),
            content: String::from("A REST API is a web service that lets clients interact with data on a server using standard HTTP methods like GET, POST, PUT, and DELETE, usually returning results in JSON format."),
        }],
    }]
}

pub fn get_data_types() -> Vec<DataType> {
    vec![
        DataType {
            variant: 2,
            name: vec![Translation {
                code: String::from("en-US"),
                content: String::from("HTTP Method"),
            }],
            identifier: String::from("HTTP_METHOD"),
            input_types: vec![],
            return_type: None,
            parent_type_identifier: None,
            rules: vec![DataTypeRule {
                config: Some(Config::ItemOfCollection(
                    tucana::shared::DataTypeItemOfCollectionRuleConfig {
                        items: vec![
                            Value {
                                kind: Some(Kind::StringValue(String::from("GET"))),
                            },
                            Value {
                                kind: Some(Kind::StringValue(String::from("POST"))),
                            },
                            Value {
                                kind: Some(Kind::StringValue(String::from("PUT"))),
                            },
                            Value {
                                kind: Some(Kind::StringValue(String::from("DELETE"))),
                            },
                            Value {
                                kind: Some(Kind::StringValue(String::from("PATCH"))),
                            },
                            Value {
                                kind: Some(Kind::StringValue(String::from("HEAD"))),
                            },
                        ],
                    },
                )),
            }],
        },
        DataType {
            variant: 2,
            name: vec![Translation {
                code: String::from("en-US"),
                content: String::from("HTTP Route"),
            }],
            identifier: String::from("HTTP_URL"),
            input_types: vec![],
            return_type: None,
            parent_type_identifier: None,
            rules: vec![DataTypeRule {
                config: Some(Config::Regex(DataTypeRegexRuleConfig {
                    pattern: String::from(r"/^\/\w+(?:[.:~-]\w+)*(?:\/\w+(?:[.:~-]\w+)*)*$/"),
                })),
            }],
        },
        DataType {
            variant: 5,
            name: vec![Translation {
                code: String::from("en-US"),
                content: String::from("HTTP Headers"),
            }],
            identifier: String::from("HTTP_HEADER_MAP"),
            input_types: vec![],
            return_type: None,
            parent_type_identifier: Some(String::from("ARRAY")),
            rules: vec![DataTypeRule {
                config: Some(Config::ContainsType(DataTypeContainsTypeRuleConfig {
                    data_type_identifier: String::from("HTTP_HEADER_ENTRY"),
                })),
            }],
        },
        DataType {
            variant: 3,
            name: vec![Translation {
                code: String::from("en-US"),
                content: String::from("HTTP Header Entry"),
            }],
            identifier: String::from("HTTP_HEADER_ENTRY"),
            input_types: vec![],
            return_type: None,
            parent_type_identifier: Some(String::from("OBJECT")),
            rules: vec![
                DataTypeRule {
                    config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                        key: String::from("key"),
                        data_type_identifier: String::from("TEXT"),
                    })),
                },
                DataTypeRule {
                    config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                        key: String::from("value"),
                        data_type_identifier: String::from("TEXT"),
                    })),
                },
            ],
        },
        DataType {
            variant: 3,
            name: vec![Translation {
                code: String::from("en-US"),
                content: String::from("HTTP Request"),
            }],
            identifier: String::from("HTTP_REQUEST_OBJECT"),
            input_types: vec![],
            return_type: None,
            parent_type_identifier: Some(String::from("OBJECT")),
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
                        key: String::from("headers"),
                        data_type_identifier: String::from("HTTP_HEADER_MAP"),
                    })),
                },
            ],
        },
        DataType {
            variant: 3,
            name: vec![Translation {
                code: String::from("en-US"),
                content: String::from("HTTP Response"),
            }],
            identifier: String::from("HTTP_RESPONSE_OBJECT"),
            input_types: vec![],
            return_type: None,
            parent_type_identifier: Some(String::from("OBJECT")),
            rules: vec![
                DataTypeRule {
                    config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                        key: String::from("body"),
                        data_type_identifier: String::from("OBJECT"),
                    })),
                },
                DataTypeRule {
                    config: Some(Config::ContainsKey(DataTypeContainsKeyRuleConfig {
                        key: String::from("headers"),
                        data_type_identifier: String::from("HTTP_HEADER_MAP"),
                    })),
                },
            ],
        },
    ]
}
