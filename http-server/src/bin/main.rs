extern crate http_server;

use http_server::http_server::HttpServer;
use serde::{Serialize, Deserialize};
use std::io;
use std::path::Path;
//use serde_json::Result as SerdeResult;

const SERVER_IP: &str = "127.0.0.1";
const SERVER_PORT: u16 = 3000;

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

    server.static_path("/public", Path::new("public"));

    server.get("/", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.redirect("/hi")
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/hello-world", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.status(200);
            response.write("Hello ")?;
            response.write("World")?;
            response.write("\r\n")?;
            response.end()
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/hello", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            let hello_str = String::from("Hello");
            let msg = HelloResponse { message: hello_str };
            response.json(&msg)
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/hi", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.content_type("text/html");
            response.status(200).send("<h1>Hello!</h1>")
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/file/json", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.send_file(Path::new("test.json"))
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/file/image", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.send_file(Path::new("/home/debian/workspace/wazahm/soap-bubble.jpg"))
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/file/octet", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.send_file(Path::new("/home/debian/workspace/wazahm/soap-bubble"))
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    server.get("/download", | _request, mut response | {
        let mut handler = || -> Result<(), io::Error> {
            response.download(Path::new("/home/debian/workspace/wazahm/soap-bubble.jpg"))
        };
        if let Err(x) = handler() {
            eprintln!("Error: {}", x);
        }
    });

    /*
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
    */

    let ret = server.run(SERVER_IP, SERVER_PORT);
    if let Err(x) = ret {
        eprintln!("Error: {}", x);
    }
}
