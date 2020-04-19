mod http_server;

use http_server::HttpServer;
use serde::{Serialize, Deserialize};
use serde_json::Result as SerdeResult;

const SERVER_IP: &str = "127.0.0.1";
const SERVER_PORT: u16 = 8080;

#[derive(Deserialize, Debug)]
struct HelloRequest {
    name: String
}

#[derive(Serialize, Debug)]
struct HelloResponse {
    message: String
}

fn main() {
    let mut server = HttpServer::new();

    server.post("/hello", | request, response | {
        let json_body: SerdeResult<HelloRequest> = serde_json::from_slice(&request.body);
        match json_body {
            Ok(hello_req) => {
                let hello_res = HelloResponse { message: String::from("Hi, ") + &hello_req.name + "!" };
                let data = serde_json::to_string(&hello_res).unwrap();
                // response.send(&data);
            },
            Err(error) => {
                let data = error.to_string();
                // response.send(&data);
            }
        }
    });

    server.run(SERVER_IP, SERVER_PORT);
}
