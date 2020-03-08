use tonic::{transport::{Server, Endpoint}};

use hello_world::greeter_server::GreeterServer;
use hello_world::greeter_gateway::GreeterGateway;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    // TODO: Convert channel to futrure.
    let channel = Endpoint::from_static("http://[::1]:50052")
        .connect()
        .await?;
    
    let greeter = GreeterGateway(channel);

    println!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
