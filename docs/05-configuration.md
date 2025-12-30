# Bob Configuration Guide

This guide covers the complete configuration schema for Bob, including all available options, their types, and default values.

## Configuration File Format

Bob uses YAML configuration files. The root of the configuration is an array of server configurations:

```yaml
---
# Server 1
- listen:
    - port: 80
  directives:
    - location: /
      construct:
        - module: fileserver

# Server 2
- listen:
    - port: 443
      ssl:
        certificate: /etc/ssl/cert.pem
        certificate_key: /etc/ssl/key.pem
  directives:
    - location: /
      construct:
        - module: rproxy
          resolve: http://backend:8080
```

## Configuration File Location

By default, Bob looks for `./config.yaml`. Override with:

```bash
bob run --config /path/to/config.yaml
```

---

## Server Configuration (`ServerConfig`)

The top-level configuration for each virtual server.

```yaml
- # Server configuration fields
  disable: false
  listen: []
  logging: {}
  server_name: []
  middleware: []
  directives: []
  root: /var/www/html
  index: [index.html]
  body_buffer_size: 65536
  sanitize_errors: true
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `disable` | `bool` | No | `false` | Temporarily disable this server |
| `listen` | `list<ListenCfg>` | No | `[]` | Listener bindings |
| `logging` | `LoggingCfg` | No | `{}` | Logging configuration |
| `server_name` | `list<string>` | No | `[]` | Domain name patterns (glob) |
| `middleware` | `list<Middleware>` | No | `[]` | Server-wide middleware |
| `directives` | `list<DirectiveCfg>` | No | `[]` | Request handlers |
| `root` | `path` | No | `.` | Default document root |
| `index` | `list<string>` | No | `[]` | Index file patterns |
| `body_buffer_size` | `usize` | No | - | Max body buffer size |
| `sanitize_errors` | `bool` | No | `true` | Hide detailed errors |

---

## Listener Configuration (`ListenCfg`)

Defines how Bob binds to network addresses.

```yaml
listen:
  - port: 80
    host: 0.0.0.0

  - port: 443
    host: 0.0.0.0
    ssl:
      certificate: /etc/ssl/server.crt
      certificate_key: /etc/ssl/server.key
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `port` | `u16` | Yes | - | Port number to bind |
| `host` | `string` | No | `0.0.0.0` | Host address to bind |
| `ssl` | `SSLCfg` | No | - | TLS configuration |

### SSL Configuration (`SSLCfg`)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `certificate` | `path` | Yes | Path to PEM certificate file |
| `certificate_key` | `path` | Yes | Path to PEM private key file |

### Examples

**HTTP Only:**
```yaml
listen:
  - port: 8080
```

**HTTPS Only:**
```yaml
listen:
  - port: 443
    ssl:
      certificate: /etc/ssl/fullchain.pem
      certificate_key: /etc/ssl/privkey.pem
```

**HTTP + HTTPS:**
```yaml
listen:
  - port: 80
  - port: 443
    ssl:
      certificate: /etc/ssl/fullchain.pem
      certificate_key: /etc/ssl/privkey.pem
```

**Multiple IPs:**
```yaml
listen:
  - port: 80
    host: 192.168.1.10
  - port: 80
    host: 192.168.1.20
```

---

## Logging Configuration (`LoggingCfg`)

Controls request logging behavior.

```yaml
logging:
  disable: false
  log_level: info
  use_ipware: true
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `disable` | `bool` | No | `false` | Disable request logging |
| `log_level` | `string` | No | `info` | Log level for requests |
| `use_ipware` | `bool` | No | `true` | Use IpWare resolved IP in logs |

### Log Levels

- `trace` - Most verbose
- `debug` - Debug information
- `info` - Standard request logging
- `warn` - Warnings only
- `error` - Errors only

### Log Format

**Default (without IpWare):**
```
{peer_addr} "{method} {uri} {version}" {status} {size} "{referer}" "{user_agent}" {duration}
```

**With IpWare:**
```
{resolved_ip} "{method} {uri} {version}" {status} {size} "{referer}" "{user_agent}" {duration}
```

---

## Domain Matching (`server_name`)

The `server_name` field specifies which Host headers this server responds to.

```yaml
server_name:
  - example.com
  - "*.example.com"
  - "api.example.*"
```

### Pattern Syntax

Uses glob patterns:
- `*` - Matches any characters
- `?` - Matches single character
- `[abc]` - Matches one of the characters

### Examples

| Pattern | Matches |
|---------|---------|
| `example.com` | Exact match |
| `*.example.com` | `www.example.com`, `api.example.com` |
| `*.*.example.com` | `api.v1.example.com` |
| `example.*` | `example.com`, `example.org` |

### SNI (Server Name Indication)

When using TLS, Bob uses SNI to select the appropriate certificate:

```yaml
# Server for example.com
- server_name: [example.com, "*.example.com"]
  listen:
    - port: 443
      ssl:
        certificate: /etc/ssl/example.com/fullchain.pem
        certificate_key: /etc/ssl/example.com/privkey.pem
  directives:
    - location: /
      construct:
        - module: fileserver
          root: /var/www/example.com

# Server for other.com
- server_name: [other.com]
  listen:
    - port: 443
      ssl:
        certificate: /etc/ssl/other.com/fullchain.pem
        certificate_key: /etc/ssl/other.com/privkey.pem
  directives:
    - location: /
      construct:
        - module: fileserver
          root: /var/www/other.com
