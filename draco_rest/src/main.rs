use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
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
    // Create a buffer to read the request into
    let mut buffer = [0; 1024];

    // Read from the stream into our buffer
    stream.read(&mut buffer).unwrap();

    // Convert the buffer to a string
    let request = String::from_utf8_lossy(&buffer[..]);

    // Extract the request method and URL
    let request_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() >= 2 {
        let method = parts[0]; // GET, POST, etc.
        let url = parts[1]; // /path, /index.html, etc.

        println!("Request type: {}", method);
        println!("URL: {}", url);

        // Process based on request method
        match method {
            "GET" => {
                // Parse query parameters if present
                if let Some(query_start) = url.find('?') {
                    let query_str = &url[query_start + 1..];
                    println!("Query string: {}", query_str);

                    // Parse and print individual query parameters
                    for param in query_str.split('&') {
                        println!("Query param: {}", param);
                    }
                }
            }
            "POST" => {
                // Extract the body from the request
                if let Some(body_start) = request.find("\r\n\r\n") {
                    let body = &request[body_start + 4..];
                    println!("POST body: {}", body.trim_end_matches('\0'));
                } else {
                    println!("No body found in POST request");
                }
            }
            _ => {}
        }
    } else {
        println!("Could not parse request line: {}", request_line);
    }

    // Send a simple response
    let response = "HTTP/1.1 200 OK\r\n\r\nRequest received and parsed";

    {
        let mut service = flow_client_service.lock().await;
        let ids = service.get_all_flow_ids().await.unwrap();
        print!("{}", ids.len())

        //TODO: Verfiy flow exists & request has correct body!
    }

    //TODO: RabbitMQ Send and recieve

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
