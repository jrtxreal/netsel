# NetSel - Service Registration and Discovery System

A lightweight, Rust-based service registration and discovery system designed for distributed applications. NetSel provides a simple yet powerful way to register services, discover them through a DNS server, and route traffic between them using built-in proxies.

## 📋 Table of Contents

- [Features](#-features)
- [How It Works](#-how-it-works)
- [Architecture](#-architecture)
- [Quick Start](#-quick-start)
- [Usage Examples](#-usage-examples)
- [Running Examples](#-running-examples)
- [Configuration](#-configuration)
- [Modules](#-modules)
- [Tech Stack](#-tech-stack)
- [Contributing](#-contributing)
- [License](#-license)
- [Support](#-support)

## ✨ Features

- **Service Registration**: Simple API for services to register themselves
- **Heartbeat Mechanism**: Automatic service health monitoring
- **DNS Resolution**: Service name to IP address resolution
- **TCP and HTTP Proxies**: Built-in traffic routing
- **Virtual Network**: Simplified IP address management
- **Concurrent Design**: Built with Tokio for high performance
- **Fault Tolerance**: Graceful handling of service failures

## 📡 How It Works

NetSel operates on a simple yet robust principle: services register themselves with a central registry, receive a virtual IP address, and then periodically send heartbeat messages to maintain their registration. Other services can discover registered services through DNS resolution or by querying the registry directly.

### Core Workflow

1. **Service Registration**: A service sends a registration request to the NetSel server
2. **IP Allocation**: NetSel assigns a virtual IP address and port to the service
3. **Heartbeat Mechanism**: The service sends periodic heartbeat messages (every 10 seconds by default)
4. **Service Discovery**: Other services can resolve the registered service's name to its virtual IP via DNS
5. **Traffic Routing**: The built-in TCP/HTTP proxies route traffic between services
6. **Health Monitoring**: NetSel automatically removes services that stop sending heartbeats

## 🏗️ Architecture

The NetSel system consists of several components working together:

### 1. Registry Server
- Manages service registration and heartbeat messages
- Maintains a list of active services
- Handles service deregistration and cleanup

### 2. TCP Proxy
- Routes TCP traffic between registered services
- Uses the registry to resolve service names to virtual IPs
- Provides load balancing capabilities

### 3. HTTP Proxy
- Routes HTTP requests between registered services
- Supports HTTP/1.1 and HTTP/2
- Maintains persistent connections

### 4. DNS Server
- Resolves service names to IP addresses
- Listens on a configurable port (default: 5353)
- Uses the registry as its data source

### 5. Virtual Network
- Manages IP address allocation for services
- Provides a simplified IP addressing scheme
- Handles IP conflicts automatically

### 6. Health Checker
- Monitors service health based on heartbeat messages
- Removes inactive services after a configurable timeout (default: 60 seconds)
- Runs periodic cleanup tasks

### 7. Service Client
- Library for services to register and send heartbeats
- Provides a simple API for service integration
- Built-in retry mechanisms for robustness

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+ (with Cargo)
- Tokio runtime

### Installation

Add NetSel to your Cargo.toml:

```toml
dependencies =
    tokio = { version = "1.37", features = ["full"] }
    netsel = { path = "." }
```

### Basic Usage

#### Starting the NetSel Server

```rust
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
```

#### Registering a Service

```rust
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tokio::time::Duration;
use netsel::client::ServiceClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service_a_addr = SocketAddr::from_str("127.0.0.1:9000")?;
    let hostname = "my-service";
    
    // Create service client
    let mut client = ServiceClient::new(service_a_addr, hostname.to_string());
    
    // Register with Service A
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
    
    // ... start your service logic
    
    Ok(())
}
```

## 📚 Usage Examples

### Example 1: Basic Server Setup

```rust
use netsel::NetSelServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = NetSelServer::new();
    server.start().await?;
    
    Ok(())
}
```

### Example 2: Custom Configuration

```rust
use std::net::{IpAddr, SocketAddr};
use netsel::{NetSelConfig, NetSelServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NetSelConfig {
        registry_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 9000),
        tcp_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8080),
        http_proxy_addr: SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 8081),
        dns_addr: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 5353),
        health_check_interval: 30,
        max_heartbeat_age: 60,
    };
    
    let server = NetSelServer::with_config(config);
    server.start().await?;
    
    Ok(())
}
```

## 🧪 Running Examples

The project includes three example programs that demonstrate the full NetSel workflow:

### 1. Start the NetSel Server

```bash
cargo run --example netsel_server
```

### 2. Register a Service

```bash
cargo run --example service_b
```

### 3. Test the System

```bash
cargo run --example test_client
```

### Example Output

```
# netsel_server output
Starting NetSel Service...
NetSel Service started successfully!
- Registration server: 0.0.0.0:9000
- TCP proxy: 0.0.0.0:8080
- HTTP proxy: 0.0.0.0:8081
- DNS server: 127.0.0.1:5353

# service_b output
Registering service 'test-service-3' with Service A at 127.0.0.1:9000
Registration response: SUCCESS|10.0.0.100|9000|86400
Successfully registered! Assigned address: 10.0.0.100:9000
Starting echo server on 127.0.0.1:11000 (local testing)

# test_client output
Testing NetSel system...

Test 1: Testing Service B's echo server directly...
Sending test data: Hello, NetSel Service B!
Received response: Hello, NetSel Service B!
✓ Echo server test passed!

Test 2: Checking Service A logs...
✓ Service A is receiving heartbeats from test-service-3
✓ Heartbeat mechanism test passed!

Test 3: Checking service registration...
✓ Service 'test-service-3' is registered successfully
✓ Service registration test passed!

🎉 All tests passed! NetSel system is working correctly.
```

## ⚙️ Configuration

NetSel can be configured using the `NetSelConfig` struct. Here are the available configuration options:

| Option | Default Value | Description |
|--------|---------------|-------------|
| `registry_addr` | `0.0.0.0:9000` | Address for the registration server |
| `tcp_proxy_addr` | `0.0.0.0:8080` | Address for the TCP proxy |
| `http_proxy_addr` | `0.0.0.0:8081` | Address for the HTTP proxy |
| `dns_addr` | `127.0.0.1:5353` | Address for the DNS server |
| `health_check_interval` | `30` | Health check interval in seconds |
| `max_heartbeat_age` | `60` | Maximum allowed time since last heartbeat before removing a service (seconds) |

## 📦 Modules

### `client`
- Service client implementation for registering services and sending heartbeats
- Provides the `ServiceClient` struct for service integration

### `dns`
- DNS server implementation for service discovery
- Resolves service names to IP addresses
- Built on trust-dns-server

### `network`
- Virtual network implementation for IP allocation
- Manages IP address assignments for registered services

### `proxy`
- TCP and HTTP proxy implementations for traffic routing
- Routes traffic between registered services

### `registry`
- Service registry implementation for managing service information
- Handles registration, heartbeat, and cleanup operations

## 🛠️ Tech Stack

| Technology | Purpose |
|------------|---------|
| Rust | Core programming language |
| Tokio | Async runtime for concurrency |
| Hyper | HTTP server and client library |
| trust-dns | DNS server implementation |
| socket2 | Low-level socket operations |

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

1. Clone the repository
2. Install dependencies: `cargo build`
3. Run tests: `cargo test`
4. Run examples: `cargo run --example <example_name>`

### Code Style

- Follow Rust's standard code style (enforced by rustfmt)
- Write comprehensive tests for new features
- Document all public APIs

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📞 Support

For questions or issues, please open an issue on the [GitHub repository](https://github.com/jrtxreal/netsel).

---

**NetSel** - Making service registration and discovery simple for distributed systems.
