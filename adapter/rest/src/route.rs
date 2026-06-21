use base::traits::IdentifiableFlow;
use std::collections::HashMap;
use tucana::shared::{ValidationFlow, value::Kind};

pub struct RequestRoute {
    pub url: String,
    pub method: hyper::Method,
}

// Checks if the Method and Url matches any of the
// Flows that matched the original slug pattern for project
// Only if both matched, it will return true
impl IdentifiableFlow for RequestRoute {
    fn identify(&self, flow: &ValidationFlow) -> bool {
        log::debug!(
            "route identify start: flow_id={} project_slug={} request_method={} request_path={:?}",
            flow.flow_id,
            flow.project_slug,
            self.method.as_str(),
            self.url
        );

        let Some(flow_method) = extract_flow_setting_as_string(flow, "httpMethod") else {
            log::debug!(
                "route identify reject: flow_id={} reason=missing_or_invalid_httpMethod",
                flow.flow_id
            );
            return false;
        };

        log::debug!(
            "route identify method check: flow_id={} flow_method={} request_method={}",
            flow.flow_id,
            flow_method,
            self.method.as_str()
        );

        if flow_method != self.method.as_str() {
            log::debug!(
                "route identify reject: flow_id={} reason=method_mismatch flow_method={} request_method={}",
                flow.flow_id,
                flow_method,
                self.method.as_str()
            );
            return false;
        }

        let Some(flow_http_url) = extract_flow_setting_as_string(flow, "httpURL") else {
            log::debug!(
                "route identify reject: flow_id={} reason=missing_or_invalid_httpURL",
                flow.flow_id
            );
            return false;
        };

        let route_pattern = flow_route_pattern(flow, flow_http_url);
        log::debug!(
            "route identify route check: flow_id={} httpURL={:?} resolved_pattern={:?} request_path={:?}",
            flow.flow_id,
            flow_http_url,
            route_pattern,
            self.url
        );

        let is_match = matches_route_pattern(&route_pattern, &self.url);
        log::debug!(
            "route identify result: flow_id={} matched={}",
            flow.flow_id,
            is_match
        );
        is_match
    }
}

pub fn extract_slug_from_path(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    trimmed.split('/').next().filter(|s| !s.is_empty())
}

pub fn extract_path_params(flow: &ValidationFlow, path: &str) -> HashMap<String, String> {
    let Some(flow_http_url) = extract_flow_setting_as_string(flow, "httpURL") else {
        return HashMap::new();
    };

    extract_named_route_captures(&flow_route_pattern(flow, flow_http_url), path)
}

fn flow_route_pattern(flow: &ValidationFlow, flow_http_url: &str) -> String {
    format!("/{}{}", flow.project_slug, flow_http_url)
}

fn matches_route_pattern(pattern: &str, route: &str) -> bool {
    let anchored_pattern = match compile_route_pattern(pattern) {
        Ok(pattern) => pattern,
        Err(err) => {
            log::error!(
                "route pattern invalid: raw_pattern={:?} error={}",
                pattern,
                err
            );
            return false;
        }
    };
    log::debug!(
        "route pattern eval: raw_pattern={:?} anchored_pattern={:?} route={:?}",
        pattern,
        anchored_pattern,
        route
    );

    let regex = match regex::Regex::new(&anchored_pattern) {
        Ok(regex) => regex,
        Err(err) => {
            log::error!(
                "route pattern invalid regex: anchored_pattern={:?} error={}",
                anchored_pattern,
                err
            );
            return false;
        }
    };

    let is_match = regex.is_match(route);
    log::debug!(
        "route pattern result: anchored_pattern={:?} route={:?} matched={}",
        anchored_pattern,
        route,
        is_match
    );
    is_match
}

