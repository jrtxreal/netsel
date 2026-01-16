//! Service client implementation for NetSel
//! 
//! This module provides the `ServiceClient` struct, which allows services to register with the NetSel server
//! and send heartbeat messages to maintain their health status.

use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Service client for registering with NetSel server and sending heartbeats
/// 
/// This struct provides methods to register a service with the NetSel server, send periodic heartbeat
/// messages, and retrieve information about the assigned address.
/// 
/// The `ServiceClient` is `Clone`able, making it easy to share between tasks.
/// 
/// # Example
/// 
/// ```rust,ignore
/// use std::net::SocketAddr;
/// use std::str::FromStr;
/// use tokio::time::Duration;
/// use netsel::client::ServiceClient;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let server_addr = SocketAddr::from_str("127.0.0.1:9000")?;
///     let hostname = "my-service";
///     
///     // Create service client
///     let mut client = ServiceClient::new(server_addr, hostname.to_string());
///     
///     // Register with the server
///     let (assigned_ip, assigned_port) = client.register().await?;
///     println!("Successfully registered! Assigned address: {}:{}", assigned_ip, assigned_port);
///     
///     // Start sending heartbeats
///     let heartbeat_client = client.clone();
///     tokio::spawn(async move {
///         loop {
///             if let Err(e) = heartbeat_client.send_heartbeat().await {
///                 eprintln!("Heartbeat failed: {}", e);
///             }
///             tokio::time::sleep(Duration::from_secs(10)).await;
///         }
///     });
///     
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct ServiceClient {
    server_addr: SocketAddr,
    hostname: String,
    registered: bool,
    assigned_ip: Option<IpAddr>,
    assigned_port: Option<u16>,
}

impl ServiceClient {
    /// Create a new ServiceClient instance
    /// 
    /// # Arguments
    /// 
    /// * `server_addr` - The address of the NetSel server's registration endpoint
    /// * `hostname` - The unique hostname for this service
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// use netsel::client::ServiceClient;
    /// 
    /// let server_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    /// let client = ServiceClient::new(server_addr, "my-service".to_string());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(server_addr: SocketAddr, hostname: String) -> Self {
        Self {
            server_addr,
            hostname,
            registered: false,
            assigned_ip: None,
            assigned_port: None,
        }
    }
    
    /// Register the service with the NetSel server
    /// 
    /// This method sends a registration request to the NetSel server, which will assign an IP address
    /// and port to the service. It returns the assigned IP and port if registration is successful.
    /// 
    /// # Returns
    /// 
    /// * `Ok((IpAddr, u16))` - The assigned IP address and port if registration succeeds
    /// * `Err(Box<dyn std::error::Error>)` - If registration fails
    /// 
    /// # Example
    /// 
    /// ```rust,ignore
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// use netsel::client::ServiceClient;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    ///     let mut client = ServiceClient::new(server_addr, "my-service".to_string());
    ///     
    ///     let (ip, port) = client.register().await?;
    ///     println!("Assigned address: {}:{}", ip, port);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn register(&mut self) -> Result<(IpAddr, u16), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(self.server_addr).await?;
        
        // Send hostname (padded to 256 bytes with null terminator)
        let mut hostname_buf = [0u8; 256];
        let hostname_bytes = self.hostname.as_bytes();
        hostname_buf[..hostname_bytes.len()].copy_from_slice(hostname_bytes);
        stream.write_all(&hostname_buf).await?;
        
        // Read response
        let mut response_buf = [0u8; 512];
        let n = stream.read(&mut response_buf).await?;
        let response = String::from_utf8_lossy(&response_buf[..n])
            .trim_end_matches(char::from(0))
            .to_string();
        
        println!("Registration response: {}", response);
        
        let parts: Vec<&str> = response.split('|').collect();
        if parts.len() < 4 || parts[0] != "SUCCESS" {
            return Err(format!("Registration failed: {}", response).into());
        }
        
        let ip = parts[1].parse::<IpAddr>()?;
        let port = parts[2].parse::<u16>()?;
        
        self.registered = true;
        self.assigned_ip = Some(ip);
        self.assigned_port = Some(port);
        
        Ok((ip, port))
    }
    
    /// Send a heartbeat message to the NetSel server
    /// 
    /// This method sends a heartbeat message to the NetSel server to indicate that the service is still alive.
    /// Heartbeat messages should be sent periodically to maintain the service's health status.
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If the heartbeat is sent successfully
    /// * `Err(Box<dyn std::error::Error>)` - If sending the heartbeat fails
    /// 
    /// # Example
    /// 
    /// ```rust,ignore
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// use tokio::time::Duration;
    /// use netsel::client::ServiceClient;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    ///     let client = ServiceClient::new(server_addr, "my-service".to_string());
    ///     
    ///     // Send heartbeat every 10 seconds
    ///     loop {
    ///         if let Err(e) = client.send_heartbeat().await {
    ///             eprintln!("Heartbeat failed: {}", e);
    ///         }
    ///         tokio::time::sleep(Duration::from_secs(10)).await;
    ///     }
    /// }
    /// ```
    pub async fn send_heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        // For simplicity, we'll reuse the registration protocol for heartbeats
        let mut stream = TcpStream::connect(self.server_addr).await?;
        
        // Send hostname with a special prefix for heartbeat
        let heartbeat_msg = format!("HEARTBEAT|{}", self.hostname);
        let mut buf = [0u8; 256];
        let msg_bytes = heartbeat_msg.as_bytes();
        buf[..msg_bytes.len()].copy_from_slice(msg_bytes);
        stream.write_all(&buf).await?;
        
        Ok(())
    }
    
    /// Get the assigned address for this service
    /// 
    /// This method returns the SocketAddr that was assigned to this service during registration.
    /// If the service is not yet registered, it returns None.
    /// 
    /// # Returns
    /// 
    /// * `Some(SocketAddr)` - The assigned address if registered
    /// * `None` - If the service is not registered
    /// 
    /// # Example
    /// 
    /// ```rust,ignore
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// use netsel::client::ServiceClient;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    ///     let mut client = ServiceClient::new(server_addr, "my-service".to_string());
    ///     
    ///     // Register first
    ///     client.register().await?;
    ///     
    ///     // Get assigned address
    ///     if let Some(addr) = client.get_assigned_addr() {
    ///         println!("Assigned address: {}", addr);
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn get_assigned_addr(&self) -> Option<SocketAddr> {
        match (self.assigned_ip, self.assigned_port) {
            (Some(ip), Some(port)) => Some(SocketAddr::new(ip, port)),
            _ => None,
        }
    }
    
    /// Check if the service is registered
    /// 
    /// This method returns whether the service has been successfully registered with the NetSel server.
    /// 
    /// # Returns
    /// 
    /// * `true` - If the service is registered
    /// * `false` - If the service is not registered
    /// 
    /// # Example
    /// 
    /// ```rust,ignore
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// use netsel::client::ServiceClient;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    ///     let mut client = ServiceClient::new(server_addr, "my-service".to_string());
    ///     
    ///     println!("Is registered: {}", client.is_registered()); // false
    ///     
    ///     client.register().await?;
    ///     
    ///     println!("Is registered: {}", client.is_registered()); // true
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn is_registered(&self) -> bool {
        self.registered
    }
}
