# Bob Documentation Index

This directory contains comprehensive documentation for the Bob web server and reverse proxy project.

## Documentation Files

| File | Description |
|------|-------------|
| [01-overview.md](./01-overview.md) | Project overview, features, installation, and quick start |
| [02-architecture.md](./02-architecture.md) | System architecture, design patterns, and code organization |
| [03-modules.md](./03-modules.md) | Request module reference (FileServer, ReverseProxy, FastCGI, etc.) |
| [04-middleware.md](./04-middleware.md) | Middleware reference (Auth, RateLimit, ModSecurity, etc.) |
| [05-configuration.md](./05-configuration.md) | Complete configuration schema and options |
| [06-examples.md](./06-examples.md) | Practical configuration examples and use cases |

## Quick Links

### Getting Started
- [Installation](./01-overview.md#installation)
- [Quick Start](./01-overview.md#quick-start)
- [CLI Commands](./01-overview.md#available-cli-commands)

### Configuration
- [Server Configuration](./05-configuration.md#server-configuration-serverconfig)
- [Listener Setup](./05-configuration.md#listener-configuration-listencfg)
- [TLS/SSL Configuration](./05-configuration.md#ssl-configuration-sslcfg)
- [Domain Matching](./05-configuration.md#domain-matching-server_name)

### Modules
- [FileServer](./03-modules.md#fileserver-module)
- [ReverseProxy](./03-modules.md#reverseproxy-module)
- [FastCGI](./03-modules.md#fastcgi-module)
- [Redirect](./03-modules.md#redirect-module)
- [Static](./03-modules.md#static-module)

### Middleware
- [Basic Authentication](./04-middleware.md#authbasic-middleware)
- [Session Authentication](./04-middleware.md#authsession-middleware)
- [IP Detection (IpWare)](./04-middleware.md#ipware-middleware)
- [IP Filtering](./04-middleware.md#ipfilter-middleware)
- [ModSecurity WAF](./04-middleware.md#modsecurity-middleware)
- [URL Rewriting](./04-middleware.md#rewrite-middleware)
- [Rate Limiting](./04-middleware.md#ratelimit-middleware)
- [Timeout](./04-middleware.md#timeout-middleware)

### Examples
- [Static Website](./06-examples.md#basic-static-website)
- [PHP Application](./06-examples.md#php-application-laravelwordpress)
- [Single Page Application](./06-examples.md#single-page-application-reactvueangular)
- [Virtual Hosting](./06-examples.md#multi-domain-virtual-hosting)
- [API Gateway](./06-examples.md#api-gateway-with-rate-limiting)
- [Microservices](./06-examples.md#microservices-gateway)

## Project Structure

```
bob/
├── Cargo.toml              # Workspace configuration
├── README.md               # Project README
├── example-config.yaml     # Example configuration
├── bob/                    # Main application crate
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       ├── main.rs         # Entry point
│       ├── cli.rs          # CLI handling
│       ├── config/         # Configuration types
│       │   ├── mod.rs
│       │   ├── modules.rs
│       │   └── middleware.rs
│       └── tls/            # TLS configuration
│           ├── mod.rs
│           ├── client.rs
│           └── server.rs
├── bob-cli/                # CLI library crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
└── claudedocs/             # This documentation
    ├── 00-index.md
    ├── 01-overview.md
    ├── 02-architecture.md
    ├── 03-modules.md
    ├── 04-middleware.md
    ├── 05-configuration.md
    └── 06-examples.md
```

## Feature Flags Summary

### Default Features (Enabled)
- `fileserver` - HTTP file serving
- `rproxy` - Reverse proxy
- `fastcgi` - FastCGI client
- `middleware` - All middleware (meta-feature)
  - `authn` - HTTP Basic Authentication
  - `modsecurity` - OWASP ModSecurity WAF
  - `rewrite` - URL rewriting
  - `ipware` - Client IP detection
  - `ipfilter` - IP filtering
  - `ratelimit` - Rate limiting
  - `timeout` - Request timeout

### Optional Features
- `schema` - JSON schema generation
- `doc` - Documentation image handling

## Version Information

- **Rust Edition**: 2024
- **actix-web**: 4.11.0
- **rustls**: 0.23.x
- **clap**: 4.5.x

## Contributing

See the main [README.md](../README.md) for contribution guidelines.

## License

See [LICENSE](../LICENSE) for license information.
