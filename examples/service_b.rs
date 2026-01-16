use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::Duration;
use netsel::client::ServiceClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service_a_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    let hostname = "test-service-3"; // Use unique hostname to avoid conflicts
    
    // Create service client
    let mut client = ServiceClient::new(service_a_addr, hostname.to_string());
    
    // Register with Service A
    println!("Registering service '{}' with Service A at {}", hostname, service_a_addr);
    let (assigned_ip, assigned_port) = client.register().await?;
    let assigned_addr = SocketAddr::new(assigned_ip, assigned_port);
    
    println!("Successfully registered! Assigned address: {}", assigned_addr);
    
    // Start heartbeats
    let heartbeat_client = client.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = heartbeat_client.send_heartbeat().await {
                eprintln!("Heartbeat failed: {}", e);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    
    // Use a larger offset to avoid port conflicts
    let echo_port = assigned_port + 2000; // Use 11000 instead of 10000
    let local_addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), echo_port);
    println!("Starting echo server on {} (local testing)", local_addr);
    let listener = TcpListener::bind(local_addr).await?;
    
    loop {
        match listener.accept().await {
            Ok((mut stream, peer_addr)) => {
                println!("New connection from {}", peer_addr);
                
                tokio::spawn(async move {
                    let mut buffer = [0u8; 1024];
                    loop {
                        let n = match stream.read(&mut buffer).await {
                            Ok(n) if n == 0 => {
                                println!("Connection closed by {}", peer_addr);
                                return;
                            }
                            Ok(n) => n,
                            Err(e) => {
                                eprintln!("Error reading from {}: {}", peer_addr, e);
                                return;
                            }
                        };
                        
                        println!("Received {} bytes from {}: {:?}", n, peer_addr, &buffer[..n]);
                        
                        if let Err(e) = stream.write_all(&buffer[..n]).await {
                            eprintln!("Error writing to {}: {}", peer_addr, e);
                            return;
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}