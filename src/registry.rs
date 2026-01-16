use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub hostname: String,
    pub ip: IpAddr,
    pub port: u16,
    pub addr: SocketAddr,
    pub registered_at: Instant,
    pub last_heartbeat: Instant,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Ready,
    Offline,
}

pub struct ServiceRegistry {
    pub services: HashMap<String, ServiceInfo>,
    port_pool: PortPool,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            port_pool: PortPool::new(9000, 9999),
        }
    }

    pub fn register(&mut self, hostname: String, ip: IpAddr) -> Option<ServiceInfo> {
        if self.services.contains_key(&hostname) {
            return None;
        }

        if let Some(port) = self.port_pool.allocate() {
            let addr = SocketAddr::new(ip, port);
            let now = Instant::now();
            let service_info = ServiceInfo {
                hostname: hostname.clone(),
                ip,
                port,
                addr,
                registered_at: now,
                last_heartbeat: now,
                status: ServiceStatus::Ready,
            };
            self.services.insert(hostname, service_info.clone());
            Some(service_info)
        } else {
            None
        }
    }

    pub fn unregister(&mut self, hostname: &str) -> bool {
        if let Some(service) = self.services.remove(hostname) {
            self.port_pool.release(service.port);
            true
        } else {
            false
        }
    }

    pub fn get_service(&self, hostname: &str) -> Option<&ServiceInfo> {
        self.services.get(hostname)
    }

    pub fn update_heartbeat(&mut self, hostname: &str) -> bool {
        if let Some(service) = self.services.get_mut(hostname) {
            service.last_heartbeat = Instant::now();
            service.status = ServiceStatus::Ready;
            true
        } else {
            false
        }
    }

    pub fn cleanup_offline(&mut self, timeout: Duration) {
        let now = Instant::now();
        let to_remove: Vec<String> = self.services
            .iter()
            .filter(|(_, service)| {
                now.duration_since(service.last_heartbeat) > timeout
            })
            .map(|(hostname, _)| hostname.clone())
            .collect();

        for hostname in to_remove {
            self.unregister(&hostname);
        }
    }
}

pub struct PortPool {
    start: u16,
    end: u16,
    used: HashSet<u16>,
}

impl PortPool {
    pub fn new(start: u16, end: u16) -> Self {
        Self {
            start,
            end,
            used: HashSet::new(),
        }
    }

    pub fn allocate(&mut self) -> Option<u16> {
        for port in self.start..=self.end {
            if !self.used.contains(&port) {
                self.used.insert(port);
                return Some(port);
            }
        }
        None
    }

    pub fn release(&mut self, port: u16) {
        self.used.remove(&port);
    }
}

pub type SharedRegistry = RwLock<ServiceRegistry>;

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Start the registration server
pub async fn start_registration_server(
    addr: SocketAddr,
    registry: Arc<SharedRegistry>
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    println!("Registration server listening on {}", addr);
    
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                println!("New registration from {}", peer_addr);
                
                let registry_clone = registry.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_registration(stream, registry_clone).await {
                        eprintln!("Error handling registration: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting registration connection: {}", e);
            }
        }
    }
}

/// Handle registration requests
async fn handle_registration(
    mut stream: tokio::net::TcpStream,
    registry: Arc<SharedRegistry>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).await?;
    
    if n == 0 {
        return Ok(());
    }
    
    let message = String::from_utf8_lossy(&buf[..n])
        .trim_end_matches(char::from(0))
        .to_string();
    
    println!("Received message: {}", message);
    
    if message.starts_with("HEARTBEAT|") {
        // Handle heartbeat - properly parse hostname and trim any whitespace
        let hostname = message.split('|').nth(1).unwrap_or("").trim();
        println!("Heartbeat from: {}", hostname);
        
        let success = {
            let mut registry_w = registry.write().await;
            registry_w.update_heartbeat(hostname)
        };
        
        // Send response without null terminator
        let response = if success {
            "HEARTBEAT_OK"
        } else {
            "HEARTBEAT_FAILED"
        };
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
    
    // Handle registration
    let hostname = message;
    println!("Registering service: {}", hostname);
    
    // Allocate IP from virtual network
    let virtual_net = crate::network::VirtualNetwork::new();
    let ip = virtual_net.allocate_ip();
    
    let registration_result = {
        let mut registry_w = registry.write().await;
        registry_w.register(hostname.clone(), ip)
    };
    
    match registration_result {
        Some(service_info) => {
            println!("Service registered successfully: {:?}", service_info);
            
            // Send registration response
            let response = format!("SUCCESS|{}|{}|86400\0", service_info.ip, service_info.port);
            stream.write_all(response.as_bytes()).await?;
        }
        None => {
            println!("Failed to register service: {}", hostname);
            let response = "FAILED|Service already registered or port unavailable\0";
            stream.write_all(response.as_bytes()).await?;
        }
    }
    
    Ok(())
}
