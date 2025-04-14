use std::collections::HashMap;

use serde_json::Value;
use tucana::shared::{value::Kind, Flow};

pub fn resolve_flow(flow: &mut Flow, body: Value) -> Result<Flow, ()> {
    let node = match &mut flow.starting_node {
        Some(node) => node,
        None => return Ok(flow.clone()),
    };

    for parameter in &mut node.parameters {
        let value = match &mut parameter.value {
            Some(value) => value,
            None => continue,
        };

        match value {
            tucana::shared::node_parameter::Value::LiteralValue(param_value) => {
                if let Some(Kind::StructValue(struct_value)) = &mut param_value.kind {
                    let mut result = HashMap::new();
                    for (field, _) in struct_value.fields.clone() {
                        let body_fields = match &body {
                            Value::Object(body_struct) => body_struct,
                            _ => panic!("Expected struct value for body"),
                        };

                        println!("{}", field.clone());
                        let body_value = body_fields.get(&field);
                        println!(
                            "Field: {}, will be replaced from body with the value: {:?}",
                            field, body_value
                        );

                        result.insert(field.clone(), to_tucana_value(body_value.unwrap().clone()));
                    }

                    struct_value.fields = result;
                    println!("Struct value fields updated, {:?}", struct_value);
                    println!("Parameter value updated {:?}", param_value);
                } else {
                    panic!("not implemented yet")
                }
            }
            tucana::shared::node_parameter::Value::FunctionValue(_function) => {
                // Handle unsupported parameter type
            }
        }
    }

    Ok(flow.clone())
}

fn to_tucana_value(value: Value) -> tucana::shared::Value {
    match value {
        Value::Null => tucana::shared::Value {
            kind: Some(Kind::NullValue(0)),
        },
        Value::Bool(b) => tucana::shared::Value {
            kind: Some(Kind::BoolValue(b)),
        },
        Value::Number(n) => tucana::shared::Value {
            kind: Some(Kind::NumberValue(n.as_f64().unwrap())),
        },
        Value::String(s) => tucana::shared::Value {
            kind: Some(Kind::StringValue(s)),
        },
        Value::Array(arr) => tucana::shared::Value {
            kind: Some(Kind::ListValue(tucana::shared::ListValue {
                values: arr.into_iter().map(|v| to_tucana_value(v)).collect(),
            })),
        },
        Value::Object(obj) => tucana::shared::Value {
            kind: Some(Kind::StructValue(tucana::shared::Struct {
                fields: obj
                    .into_iter()
                    .map(|(k, v)| (k, to_tucana_value(v)))
                    .collect(),
            })),
        },
    }
}
