use tokio::signal;
use netsel::NetSelServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and start NetSel server with default configuration
    let server = NetSelServer::new();
    server.start().await?;
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    println!("Shutting down NetSel Service...");
    
    Ok(())
}