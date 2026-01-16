//! NetSel - Service Registration and Discovery System
//! 
//! A lightweight, Rust-based service registration and discovery system designed for distributed applications.
//! 
//! NetSel provides a simple yet powerful way to register services, discover them through a DNS server, and route traffic
//! between them using built-in proxies. It's built with Tokio for high performance and reliability.
//! 
//! ## Features
//! 
//! - **Service Registration**: Simple API for services to register themselves
//! - **Heartbeat Mechanism**: Automatic service health monitoring
//! - **DNS Resolution**: Service name to IP address resolution
//! - **TCP and HTTP Proxies**: Built-in traffic routing
//! - **Virtual Network**: Simplified IP address management
//! - **Concurrent Design**: Built with Tokio for high performance
//! - **Fault Tolerance**: Graceful handling of service failures
//! 
//! ## Getting Started
//! 
//! ```rust,ignore
//! use tokio::signal;
//! use netsel::NetSelServer;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and start NetSel server with default configuration
//!     let server = NetSelServer::new();
//!     server.start().await?;
//!     
//!     // Wait for shutdown signal
//!     signal::ctrl_c().await?;
//!     println!("Shutting down NetSel Service...");
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## Service Client Example
//! 
//! ```rust,ignore
//! use std::net::{IpAddr, SocketAddr};
//! use std::str::FromStr;
//! use tokio::time::Duration;
//! use netsel::client::ServiceClient;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let service_a_addr = SocketAddr::from_str("127.0.0.1:9000")?;
//!     let hostname = "my-service";
//!     
//!     // Create service client
//!     let mut client = ServiceClient::new(service_a_addr, hostname.to_string());
//!     
//!     // Register with Service A
//!     let (assigned_ip, assigned_port) = client.register().await?;
//!     let assigned_addr = SocketAddr::new(assigned_ip, assigned_port);
//!     
//!     println!("Successfully registered! Assigned address: {}", assigned_addr);
//!     
//!     // Start heartbeats
//!     let heartbeat_client = client.clone();
//!     tokio::spawn(async move {
//!         loop {
//!             if let Err(e) = heartbeat_client.send_heartbeat().await {
//!                 eprintln!("Heartbeat failed: {}", e);
//!             }
//!             tokio::time::sleep(Duration::from_secs(10)).await;
//!         }
//!     });
//!     
//!     // ... start your service logic
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## Architecture
//! 
//! The NetSel system consists of several components working together:
//! 
//! 1. **Registry Server**: Manages service registration and heartbeat messages
//! 2. **TCP Proxy**: Routes TCP traffic between registered services
//! 3. **HTTP Proxy**: Routes HTTP requests between registered services
//! 4. **DNS Server**: Resolves service names to IP addresses
//! 5. **Virtual Network**: Manages IP address allocation for services
//! 6. **Health Checker**: Monitors service health based on heartbeat messages
//! 7. **Service Client**: Library for services to register and send heartbeats
//! 
//! ## Modules
//! 
//! - `client`: Service client implementation for registering services and sending heartbeats
//! - `dns`: DNS server implementation for service discovery
//! - `network`: Virtual network implementation for IP allocation
//! - `proxy`: TCP and HTTP proxy implementations for traffic routing
//! - `registry`: Service registry implementation for managing service information

pub mod client;
pub mod dns;
pub mod network;
pub mod proxy;
pub mod registry;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::time::Duration;

use crate::registry::SharedRegistry;

/// Main NetSel server configuration
/// 
/// This struct defines the configuration for the NetSel server, including addresses for all components
/// and health check settings.
/// 
/// # Example
/// 
/// ```rust
/// use std::net::{IpAddr, SocketAddr};
/// use netsel::NetSelConfig;
/// 
/// let config = NetSelConfig {
///     registry_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 9000),
///     tcp_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8080),
///     http_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8081),
///     dns_addr: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 5353),
///     health_check_interval: 30,
///     max_heartbeat_age: 60,
/// };
/// ```
pub struct NetSelConfig {
    /// Address for the registration server where services will register themselves
    pub registry_addr: SocketAddr,
    /// Address for the TCP proxy that routes TCP traffic between services
    pub tcp_proxy_addr: SocketAddr,
    /// Address for the HTTP proxy that routes HTTP requests between services
    pub http_proxy_addr: SocketAddr,
    /// Address for the DNS server that resolves service names to IP addresses
    pub dns_addr: SocketAddr,
    /// Health check interval in seconds - how often to check for inactive services
    pub health_check_interval: u64,
    /// Max allowed time since last heartbeat before removing a service (in seconds)
    pub max_heartbeat_age: u64,
}

impl Default for NetSelConfig {
    fn default() -> Self {
        Self {
            registry_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 9000),
            tcp_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8080),
            http_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8081),
            dns_addr: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 5353),
            health_check_interval: 30,
            max_heartbeat_age: 60,
        }
    }
}

