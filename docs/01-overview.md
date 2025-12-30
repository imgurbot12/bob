# Bob - Web Server and Reverse Proxy

## Overview

Bob is a high-performance web server and reverse proxy service written in Rust, powered by the [actix-web](https://actix.rs/) framework. It provides a simple YAML-based configuration system with powerful features including HTTP file serving, reverse proxying, FastCGI support, and integrated security middleware.

## Key Features

- **High Performance**: Built on actix-web with experimental io_uring support
- **Simple Configuration**: YAML-based configuration with sensible defaults
- **Protocol Support**: HTTP/1.1 and HTTP/2 out of the box
- **TLS/SSL**: Native HTTPS support with SNI (Server Name Indication)
- **ModSecurity Integration**: Built-in OWASP ModSecurity WAF support
- **Request Chaining**: Chain multiple modules together with conditional routing
- **Flexible Middleware**: Authentication, rate limiting, IP filtering, URL rewriting

## Project Structure

The Bob project is organized as a Cargo workspace with two crates:

```
bob/
├── Cargo.toml           # Workspace configuration
├── bob/                  # Main application crate
│   ├── Cargo.toml
│   ├── build.rs         # Build script for man page generation
│   └── src/
│       ├── main.rs      # Application entry point
│       ├── cli.rs       # CLI command processing
│       ├── config/      # Configuration parsing and types
│       │   ├── mod.rs
│       │   ├── modules.rs
│       │   └── middleware.rs
│       └── tls/         # TLS client/server configuration
│           ├── mod.rs
│           ├── client.rs
│           └── server.rs
└── bob-cli/              # CLI library crate (shared types)
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

## Installation

### Build from Source

```bash
git clone https://github.com/imgurbot12/bob.git
cd bob/bob
cargo install --path .
```

### Enable Low Port Binding (Linux)

If binding to ports below 1024 (e.g., 80, 443):

```bash
sudo setcap cap_net_bind_service=+ep $(which bob)
```

## Quick Start

### Using Configuration File

```bash
# Uses ./config.yaml by default
bob run

# Specify a custom config file
bob run --config /path/to/config.yaml
```

### Quick File Server

```bash
# Serve current directory on localhost:8000
bob file-server

# Serve specific directory with options
bob file-server --root /var/www --listen 0.0.0.0:8080 --browse
```

### Quick Reverse Proxy

```bash
# Proxy localhost:8000 to https://api.example.com
bob reverse-proxy --from localhost:8000 --to https://api.example.com
```

### Quick FastCGI Client

```bash
# Connect to PHP-FPM socket
bob fastcgi 127.0.0.1:9000 --root /var/www/html
```

## Available CLI Commands

| Command | Description |
|---------|-------------|
| `run` | Start server with YAML configuration file |
| `file-server` | Quick file server mode |
| `reverse-proxy` | Quick reverse proxy mode |
| `fastcgi` | Quick FastCGI client mode |
| `passwd` | Generate bcrypt password hash for basic auth |
| `schema` | Generate JSON schema for configuration |

## Feature Flags

Bob uses Cargo feature flags to enable/disable functionality:

### Request Modules
| Feature | Description | Default |
|---------|-------------|---------|
| `fileserver` | HTTP file server module | Enabled |
| `rproxy` | Reverse proxy module | Enabled |
| `fastcgi` | FastCGI client module | Enabled |

### Middleware
| Feature | Description | Default |
|---------|-------------|---------|
| `middleware` | All middleware (meta-feature) | Enabled |
| `authn` | HTTP Basic Authentication | Enabled |
| `modsecurity` | OWASP ModSecurity WAF | Enabled |
| `rewrite` | URL rewriting (mod_rewrite style) | Enabled |
| `ipware` | Client IP detection | Enabled |
| `ipfilter` | IP whitelist/blacklist filtering | Enabled |
| `ratelimit` | Request rate limiting | Enabled |
| `timeout` | Request timeout handling | Enabled |

### Utility Features
| Feature | Description | Default |
|---------|-------------|---------|
| `schema` | JSON schema generation | Disabled |
| `doc` | Documentation image handling | Disabled |

## Dependencies

### Core Dependencies

- **actix-web 4.11.0**: Web framework with rustls and io_uring support
- **actix-chain**: Request chain management for module linking
- **rustls 0.23.x**: TLS implementation
- **clap 4.5.x**: Command-line argument parsing
- **serde / serde_yaml**: Configuration serialization

### Optional Dependencies

- **actix-files**: File serving
- **actix-revproxy**: Reverse proxy functionality
- **actix-fastcgi**: FastCGI client support
- **actix-authn**: Authentication middleware
- **actix-modsecurity**: ModSecurity WAF integration
- **actix-rewrite**: URL rewriting engine
- **actix-ip-filter**: IP filtering middleware
- **actix-extensible-rate-limit**: Rate limiting

## Environment Variables

| Variable | Description |
|----------|-------------|
| `BOB_LOG` | Log level configuration (e.g., `info`, `debug`, `warn`) |

## Minimum Supported Rust Version

Bob requires Rust edition 2024, which requires a recent nightly or stable Rust compiler.

## License

See the [LICENSE](../LICENSE) file for details.
