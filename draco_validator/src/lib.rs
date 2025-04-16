pub mod resolver;
pub mod rules;

use rules::{
    contains_key::apply_contains_key,
    regex::apply_regex,
    violation::{DataTypeRuleError, DataTypeRuleViolation},
};
use serde::{Deserialize, Serialize};
use std::i32;
use tucana::shared::{data_type_rule::Variant, DataType, Flow, Struct, Value};

pub struct VerificationResult;

#[derive(Serialize, Deserialize)]
pub struct RegexRule {
    pub pattern: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainsRule {
    pub key: String,
    pub r#type: String,
}

pub fn verify_flow(flow: Flow, body: Value) -> Result<(), DataTypeRuleError> {
    println!("Root body: {:?}", body);

    let input_type = match &flow.input_type {
        Some(r) => r.clone(),
        None => return Ok(()), //Returns directly because no rule is given. The body is ok and will not be concidered
    };

    verify_body(flow, body, input_type)
}

pub fn convert_to_variant(number: i32) -> Variant {
    match number {
        x if x == Variant::Unknown as i32 => Variant::Unknown,
        x if x == Variant::Regex as i32 => Variant::Regex,
        x if x == Variant::NumberRange as i32 => Variant::NumberRange,
        x if x == Variant::ItemOfCollection as i32 => Variant::ItemOfCollection,
        x if x == Variant::ContainsType as i32 => Variant::ContainsType,
        x if x == Variant::ContainsKey as i32 => Variant::ContainsKey,
        _ => Variant::Unknown,
    }
}

pub fn verify_body(flow: Flow, body: Value, data_type: DataType) -> Result<(), DataTypeRuleError> {
    let mut violations: Vec<DataTypeRuleViolation> = Vec::new();

    for rule in data_type.rules {
        let varriant = convert_to_variant(rule.variant);

        match varriant {
            Variant::NumberRange => panic!("not implemented"),
            Variant::ItemOfCollection => panic!("not implemented"),
            Variant::ContainsType => panic!("not implemented"),
            Variant::Unknown => continue,
            Variant::Regex => {
                //This will be replaced through typed rules!
                let rule_definition = match rule.config {
                    None => panic!("No Regex expression present"),
                    Some(config) => to_regex_rule(config)?,
                };

                match apply_regex(rule_definition, body.clone()) {
                    Ok(_) => continue,
                    Err(violation) => {
                        violations.extend(violation.violations);
                        continue;
                    }
                };
            }
            Variant::ContainsKey => {
                //This will be replaced through typed rules!
                let rule_definition = match rule.config {
                    None => panic!("No rule definition present"),
                    Some(config) => to_contains_value_rule(config)?,
                };

                match apply_contains_key(rule_definition, body.clone(), flow.clone()) {
                    Ok(_) => continue,
                    Err(violation) => {
                        violations.extend(violation.violations);
                        continue;
                    }
                };
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(DataTypeRuleError { violations })
    }
}

// Will be replaced through typed rules!
fn to_regex_rule(config: Struct) -> Result<RegexRule, DataTypeRuleError> {
    let pattern = match config.fields.get("pattern") {
        Some(value) => {
            let kind = value.kind.clone().expect("");
            match kind {
                tucana::shared::value::Kind::StringValue(str) => str,
                _ => panic!(""),
            }
        }
        None => panic!("no pattern present"),
    };

    Ok(RegexRule { pattern })
}

// Will be replaced through typed rules!
fn to_contains_value_rule(config: Struct) -> Result<ContainsRule, DataTypeRuleError> {
    println!("incomming config: {:?}", config);
    let key = match config.fields.get("key") {
        Some(value) => {
            let kind = value.kind.clone().expect("");
            match kind {
                tucana::shared::value::Kind::StringValue(str) => str,
                _ => panic!("Wrong kind present"),
            }
        }
        None => panic!("No key present"),
    };

    let r#type = match config.fields.get("type") {
        Some(value) => {
            let kind = value.kind.clone().expect("");
            match kind {
                tucana::shared::value::Kind::StringValue(str) => str,
                _ => panic!("Wrong kind present"),
            }
        }
        None => panic!("No type present"),
    };

    Ok(ContainsRule { key, r#type })
}
