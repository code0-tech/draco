pub mod http {
    use std::str::FromStr;

    use serde_json::Value;

    #[derive(Debug)]
    pub enum HttpOption {
        GET,
        POST,
        PUT,
        DELETE,
    }

    impl FromStr for HttpOption {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "GET" => Ok(HttpOption::GET),
                "POST" => Ok(HttpOption::POST),
                "PUT" => Ok(HttpOption::PUT),
                "DELETE" => Ok(HttpOption::DELETE),
                _ => Err(()),
            }
        }
    }

    impl ToString for HttpOption {
        fn to_string(&self) -> String {
            match self {
                HttpOption::GET => "GET".to_string(),
                HttpOption::POST => "POST".to_string(),
                HttpOption::PUT => "PUT".to_string(),
                HttpOption::DELETE => "DELETE".to_string(),
            }
        }
    }

    #[derive(Debug)]
    pub struct HttpRequest {
        pub method: HttpOption,
        pub path: String,
        pub version: String,
        pub headers: Vec<String>,
        pub body: Option<Value>,
    }
}
