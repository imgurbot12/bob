# Bob Modules Reference

Modules are the primary request handlers in Bob. They process incoming HTTP requests and produce responses. Modules can be chained together using the `next` configuration to create fallback patterns.

## Module System Overview

### Configuration Structure

Each module is configured within a directive's `construct` array:

```yaml
directives:
  - location: /api
    construct:
      - module: <module_name>
        # module-specific configuration
        next: [404, 405]  # optional: status codes to pass to next module
```

### Module Chaining

The `next` field specifies which HTTP status codes should trigger the next module in the chain:

```yaml
construct:
  - module: fileserver
    next: [404, 405]
  - module: rproxy
    resolve: https://backend.example.com
```

**Default behavior**: If `next` is not specified, the module's response is final.

---

## Redirect Module

Performs simple HTTP redirects to a specified URL.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `redirect` | `string` | Yes | - | Target URI for redirection |
| `status_code` | `u16` | No | 302 | HTTP redirect status code |

### Example

```yaml
directives:
  - location: /old-path
    construct:
      - module: redirect
        redirect: /new-path
        status_code: 301
```

### Implementation Details

**Source**: `config/modules.rs::redirect`

- Creates an `actix_web::Route` that responds with a redirect
- Sets the `Location` header to the configured URI
- Uses `302 Found` by default for temporary redirects

**Supported Status Codes:**
- `301` - Moved Permanently
- `302` - Found (Temporary Redirect)
- `303` - See Other
- `307` - Temporary Redirect
- `308` - Permanent Redirect

---

## Static Module

Returns a static HTTP response with configurable content, headers, and status code.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `body` | `string` | No | `""` | Response body content |
| `content_type` | `string` | No | `text/html; charset=UTF-8` | Content-Type header value |
| `headers` | `map<string, string>` | No | `{}` | Additional response headers |
| `status_code` | `u16` | No | 200 | HTTP status code |

### Example

```yaml
directives:
  - location: /health
    construct:
      - module: static
        body: '{"status": "ok"}'
        content_type: application/json
        status_code: 200
        headers:
          X-Health-Check: "true"
```

### Implementation Details

**Source**: `config/modules.rs::rstatic`

- Creates an async route handler that returns the configured body
- All headers and content are cloned for each request
- Useful for health checks, stub endpoints, and maintenance pages

---

## FileServer Module

**Feature Flag**: `fileserver`

Serves static files from a filesystem directory.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `root` | `path` | No | Server's `root` or `.` | Root directory for file serving |
| `hidden_files` | `bool` | No | `false` | Allow serving dotfiles (e.g., `.htaccess`) |
| `index_files` | `bool` | No | `false` | Enable directory listing/browsing |
| `async_threshold` | `u64` | No | `65535` | File size threshold for async I/O (bytes) |

### Example

```yaml
root: /var/www/html
index: [index.html, index.htm]

directives:
  - location: /
    construct:
      - module: fileserver
        hidden_files: false
        index_files: true
        async_threshold: 65536
```

### Implementation Details

**Source**: `config/modules.rs::fileserver`

**Underlying Service**: `actix_files::Files`

**Behavior:**
- Serves files from the configured root directory
- Uses the server-level `index` setting for index file resolution
- Optionally enables directory browsing when `index_files` is true
- Hidden files (starting with `.`) are blocked by default
- Files above `async_threshold` are read asynchronously

**Index File Resolution:**
1. Request for `/path/` checks for configured index files
2. Falls back to directory listing if enabled
3. Returns 404 if no index found and listing disabled

---

## ReverseProxy Module

**Feature Flag**: `rproxy`

Proxies requests to an upstream HTTP server.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `resolve` | `uri` | Yes | - | Upstream server URL |
| `change_host` | `bool` | No | `false` | Set Host header to upstream address |
| `max_redirects` | `u8` | No | `0` | Maximum redirects to follow |
| `initial_conn_size` | `u32` | No | `65535` | Initial connection window size |
| `initial_window_size` | `u32` | No | `65535` | Initial stream window size |
| `timeout` | `duration` | No | `5s` | Request timeout |
| `verify_ssl` | `bool` | No | `true` | Verify upstream TLS certificates |
| `upstream_headers` | `map<string, string>` | No | `{}` | Headers to add to upstream requests |
| `downstream_headers` | `map<string, string>` | No | `{}` | Headers to add to downstream responses |

