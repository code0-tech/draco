pub mod resolver;
pub mod rules;

use rules::{
    contains_key::apply_contains_key,
    regex::apply_regex,
    violation::{DataTypeNotFoundRuleViolation, DataTypeRuleError, DataTypeRuleViolation},
};
use tucana::shared::{data_type_rule::Config, DataType, Flow, Value};

pub struct VerificationResult;

pub fn verify_flow(flow: Flow, body: Value) -> Result<(), DataTypeRuleError> {
    let input_type = match &flow.input_type_identifier {
        Some(r) => r.clone(),
        None => return Ok(()), //Returns directly because no rule is given. The body is ok and will not be concidered
    };

    let data_type = match flow
        .data_types
        .iter()
        .find(|dt| dt.identifier == input_type)
    {
        Some(dt) => dt.clone(),
        None => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::DataTypeNotFound(
                    DataTypeNotFoundRuleViolation {
                        data_type: input_type,
                    },
                )],
            });
        }
    };

    verify_body(flow, body, data_type)
}

pub fn verify_body(flow: Flow, body: Value, data_type: DataType) -> Result<(), DataTypeRuleError> {
    let mut violations: Vec<DataTypeRuleViolation> = Vec::new();
    for rule in data_type.rules {
        let rule_config = match rule.config {
            None => continue,
            Some(config) => config,
        };

        match rule_config {
            Config::NumberRange(_) => panic!("not implemented"),
            Config::ItemOfCollection(_) => panic!("not implemented"),
            Config::ContainsType(_) => panic!("not implemented"),
            Config::Regex(config) => {
                match apply_regex(config, &body) {
                    Ok(_) => continue,
                    Err(violation) => {
                        violations.extend(violation.violations);
                        continue;
                    }
                };
            }
            Config::ContainsKey(config) => {
                match apply_contains_key(config, body.clone(), flow.clone()) {
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
