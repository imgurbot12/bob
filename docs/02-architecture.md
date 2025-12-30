# Bob Architecture

## System Architecture Overview

Bob follows a layered architecture pattern built on top of the actix-web framework:

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Layer                                │
│              (bob-cli crate + cli.rs)                           │
│         Command parsing, config generation                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Configuration Layer                           │
│              (config/mod.rs, modules.rs, middleware.rs)         │
│         YAML parsing, type validation, defaults                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Chain Assembly                               │
│              (main.rs::assemble_chain)                          │
│         Middleware wrapping, module linking                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    actix-web Server                              │
│              HTTP/1.1, HTTP/2, TLS/SNI                          │
│         io_uring (experimental), connection handling             │
└─────────────────────────────────────────────────────────────────┘
```

## Core Concepts

### 1. Server Configuration (`ServerConfig`)

The root configuration unit representing a single virtual server. Key fields:

| Field | Type | Purpose |
|-------|------|---------|
| `disable` | `bool` | Temporarily disable the configuration |
| `listen` | `Vec<ListenCfg>` | Binding addresses and TLS settings |
| `logging` | `LoggingCfg` | Request logging configuration |
| `server_name` | `Vec<DomainMatch>` | Domain matching patterns (SNI) |
| `middleware` | `Vec<Middleware>` | Server-wide middleware stack |
| `directives` | `Vec<DirectiveCfg>` | Location-based request handlers |
| `root` | `Option<PathBuf>` | Default document root |
| `index` | `Vec<String>` | Index file patterns |
| `sanitize_errors` | `Option<bool>` | Hide detailed error messages |

### 2. Request Chain (`actix_chain::Chain`)

Bob uses the `actix-chain` crate to create composable request processing pipelines:

```
Request → [Middleware Stack] → [Module Chain] → Response
              ↑                      ↑
           (wrap)                 (link)
```

**Chain Operations:**
- `chain.guard(domain)`: Add domain matching guard
- `chain.push_link(link)`: Add a request handler link
- `chain.wrap(middleware)`: Wrap chain in middleware

### 3. Modules vs Middleware

| Aspect | Modules | Middleware |
|--------|---------|------------|
| **Purpose** | Handle requests, produce responses | Transform requests/responses |
| **Position** | Terminal in chain | Wrapping handlers |
| **Examples** | FileServer, ReverseProxy, FastCGI | Auth, RateLimit, ModSecurity |
| **Chaining** | Via `next` status codes | Via `wrap_with()` |

### 4. Module Chaining with `next`

Modules can be chained together based on HTTP status codes:

```yaml
construct:
  - module: fileserver
    next: [404, 405]      # Pass to next module on these codes
  - module: rproxy
    resolve: https://backend.example.com
```

**Flow:**
1. FileServer attempts to serve the file
2. On 404 (Not Found) or 405 (Method Not Allowed), control passes to ReverseProxy
3. ReverseProxy handles the request

## Module Architecture

### Module Configuration Enum (`ModuleConfig`)

```rust
pub enum ModuleConfig {
    Redirect(redirect::Config),
    Static(rstatic::Config),
    FileServer(fileserver::Config),  // feature: fileserver
    ReverseProxy(rproxy::Config),    // feature: rproxy
    FastCGI(fastcgi::Config),        // feature: fastcgi
}
```

Each variant:
1. Holds its specific configuration
2. Implements `link(&self, spec: &Spec) -> Link` to produce an actix-chain Link
3. Uses the `Spec` context for shared configuration (root, index files, etc.)

### Module Trait Pattern

```rust
impl Config {
    // Create the underlying actix service
    pub fn factory(&self, spec: &Spec) -> ServiceType { ... }

    // Convert to actix-chain Link
    pub fn link(&self, spec: &Spec) -> Link {
        Link::new(self.factory(spec))
    }
}
```

## Middleware Architecture

### Middleware Configuration Enum

```rust
pub enum Middleware {
    AuthBasic(auth_basic::Config),      // feature: authn
    AuthSession(auth_session::Config),  // feature: authn
    Ipware(ipware::Config),             // feature: ipware
    Ipfilter(ipfilter::Config),         // feature: ipfilter
    ModSecurity(modsecurity::Config),   // feature: modsecurity
    Rewrite(rewrite::Config),           // feature: rewrite
    Ratelimit(ratelimit::Config),       // feature: ratelimit
    Timeout(timeout::Config),           // feature: timeout
}
```

### Middleware Wrap Pattern

```rust
impl Config {
    // Create the underlying actix middleware
    pub fn factory(&self, spec: &Spec) -> MiddlewareType { ... }