### Example

```yaml
directives:
  - location: /api
    construct:
      - module: rproxy
        resolve: https://api.internal.example.com:8443
        change_host: true
        timeout: 30s
        verify_ssl: true
        upstream_headers:
          X-Forwarded-Host: "example.com"
          X-Real-IP: "${CLIENT_IP}"
        downstream_headers:
          X-Proxy: "bob"
```

### Implementation Details

**Source**: `config/modules.rs::rproxy`

**Underlying Service**: `actix_revproxy::RevProxy`

**HTTP Client Configuration:**
- Uses `awc::Client` with configurable TLS
- ALPN protocols: `h2`, `http/1.1`
- No default headers (clean proxy)
- Connection pooling via `awc::Connector`

**TLS Handling:**
- When `verify_ssl: false`, uses `NoCertificateVerification` (dangerous)
- When `verify_ssl: true` (default), uses WebPKI roots

**Header Manipulation:**
- `upstream_headers`: Added to every request sent upstream
- `downstream_headers`: Added to every response sent to client
- `change_host: true`: Replaces Host header with upstream hostname

---

## FastCGI Module

**Feature Flag**: `fastcgi`

Connects to FastCGI backends (e.g., PHP-FPM).

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `connect` | `string` | Yes | - | FastCGI server address (host:port or socket path) |
| `root` | `path` | No | Server's `root` or `.` | Document root for SCRIPT_FILENAME |

### Example

```yaml
root: /var/www/html
index: [index.php, index.html]

directives:
  - location: /
    construct:
      - module: fastcgi
        connect: 127.0.0.1:9000
        root: /var/www/html
```

### Unix Socket Example

```yaml
directives:
  - location: /
    construct:
      - module: fastcgi
        connect: /run/php/php8.2-fpm.sock
        root: /var/www/html
```

### Implementation Details

**Source**: `config/modules.rs::fastcgi`

**Underlying Service**: `actix_fastcgi::FastCGI`

**FastCGI Parameters Set:**
- `SCRIPT_FILENAME`: Resolved from root + request path
- `DOCUMENT_ROOT`: Configured root directory
- Index files from server configuration

**Connection Types:**
- TCP: `host:port` format
- Unix Socket: `/path/to/socket` format

---

## Module Chaining Examples

### Fileserver with Proxy Fallback

Serve static files, fall back to reverse proxy for dynamic content:

```yaml
directives:
  - location: /
    construct:
      - module: fileserver
        root: /var/www/static
        next: [404]
      - module: rproxy
        resolve: http://localhost:3000
```

### Multi-tier Fallback

```yaml
directives:
  - location: /
    construct:
      # Try static files first
      - module: fileserver
        root: /var/www/html
        next: [404, 405]
      # Then try PHP files
      - module: fastcgi
        connect: /run/php/php-fpm.sock
        next: [404]
      # Finally proxy to backend
      - module: rproxy
        resolve: http://backend:8080
```

### Path-specific Handlers

```yaml
directives:
  # API routes to backend
  - location: /api
    construct:
      - module: rproxy
        resolve: http://api-server:3000

  # Static assets
  - location: /static
    construct:
      - module: fileserver
        root: /var/www/static

  # Catch-all for SPA
  - location: /
    construct:
      - module: static
        body: |
          <!DOCTYPE html>
          <html>
            <head><title>App</title></head>
            <body><div id="app"></div></body>
          </html>
        content_type: text/html
```

---

## Common Patterns

### Health Check Endpoint

```yaml
directives:
  - location: /health
    construct:
      - module: static
        body: "OK"
        status_code: 200

  - location: /
    construct:
      - module: rproxy
        resolve: http://backend:8080
```

### Maintenance Mode

```yaml
directives:
  - location: /
    construct:
      - module: static
        body: |
          <html>
            <body>
              <h1>Maintenance in Progress</h1>
              <p>Please check back soon.</p>
            </body>
          </html>
        status_code: 503
        headers:
          Retry-After: "3600"
```

### Legacy URL Redirects

```yaml
directives:
  - location: /old-blog
    construct:
      - module: redirect
        redirect: /blog
        status_code: 301

  - location: /legacy-api
    construct:
      - module: redirect
        redirect: /api/v2
        status_code: 308
```
