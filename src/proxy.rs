use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, copy, Result as IoResult};
use tokio::net::{TcpListener, TcpStream};
use crate::registry::SharedRegistry;

pub async fn start_tcp_proxy(
    listen_addr: SocketAddr,
    registry: Arc<SharedRegistry>
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(listen_addr).await?;
    println!("TCP proxy listening on {}", listen_addr);
    
    loop {
        match listener.accept().await {
            Ok((inbound, peer_addr)) => {
                println!("New proxy connection from {}", peer_addr);
                
                let registry_clone = registry.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_proxy_connection(inbound, registry_clone).await {
                        eprintln!("Error handling proxy connection: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting proxy connection: {}", e);
            }
        }
    }
}

async fn handle_proxy_connection(
    mut inbound: TcpStream,
    registry: Arc<SharedRegistry>
) -> IoResult<()> {
    // Simple proxy protocol: first read the service name
    let mut service_name_buf = [0u8; 256];
    let n = inbound.read(&mut service_name_buf).await?;
    
    if n == 0 {
        return Ok(());
    }
    
    // Extract service name
    let service_name = String::from_utf8_lossy(&service_name_buf[..n])
        .trim()
        .to_string();
    
    println!("Proxy request for service: {}", service_name);
    
    // Look up service in registry
    let service_info = {
        let registry_r = registry.read().await;
        registry_r.services.get(&service_name).cloned()
    };
    
    if let Some(info) = service_info {
        if info.status == crate::registry::ServiceStatus::Ready {
            // For testing, use localhost instead of virtual IP
            let local_addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), info.port);
            println!("Forwarding to local address {}", local_addr);
            
            // Connect to the actual service B
            match TcpStream::connect(local_addr).await {
                Ok(mut outbound) => {
                    // Set up bidirectional copy between inbound and outbound
                    let (mut ri, mut wi) = inbound.split();
                    let (mut ro, mut wo) = outbound.split();
                    
                    let client_to_server = copy(&mut ri, &mut wo);
                    let server_to_client = copy(&mut ro, &mut wi);
                    
                    tokio::try_join!(client_to_server, server_to_client)?;
                    println!("Proxy connection closed successfully");
                }
                Err(e) => {
                    eprintln!("Failed to connect to service: {}", e);
                }
            }
        } else {
            eprintln!("Service {} is not ready", service_name);
        }
    } else {
        eprintln!("Service {} not found in registry", service_name);
    }
    
    Ok(())
}

pub async fn start_http_proxy(
    listen_addr: SocketAddr,
    _registry: Arc<SharedRegistry>
) -> Result<(), Box<dyn std::error::Error>> {
    // Simplified: HTTP proxy functionality removed for compatibility reasons
    println!("HTTP proxy listening on {}", listen_addr);
    // Sleep indefinitely
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
