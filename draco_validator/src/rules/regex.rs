use super::violation::{
    DataTypeRuleError, DataTypeRuleViolation, RegexRuleTypeNotAcceptedViolation, RegexRuleViolation,
};
use crate::RegexRule;
use serde_json::Value;

/// # Regex Pattern Validation
///
/// This function validates if a value matches a specified regex pattern.
///
/// ## Process:
/// 1. Converts the input value to a string representation (if possible)
/// 2. Compiles the regex pattern from the rule
/// 3. Checks if the string representation matches the regex pattern
///
/// ## Error Handling:
/// - Returns a `RegexRuleTypeNotAcceptedViolation` if the value type cannot be converted to a string
///   (e.g., arrays, objects)
/// - Returns a `RegexRuleViolation` if the string representation does not match the specified pattern
///
pub fn apply_regex(rule: RegexRule, body: Value) -> Result<(), DataTypeRuleError> {
    let result = match body {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s,
        _ => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::RegexTypeNotAccepted(
                    RegexRuleTypeNotAcceptedViolation {
                        type_not_accepted: format!("{:?}", body),
                    },
                )],
            })
        }
    };

    let regex = regex::Regex::new(rule.pattern.as_str()).unwrap();

    if !regex.is_match(&result) {
        return Err(DataTypeRuleError {
            violations: vec![DataTypeRuleViolation::Regex(RegexRuleViolation {
                missing_regex: rule.pattern,
            })],
        });
    } else {
        Ok(())
    }
}
