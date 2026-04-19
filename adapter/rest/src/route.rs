use base::traits::IdentifiableFlow;
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

        let route_pattern = format!("/{}{}", flow.project_slug, flow_http_url);
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

fn matches_route_pattern(pattern: &str, route: &str) -> bool {
    let anchored_pattern = format!("^{}$", pattern);
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
    use super::matches_route_pattern;

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
    fn invalid_regex_returns_false() {
        assert!(!matches_route_pattern("(", "/test"));
    }
}
