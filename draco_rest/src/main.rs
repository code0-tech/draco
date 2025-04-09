use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;

use code0_flow::flow_store::connection::create_flow_store_connection;
use code0_flow::flow_store::service::{FlowStoreService, FlowStoreServiceBase};
use draco_base::FromEnv;
use tokio::sync::Mutex;

#[derive(FromEnv)]
struct Config {
    port: u16,
    redis_url: String,
    rabbitmq_url: String,
}

#[tokio::main]
async fn main() {
    let config = Config::from_file("./.env");
    let url = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(url).unwrap();

    let flow_client = create_flow_store_connection(String::from("redis://localhost:6379")).await;
    let flow_client_service = Arc::new(Mutex::new(FlowStoreService::new(flow_client).await));

    // Listen for incoming TCP connections
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, flow_client_service.clone()).await;
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    flow_client_service: Arc<Mutex<FlowStoreService>>,
) {
    let buf_reader = BufReader::new(&stream);
    let raw_http_request: Vec<String> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let http_request = parse_request(raw_http_request);

    println!("Request: {:#?}", http_request);

    let response = "HTTP/1.1 200 OK\r\n\r\n";

    stream.write_all(response.as_bytes()).unwrap();
}

#[derive(Debug)]
enum HttpOption {
    GET,
    POST,
    PUT,
    DELETE,
}

impl FromStr for HttpOption {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "GET" => Ok(HttpOption::GET),
            "POST" => Ok(HttpOption::POST),
            "PUT" => Ok(HttpOption::PUT),
            "DELETE" => Ok(HttpOption::DELETE),
            _ => Err(()),
        };
    }
}

#[derive(Debug)]
struct HttpRequest {
    method: HttpOption,
    path: String,
    version: String,
    headers: Vec<String>,
}

fn parse_request(raw_http_request: Vec<String>) -> HttpRequest {
    let params = &raw_http_request[0];

    if params.is_empty() {
        panic!("TODO")
    }

    let mut header_params = params.split(" ");
    let raw_method = header_params.next().unwrap();
    let path = header_params.next().unwrap();
    let version = header_params.next().unwrap();

    let method = HttpOption::from_str(raw_method).unwrap();

    return HttpRequest {
        method,
        path: path.to_string(),
        version: version.to_string(),
        headers: Vec::new(),
    };
}