```

---

## Directive Configuration (`DirectiveCfg`)

Directives map URL paths to request handlers.

```yaml
directives:
  - location: /api
    construct:
      - module: rproxy
        resolve: http://backend:8080

  - location: /static
    construct:
      - module: fileserver
        root: /var/www/static

  - location: /
    construct:
      - module: fileserver
        root: /var/www/html
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `location` | `string` | No | `/` | URL path prefix |
| `construct` | `list<Component>` | Yes | - | Modules and middleware |

### Location Matching

- Locations are matched as prefixes
- More specific locations are matched first
- Leading `/` is optional (normalized internally)

### Construct Components

The `construct` field contains an ordered list of modules and middleware:

```yaml
construct:
  # First item MUST be a module
  - module: fileserver
    root: /var/www
    next: [404]

  # Subsequent items can be modules or middleware
  - module: rproxy
    resolve: http://backend

  # Middleware wraps everything before it
  - middleware: ratelimit
    limit: 100
    period: 1m
```

**Rules:**
1. First item must be a module
2. Module `next` field chains to subsequent modules
3. Middleware wraps modules/middleware defined before it

---

## Index Files

The `index` field specifies files to serve when a directory is requested:

```yaml
index:
  - index.html
  - index.htm
  - index.php
```

### Resolution Order

1. Check for `index.html`
2. Check for `index.htm`
3. Check for `index.php`
4. If no match and directory listing enabled, show listing
5. Otherwise return 404

---

## Error Sanitization

The `sanitize_errors` field controls error message visibility:

```yaml
sanitize_errors: true
```

| Value | Behavior |
|-------|----------|
| `true` (default) | Generic error messages (safe for production) |
| `false` | Detailed error messages (useful for debugging) |

---

## Complete Configuration Example

```yaml
---
# Production server with full configuration
- disable: false

  listen:
    - port: 80
      host: 0.0.0.0
    - port: 443
      host: 0.0.0.0
      ssl:
        certificate: /etc/ssl/example.com/fullchain.pem
        certificate_key: /etc/ssl/example.com/privkey.pem

  logging:
    disable: false
    log_level: info
    use_ipware: true

  server_name:
    - example.com
    - "*.example.com"

  root: /var/www/html
  index:
    - index.html
    - index.htm

  sanitize_errors: true
  body_buffer_size: 1048576

  middleware:
    # Real IP detection from proxy headers
    - middleware: ipware
      trusted_headers:
        - X-Forwarded-For
        - CF-Connecting-IP
      proxy_count: 1

    # Rate limiting
    - middleware: ratelimit
      limit: 1000
      period: 1m
      response_headers: true

    # Security WAF
    - middleware: modsecurity
      rules: |
        SecRuleEngine On
        SecRule REQUEST_URI "@rx /admin" "id:1,phase:1,deny,status:403"

    # Request timeout
    - middleware: timeout
      duration: 60000

  directives:
    # API proxy
    - location: /api
      construct:
        - module: rproxy
          resolve: http://api-backend:3000
          timeout: 30s
          change_host: true

    # Admin area with extra protection
    - location: /admin
      construct:
        - module: fileserver
          root: /var/www/admin
        - middleware: basic_auth
          htpasswd:
            - /etc/bob/admin.htpasswd
        - middleware: filter
          allow:
            - "10.0.0.*"

    # PHP application
    - location: /app
      construct:
        - module: fastcgi
          connect: /run/php/php-fpm.sock
          root: /var/www/app

    # Static files with fallback
    - location: /
      construct:
        - module: fileserver
          root: /var/www/html
          next: [404]
        - module: static
          body: "<h1>Not Found</h1>"
          status_code: 404
```

---

## Duration Format

Duration fields accept human-readable formats:

| Format | Example |
|--------|---------|
| Seconds | `5s`, `30s` |
| Minutes | `1m`, `5m` |
| Hours | `1h`, `24h` |
| Combined | `1h30m`, `2m30s` |
| Milliseconds | `500ms` |

### Examples

```yaml
timeout: 30s
period: 1m
session_ttl: 24h
connect_timeout: 5s
```

---

## URI Format

URI fields accept standard HTTP URIs:

```yaml
resolve: http://localhost:8080
resolve: https://api.example.com:8443
resolve: http://unix:/var/run/app.sock
```

---

## Path Format

Path fields accept absolute or relative filesystem paths:

```yaml
# Absolute path
root: /var/www/html

# Relative to working directory
root: ./public

# Home directory (shell expansion not supported)
root: /home/user/www
```

---

## Environment Variables

Bob respects the following environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `BOB_LOG` | Log filter configuration | `info`, `bob=debug`, `actix_web=warn` |

### Log Filter Syntax

```bash
# Set global level
export BOB_LOG=info

# Set per-module level
export BOB_LOG=info,bob=debug,actix_web::middleware=trace

# Disable specific module
export BOB_LOG=info,hyper=off
```

---

## Validation and Errors

Bob validates configuration at startup:

**Common Errors:**

| Error | Cause |
|-------|-------|
| `config is empty` | No server configurations defined |
| `invalid config` | YAML syntax error or unknown field |
| `failed to read tls certificate` | SSL certificate file not found |
| `invalid private tls key` | SSL key file invalid or wrong format |
| `must contain a module` | Directive construct has no modules |
| `first component must be a module` | Directive doesn't start with a module |

**Strict Parsing:**
- Unknown fields cause errors (`deny_unknown_fields`)
- Type mismatches cause errors
- Required fields must be present