/// NetSel server instance
/// 
/// This struct represents a NetSel server instance, which manages all components of the NetSel system.
/// It provides methods to create and start the server with default or custom configuration.
/// 
/// # Example
/// 
/// ```rust
/// use std::net::{IpAddr, SocketAddr};
/// use netsel::{NetSelConfig, NetSelServer};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a custom configuration
///     let config = NetSelConfig {
///         registry_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 9000),
///         tcp_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8080),
///         http_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8081),
///         dns_addr: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 5353),
///         health_check_interval: 30,
///         max_heartbeat_age: 60,
///     };
///     
///     // Create server with custom configuration
///     let server = NetSelServer::with_config(config);
///     
///     // Start the server
///     server.start().await?;
///     
///     Ok(())
/// }
/// ```
pub struct NetSelServer {
    config: NetSelConfig,
    registry: Arc<SharedRegistry>,
}

impl NetSelServer {
    /// Create a new NetSel server with default configuration
    /// 
    /// This creates a NetSel server with the following default settings:
    /// - Registry server: `0.0.0.0:9000`
    /// - TCP proxy: `0.0.0.0:8080`
    /// - HTTP proxy: `0.0.0.0:8081`
    /// - DNS server: `127.0.0.1:5353`
    /// - Health check interval: 30 seconds
    /// - Max heartbeat age: 60 seconds
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use netsel::NetSelServer;
    /// 
    /// let server = NetSelServer::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(NetSelConfig::default())
    }

    /// Create a new NetSel server with custom configuration
    /// 
    /// This creates a NetSel server with the specified configuration.
    /// 
    /// # Arguments
    /// 
    /// * `config` - The custom configuration for the NetSel server
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use std::net::{IpAddr, SocketAddr};
    /// use netsel::{NetSelConfig, NetSelServer};
    /// 
    /// let config = NetSelConfig {
    ///     registry_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 9000),
    ///     // ... other configuration settings
    ///     ..NetSelConfig::default()
    /// };
    /// 
    /// let server = NetSelServer::with_config(config);
    /// ```
    pub fn with_config(config: NetSelConfig) -> Self {
        let registry = Arc::new(SharedRegistry::new(registry::ServiceRegistry::new()));
        
        Self {
            config,
            registry,
        }
    }

    /// Start the NetSel server and all its components
    /// 
    /// This method starts all components of the NetSel server in separate Tokio tasks:
    /// 1. Virtual network
    /// 2. DNS server
    /// 3. TCP proxy
    /// 4. HTTP proxy
    /// 5. Registration server
    /// 6. Health check task
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - If all components started successfully
    /// * `Err(Box<dyn std::error::Error>)` - If any component failed to start
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use netsel::NetSelServer;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server = NetSelServer::new();
    ///     server.start().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting NetSel Service...");
        
        // Start virtual network
        let mut virtual_net = network::VirtualNetwork::new();
        tokio::spawn(async move {
            virtual_net.run().await;
        });
        
        // Start DNS server
        let dns_addr = self.config.dns_addr;
        let registry_dns = self.registry.clone();
        tokio::spawn(async move {
            if let Err(e) = dns::start_dns_server(dns_addr, registry_dns).await {
                eprintln!("DNS server error: {}", e);
            }
        });
        
        // Start TCP proxy
        let tcp_proxy_addr = self.config.tcp_proxy_addr;
        let registry_tcp = self.registry.clone();
        tokio::spawn(async move {
            if let Err(e) = proxy::start_tcp_proxy(tcp_proxy_addr, registry_tcp).await {
                eprintln!("TCP proxy error: {}", e);
            }
        });
        
        // Start HTTP proxy
        let http_proxy_addr = self.config.http_proxy_addr;
        let registry_http = self.registry.clone();
        tokio::spawn(async move {
            if let Err(e) = proxy::start_http_proxy(http_proxy_addr, registry_http).await {
                eprintln!("HTTP proxy error: {}", e);
            }
        });
        
        // Start registration server
        let reg_server_addr = self.config.registry_addr;
        let registry_reg = self.registry.clone();
        tokio::spawn(async move {
            if let Err(e) = registry::start_registration_server(reg_server_addr, registry_reg).await {
                eprintln!("Registration server error: {}", e);
            }
        });
        
        // Start health check task
        let registry_health = self.registry.clone();
        let health_check_interval = self.config.health_check_interval;
        let max_heartbeat_age = self.config.max_heartbeat_age;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(health_check_interval));
            loop {
                interval.tick().await;
                let mut registry_w = registry_health.write().await;
                registry_w.cleanup_offline(Duration::from_secs(max_heartbeat_age));
            }
        });
        
        println!("NetSel Service started successfully!");
        println!("- Registration server: {}", self.config.registry_addr);
        println!("- TCP proxy: {}", self.config.tcp_proxy_addr);
        println!("- HTTP proxy: {}", self.config.http_proxy_addr);
        println!("- DNS server: {}", self.config.dns_addr);
        
        Ok(())
    }
}