fn extract_named_route_captures(pattern: &str, route: &str) -> HashMap<String, String> {
    let anchored_pattern = match compile_route_pattern(pattern) {
        Ok(pattern) => pattern,
        Err(err) => {
            log::error!(
                "route path params invalid pattern: raw_pattern={:?} error={}",
                pattern,
                err
            );
            return HashMap::new();
        }
    };
    let regex = match regex::Regex::new(&anchored_pattern) {
        Ok(regex) => regex,
        Err(err) => {
            log::error!(
                "route path params invalid regex: anchored_pattern={:?} error={}",
                anchored_pattern,
                err
            );
            return HashMap::new();
        }
    };

    let Some(captures) = regex.captures(route) else {
        return HashMap::new();
    };

    regex
        .capture_names()
        .flatten()
        .filter_map(|name| {
            captures.name(name).map(|value| {
                (
                    name.to_string(),
                    percent_encoding::percent_decode_str(value.as_str())
                        .decode_utf8_lossy()
                        .into_owned(),
                )
            })
        })
        .collect()
}

fn compile_route_pattern(pattern: &str) -> Result<String, String> {
    if is_url_pattern_style(pattern) {
        return compile_url_pattern_path(pattern).map(|pattern| format!("^{}$", pattern));
    }

    Ok(format!("^{}$", pattern))
}

fn is_url_pattern_style(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();

    bytes.iter().enumerate().any(|(index, byte)| {
        (*byte == b':'
            && bytes
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_alphabetic() || *next == b'_'))
            || (*byte == b'*' && bytes.get(index.wrapping_sub(1)) != Some(&b'.'))
    })
}

fn compile_url_pattern_path(pattern: &str) -> Result<String, String> {
    let mut compiled = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            ':' if is_param_start(chars.get(index + 1).copied()) => {
                let (name, next_index) = read_param_name(&chars, index + 1);
                index = next_index;

                let (capture_pattern, next_index) = if chars.get(index) == Some(&'(') {
                    read_balanced_group(&chars, index)?
                } else {
                    (String::from("[^/]+"), index)
                };

                compiled.push_str(&format!("(?P<{name}>{capture_pattern})"));
                index = next_index;
            }
            '*' => {
                compiled.push_str(".*");
                index += 1;
            }
            value => {
                compiled.push_str(&regex::escape(&value.to_string()));
                index += 1;
            }
        }
    }

    Ok(compiled)
}

fn is_param_start(value: Option<char>) -> bool {
    value.is_some_and(|value| value.is_ascii_alphabetic() || value == '_')
}

fn read_param_name(chars: &[char], start: usize) -> (String, usize) {
    let mut index = start;
    let mut name = String::new();

    while let Some(value) = chars.get(index) {
        if value.is_ascii_alphanumeric() || *value == '_' {
            name.push(*value);
            index += 1;
        } else {
            break;
        }
    }

    (name, index)
}

fn read_balanced_group(chars: &[char], start: usize) -> Result<(String, usize), String> {
    let mut depth = 0;
    let mut index = start;
    let mut pattern = String::new();

    while let Some(value) = chars.get(index) {
        match value {
            '(' => {
                depth += 1;
                if depth > 1 {
                    pattern.push(*value);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((pattern, index + 1));
                }
                pattern.push(*value);
            }
            _ => pattern.push(*value),
        }

        index += 1;
    }

    Err(String::from("unclosed parameter regex group"))
}