    // Wrap any Wrappable (Chain or Link)
    pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
        w.wrap_with(self.factory(spec))
    }
}
```

## TLS Architecture

### Server-Side TLS (SNI Resolution)

```rust
pub struct TlsResolver(Vec<TlsEntry>);

impl ResolvesServerCert for TlsResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        // Match server_name patterns to find appropriate certificate
    }
}
```

**SNI Flow:**
1. Client sends TLS ClientHello with desired hostname
2. `TlsResolver` searches `TlsEntry` list for matching domain pattern
3. Returns the appropriate `CertifiedKey` (certificate + private key)
4. If no match, first entry is used as default

### Client-Side TLS (Reverse Proxy)

```rust
pub fn build_tls_config(verify_ssl: bool) -> rustls::ClientConfig {
    // Configure ALPN protocols (h2, http/1.1)
    // Optionally disable certificate verification (insecure)
}
```

## Request Flow

```
1. HTTP Request arrives
        │
2. actix-web routing
        │
3. Server Name matching (DomainMatch guards)
        │
4. Server-wide middleware (bottom-up wrap order)
   │ └── Logger
   │ └── Sanitizer (error sanitization)
   │ └── User-defined middleware (ModSecurity, etc.)
        │
5. Directive matching (location prefix)
        │
6. Directive middleware (per-location)
        │
7. Module chain execution
   │ └── First module processes request
   │ └── On `next` status codes → next module
   └── Final response
        │
8. Response flows back through middleware
        │
9. HTTP Response sent
```

## Configuration Parsing

### YAML Deserialization Flow

```
YAML File
    │
    ▼ serde_yaml::from_str
    │
Vec<ServerConfig>
    │
    ├── ListenCfg (host, port, ssl)
    ├── LoggingCfg (level, disable, ipware)
    ├── DomainMatch (glob patterns)
    ├── Middleware (tagged enum)
    └── DirectiveCfg
            └── Components (modules + middleware mix)
```

### Component Discrimination

The `Component` type distinguishes between modules and middleware in directive constructs:

```rust
impl<'de> Deserialize<'de> for Component {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        // Check for "module" key to discriminate
        match value.get("module").is_some() {
            true => Component::Module(...),
            false => Component::Middleware(...),
        }
    }
}
```

## Crate Organization

### `bob` Crate (Main Application)

| File | Purpose |
|------|---------|
| `main.rs` | Entry point, server assembly, logging setup |
| `cli.rs` | CLI command handling, config generation |
| `config/mod.rs` | Core configuration types |
| `config/modules.rs` | Request module configurations |
| `config/middleware.rs` | Middleware configurations |
| `tls/mod.rs` | TLS module organization |
| `tls/client.rs` | Client TLS configuration (for reverse proxy) |
| `tls/server.rs` | Server TLS with SNI resolution |

### `bob-cli` Crate (Shared Library)

| Export | Purpose |
|--------|---------|
| `Cli` | Root CLI parser struct |
| `Command` | Subcommand enum |
| `RunCmd`, `FileServerCmd`, etc. | Command argument structs |
| `Duration` | Human-readable duration parser |
| `Uri` | HTTP URI wrapper |
| `Header` | Header key-value pair parser |
| `de_fromstr!` | Macro for FromStr-based deserialization |

## Build System

### Build Script (`build.rs`)

**Features:**
- `schema`: Generates man pages using `clap_mangen`
- `doc`: Copies logo images to documentation output

```rust
#[cfg(feature = "schema")]
fn build_mangen() -> std::io::Result<()> {
    // Generate bob.1 and bob-{subcommand}.1 man pages
}

#[cfg(feature = "doc")]
fn add_docs_imgs() {
    // Copy logo to target/doc/img/
}
```

## Threading Model

Bob uses actix-web's default multi-worker model:

- **Workers**: One per CPU core (by default)
- **Shared State**: Configuration is cloned per worker
- **Thread Safety**: Middleware backends (ratelimit, session keys) are initialized once and shared

```rust
// Configuration is cloned for each worker
let sconfig = config.clone();
let mut server = HttpServer::new(move || {
    sconfig.iter()
        .map(assemble_chain)
        .fold(App::new(), |app, cfg| app.service(cfg))
});
```

## Performance Considerations

1. **io_uring Support**: Enabled via `experimental-io-uring` feature on actix-web
2. **Release Profile**: Optimized with `lto = true`, `codegen-units = 1`, `panic = "abort"`
3. **Async File I/O**: FileServer supports configurable async threshold
4. **Connection Pooling**: ReverseProxy uses connection pooling via `awc::Client`
5. **Memory Efficiency**: Middleware backends are shared across workers
