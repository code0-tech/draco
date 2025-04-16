use tucana::shared::{value::Kind, Flow, Value};

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
                if let Some(Kind::StringValue(key)) = &mut param_value.kind {
                    if let Some(Kind::StructValue(ref struct_value)) = body.kind {
                        let body_value = struct_value.fields.get(key);

                        println!(
                            "Field: {}, will be replaced from body with the value: {:?}",
                            key, body_value
                        );

                        if let Some(body_val) = body_value {
                            *param_value = body_val.clone();
                        }
                    }
                }

                /*    if let Some(Kind::StructValue(struct_value)) = &mut param_value.kind {
                    let mut result = HashMap::new();
                    for (field, _) in struct_value.fields.clone() {
                        let body_fields = match &body.kind.unwrap() {
                            Kind::StructValue(body_struct) => body_struct,
                            _ => panic!("Expected struct value for body"),
                        };

                        println!("{}", field.clone());
                        let body_value = body_fields.fields.get(&field);
                        println!(
                            "Field: {}, will be replaced from body with the value: {:?}",
                            field, body_value
                        );

                        result.insert(field.clone(), body_value.unwrap().clone());
                    }

                    struct_value.fields = result;
                    println!("Struct value fields updated, {:?}", struct_value);
                    println!("Parameter value updated {:?}", param_value);
                } else {
                    panic!("not implemented yet")
                }*/
            }
            tucana::shared::node_parameter::Value::FunctionValue(_function) => {
                // Handle unsupported parameter type
            }
        }
    }

    Ok(flow.clone())
}
