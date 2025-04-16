use tucana::shared::{value::Kind, Value};

use super::violation::{
    DataTypeRuleError, DataTypeRuleViolation, RegexRuleTypeNotAcceptedViolation, RegexRuleViolation,
};
use crate::RegexRule;

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
    let kind = match body.kind {
        Some(kind) => kind,
        None => return Ok(()),
    };

    let result = match kind {
        Kind::BoolValue(b) => b.to_string(),
        Kind::NumberValue(n) => n.to_string(),
        Kind::StringValue(s) => s,
        _ => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::RegexTypeNotAccepted(
                    RegexRuleTypeNotAcceptedViolation {
                        type_not_accepted: format!("{:?}", kind),
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