fn extract_flow_setting_as_string<'a>(
    flow: &'a ValidationFlow,
    flow_setting_id: &str,
) -> Option<&'a str> {
    let setting = match flow
        .settings
        .iter()
        .find(|setting| setting.flow_setting_id == flow_setting_id)
    {
        Some(setting) => setting,
        None => {
            log::debug!(
                "flow setting is missing: flow_id={} flow_setting_id={}",
                flow.flow_id,
                flow_setting_id
            );
            return None;
        }
    };

    let value = match setting.value.as_ref() {
        Some(value) => value,
        None => {
            log::debug!(
                "flow setting has no value: flow_id={} flow_setting_id={}",
                flow.flow_id,
                flow_setting_id
            );
            return None;
        }
    };

    let kind = match value.kind.as_ref() {
        Some(kind) => kind,
        None => {
            log::debug!(
                "flow setting has no kind: flow_id={} flow_setting_id={}",
                flow.flow_id,
                flow_setting_id
            );
            return None;
        }
    };

    match kind {
        Kind::StringValue(value) => Some(value.as_str()),
        _ => {
            log::debug!(
                "flow setting has non-string kind: flow_id={} flow_setting_id={} kind={:?}",
                flow.flow_id,
                flow_setting_id,
                kind
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compile_route_pattern, extract_named_route_captures, matches_route_pattern};

    #[test]
    fn exact_literal_match_works() {
        assert!(matches_route_pattern("test2", "test2"));
    }

    #[test]
    fn substring_match_is_rejected() {
        assert!(!matches_route_pattern("test", "test2"));
    }

    #[test]
    fn nested_path_match_is_anchored() {
        assert!(matches_route_pattern("/api/v1/test2", "/api/v1/test2"));
        assert!(!matches_route_pattern(
            "/api/v1/test2",
            "/api/v1/test2/extra"
        ));
    }

    #[test]
    fn dynamic_route_params_match_path_segments() {
        assert!(matches_route_pattern(
            "/project/users/:id",
            "/project/users/42"
        ));
        assert!(!matches_route_pattern(
            "/project/users/:id",
            "/project/users/42/orders"
        ));
    }

    #[test]
    fn dynamic_route_params_are_returned_as_path_params() {
        let params = extract_named_route_captures(
            "/project/books/:category/:id",
            "/project/books/classics/12345",
        );

        assert_eq!(params.get("category").map(String::as_str), Some("classics"));
        assert_eq!(params.get("id").map(String::as_str), Some("12345"));
    }

    #[test]
    fn dynamic_route_params_support_regex_constraints() {
        assert!(matches_route_pattern(
            "/project/users/:user_id(\\d+)",
            "/project/users/123"
        ));
        assert!(!matches_route_pattern(
            "/project/users/:user_id(\\d+)",
            "/project/users/abc"
        ));

        let params =
            extract_named_route_captures("/project/users/:user_id(\\d+)", "/project/users/123");
        assert_eq!(params.get("user_id").map(String::as_str), Some("123"));
    }

    #[test]
    fn dynamic_route_wildcards_match_remaining_path() {
        assert!(matches_route_pattern(
            "/project/assets/*",
            "/project/assets/images/profile.jpg"
        ));
    }

    #[test]
    fn legacy_regex_patterns_still_work() {
        assert_eq!(
            compile_route_pattern("/project/users/(?P<user_id>[^/]+)").unwrap(),
            "^/project/users/(?P<user_id>[^/]+)$"
        );
        assert!(matches_route_pattern(
            "/project/users/(?P<user_id>[^/]+)",
            "/project/users/42"
        ));
        assert!(matches_route_pattern(
            "/project/assets/.*",
            "/project/assets/images/profile.jpg"
        ));
    }

    #[test]
    fn invalid_regex_returns_false() {
        assert!(!matches_route_pattern("(", "/test"));
    }

    #[test]
    fn named_route_captures_are_returned_as_path_params() {
        let params = extract_named_route_captures(
            "/project/users/(?P<user_id>[^/]+)/orders/(?P<order_id>[^/]+)",
            "/project/users/42/orders/abc",
        );

        assert_eq!(params.get("user_id").map(String::as_str), Some("42"));
        assert_eq!(params.get("order_id").map(String::as_str), Some("abc"));
    }

    #[test]
    fn named_route_captures_are_percent_decoded() {
        let params = extract_named_route_captures(
            "/project/files/(?P<file_name>[^/]+)",
            "/project/files/report%202026.txt",
        );

        assert_eq!(
            params.get("file_name").map(String::as_str),
            Some("report 2026.txt")
        );
    }

    #[test]
    fn unnamed_route_captures_are_ignored() {
        let params = extract_named_route_captures(
            "/project/users/([^/]+)/orders/(?P<order_id>[^/]+)",
            "/project/users/42/orders/abc",
        );

        assert!(!params.contains_key("1"));
        assert_eq!(params.get("order_id").map(String::as_str), Some("abc"));
    }
}
