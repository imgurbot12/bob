# Bob Usage Examples

This document provides practical configuration examples for common use cases.

## Quick Start Commands

### Simple File Server

Serve the current directory:

```bash
bob file-server
```

With options:

```bash
bob file-server \
  --root /var/www/html \
  --listen 0.0.0.0:8080 \
  --browse true \
  --index index.html \
  --show-hidden \
  --open
```

### Simple Reverse Proxy

```bash
bob reverse-proxy \
  --from localhost:8080 \
  --to https://api.example.com \
  --timeout 30s \
  --change-host-header
```

With custom headers:

```bash
bob reverse-proxy \
  --from localhost:8080 \
  --to https://api.example.com \
  --header-up "Authorization: Bearer token123" \
  --header-down "X-Proxy: bob"
```

### Simple FastCGI Proxy

```bash
bob fastcgi 127.0.0.1:9000 \
  --root /var/www/html \
  --listen localhost:8080 \
  --index index.php
```

### Password Generation

```bash
# Interactive
bob passwd admin

# Non-interactive
bob passwd admin --password secret123 --output /etc/bob/users.htpasswd
```

---

## Configuration File Examples

### Basic Static Website

Serve a static website with HTTPS:

```yaml
---
- listen:
    - port: 80
    - port: 443
      ssl:
        certificate: /etc/ssl/example.com/fullchain.pem
        certificate_key: /etc/ssl/example.com/privkey.pem

  server_name: [example.com, www.example.com]

  root: /var/www/example.com
  index: [index.html, index.htm]

  directives:
    - location: /
      construct:
        - module: fileserver
          index_files: false
```

### PHP Application (Laravel/WordPress)

```yaml
---
- listen:
    - port: 80

  root: /var/www/html/public
  index: [index.php, index.html]

  directives:
    - location: /
      construct:
        # Try static files first
        - module: fileserver
          next: [404]
        # Fall back to PHP
        - module: fastcgi
          connect: /run/php/php8.2-fpm.sock
```

### Single Page Application (React/Vue/Angular)

```yaml
---
- listen:
    - port: 80

  root: /var/www/app/dist

  directives:
    # API proxy
    - location: /api
      construct:
        - module: rproxy
          resolve: http://localhost:3000
          timeout: 30s

    # Static assets
    - location: /assets
      construct:
        - module: fileserver
          root: /var/www/app/dist/assets

    # SPA fallback - serve index.html for all routes
    - location: /
      construct:
        - module: fileserver
          next: [404]
        - module: static
          body: |
            <!DOCTYPE html>
            <html>
              <head>
                <meta charset="utf-8">
                <title>App</title>
                <script src="/assets/app.js" defer></script>
              </head>
              <body>
                <div id="app"></div>
              </body>
            </html>
          content_type: text/html; charset=utf-8
```

### Multi-Domain Virtual Hosting

```yaml
---
# Main website
- server_name: [example.com, www.example.com]
  listen:
    - port: 80
    - port: 443
      ssl:
        certificate: /etc/ssl/example.com/fullchain.pem
        certificate_key: /etc/ssl/example.com/privkey.pem

  directives:
    - location: /
      construct:
        - module: fileserver
          root: /var/www/example.com

# API subdomain
- server_name: [api.example.com]
  listen:
    - port: 80
    - port: 443
      ssl:
        certificate: /etc/ssl/api.example.com/fullchain.pem
        certificate_key: /etc/ssl/api.example.com/privkey.pem

  directives:
    - location: /
      construct:
        - module: rproxy
          resolve: http://api-backend:8080

# Blog subdomain
- server_name: [blog.example.com]
  listen:
    - port: 80
    - port: 443
      ssl:
        certificate: /etc/ssl/blog.example.com/fullchain.pem
        certificate_key: /etc/ssl/blog.example.com/privkey.pem

  directives:
    - location: /
      construct:
        - module: fastcgi
          connect: /run/php/php-fpm.sock
          root: /var/www/blog
```

### API Gateway with Rate Limiting

```yaml
---
- listen:
    - port: 8080

  middleware:
    # Rate limiting for all endpoints
    - middleware: ratelimit
      limit: 1000
      period: 1m
      response_headers: true

  directives:
    # Public API - higher limits
    - location: /api/public
      construct:
        - module: rproxy
          resolve: http://public-api:3000

    # Authenticated API - stricter limits
    - location: /api/v1
      construct:
        - module: rproxy
          resolve: http://internal-api:3001
        - middleware: ratelimit
          limit: 100
          period: 1m
          use_path: true
        - middleware: basic_auth
          htpasswd: [/etc/bob/api-users.htpasswd]

    # Health check - no limits
    - location: /health
      construct:
        - module: static
          body: '{"status":"ok"}'
          content_type: application/json
```

### Load Balancer (Round Robin via DNS)

```yaml
---
- listen:
    - port: 80

  directives:
    # Backend 1
    - location: /
      construct:
        - module: rproxy
          resolve: http://backend1.internal:8080
          next: [502, 503, 504]
        - module: rproxy
          resolve: http://backend2.internal:8080
          next: [502, 503, 504]
        - module: rproxy
          resolve: http://backend3.internal:8080
```

### Protected Admin Panel

```yaml
---
- listen:
    - port: 443
      ssl:
        certificate: /etc/ssl/admin.example.com/fullchain.pem
        certificate_key: /etc/ssl/admin.example.com/privkey.pem

  server_name: [admin.example.com]

  middleware:
    # Only allow internal IPs
    - middleware: ipware
      trusted_headers: [X-Forwarded-For]
      proxy_count: 1

    - middleware: filter
      allow:
        - "10.0.0.*"
        - "192.168.1.*"

  directives:
    - location: /
      construct:
        - module: fileserver
          root: /var/www/admin
        - middleware: basic_auth_session
          htpasswd: [/etc/bob/admin.htpasswd]
          cookie_name: admin_session
```

### Microservices Gateway

```yaml
---
- listen:
    - port: 80

  middleware:
    - middleware: timeout
      duration: 30000

  directives:
    # User service
    - location: /users
      construct:
        - module: rproxy
          resolve: http://user-service:8001
          upstream_headers:
            X-Service: users

    # Order service
    - location: /orders
      construct:
        - module: rproxy
          resolve: http://order-service:8002
          upstream_headers:
            X-Service: orders

    # Product service
    - location: /products
      construct:
        - module: rproxy
          resolve: http://product-service:8003
          upstream_headers:
            X-Service: products

    # Aggregation endpoint
    - location: /api
      construct:
        - module: rproxy
          resolve: http://api-gateway:8000
```

### Development Server with CORS

```yaml
---
- listen:
    - port: 3000

  sanitize_errors: false

  logging:
    log_level: debug

  directives:
    # API with CORS headers
    - location: /api
      construct:
        - module: rproxy
          resolve: http://localhost:8080
          downstream_headers:
            Access-Control-Allow-Origin: "*"
            Access-Control-Allow-Methods: "GET, POST, PUT, DELETE, OPTIONS"
            Access-Control-Allow-Headers: "Content-Type, Authorization"

    # Static assets
    - location: /
      construct:
        - module: fileserver
          root: ./dist
          hidden_files: true
          index_files: true
```

### WordPress with ModSecurity

```yaml
---
- listen:
    - port: 80

  root: /var/www/wordpress
  index: [index.php, index.html]

  middleware:
    - middleware: modsecurity
      rules: |
        SecRuleEngine On
        SecRequestBodyLimit 10485760
        SecRequestBodyNoFilesLimit 131072
      rule_files:
        - /etc/modsecurity/crs-setup.conf
        - /etc/modsecurity/rules/REQUEST-901-INITIALIZATION.conf
        - /etc/modsecurity/rules/REQUEST-941-APPLICATION-ATTACK-XSS.conf
        - /etc/modsecurity/rules/REQUEST-942-APPLICATION-ATTACK-SQLI.conf

  directives:
    # wp-admin protection
    - location: /wp-admin
      construct:
        - module: fastcgi
          connect: /run/php/php-fpm.sock
        - middleware: filter
          allow: ["10.0.0.*"]

    # Main site
    - location: /
      construct:
        - module: fileserver
          next: [404]
        - module: fastcgi
          connect: /run/php/php-fpm.sock
```

### URL Rewriting (Clean URLs)

```yaml
---
- listen:
    - port: 80

  root: /var/www/html

  middleware:
    - middleware: rewrite
      rules: |
        RewriteEngine On

        # Remove trailing slashes
        RewriteRule ^(.+)/$ /$1 [R=301,L]

        # Clean URLs for PHP
        RewriteCond %{REQUEST_FILENAME} !-f
        RewriteCond %{REQUEST_FILENAME} !-d
        RewriteRule ^(.*)$ /index.php?route=$1 [L,QSA]

  directives:
    - location: /
      construct:
        - module: fileserver
          next: [404]
        - module: fastcgi
          connect: /run/php/php-fpm.sock
```

### Maintenance Mode Toggle

```yaml
---
- listen:
    - port: 80

  # Maintenance mode - uncomment to enable
  # directives:
  #   - location: /
  #     construct:
  #       - module: static
  #         body: |
  #           <html>
  #             <head><title>Maintenance</title></head>
  #             <body>
  #               <h1>Under Maintenance</h1>
  #               <p>We'll be back shortly.</p>
  #             </body>
  #           </html>
  #         status_code: 503
  #         headers:
  #           Retry-After: "3600"

  # Normal mode
  directives:
    - location: /
      construct:
        - module: rproxy
          resolve: http://backend:8080
```

### Caching Proxy Headers

```yaml
---
- listen:
    - port: 80

  directives:
    # Static assets with long cache
    - location: /static
      construct:
        - module: fileserver
          root: /var/www/static
        - middleware: timeout
          duration: 5000

    # Dynamic content with no cache
    - location: /api
      construct:
        - module: rproxy
          resolve: http://api:8080
          downstream_headers:
            Cache-Control: "no-cache, no-store, must-revalidate"
            Pragma: "no-cache"
            Expires: "0"

    # Default with moderate cache
    - location: /
      construct:
        - module: fileserver
          root: /var/www/html
```

---

## CLI Command Reference

### bob run

```bash
bob run [OPTIONS]

Options:
  -c, --config <PATH>   Configuration file path [default: ./config.yaml]
  -s, --sanitize        Override sanitize_errors setting
  -l, --log <BOOL>      Override logging enabled [default: true]
```

### bob file-server

```bash
bob file-server [OPTIONS]

Options:
  -b, --browse <BOOL>      Enable directory browsing [default: true]
  -i, --index <FILES>      Index files [default: index.html]
  -l, --listen <ADDR>      Listen address [default: localhost:8000]
  -r, --root <PATH>        Root directory [default: .]
  -s, --show-hidden        Show hidden files
      --open               Open in browser
```

### bob reverse-proxy

```bash
bob reverse-proxy [OPTIONS] --to <URI>

Options:
  -c, --change-host-header   Set Host header to upstream
  -f, --from <ADDR>          Listen address [default: localhost:8000]
      --insecure             Disable TLS verification
  -t, --to <URI>             Upstream URI
      --timeout <DURATION>   Request timeout [default: 5s]
  -d, --header-down <H:V>    Response header (repeatable)
  -u, --header-up <H:V>      Request header (repeatable)
      --open                 Open in browser
```

### bob fastcgi

```bash
bob fastcgi <CONNECT> [OPTIONS]

Arguments:
  <CONNECT>   FastCGI address (host:port or socket path)

Options:
  -i, --index <FILES>    Index files [default: index.php]
  -l, --listen <ADDR>    Listen address [default: localhost:8000]
  -r, --root <PATH>      Document root [default: .]
```

### bob passwd

```bash
bob passwd <USERNAME> [OPTIONS]

Arguments:
  <USERNAME>   Username for the password entry

Options:
  -p, --password <PASS>   Password (prompts if not provided)
  -o, --output <FILE>     Output file (stdout if not provided)
```

### bob schema

```bash
bob schema [OPTIONS]

Options:
  -o, --output <FILE>   Output file [default: schema.json]
```

---

## Environment Configuration

### Log Levels

```bash
# Default logging
bob run

# Debug logging
BOB_LOG=debug bob run

# Per-module logging
BOB_LOG=info,bob=debug,actix_web=warn bob run

# Trace all HTTP activity
BOB_LOG=trace bob run
```

### Low Port Binding (Linux)

```bash
# Grant capability to bind low ports
sudo setcap cap_net_bind_service=+ep $(which bob)

# Then run normally
bob run
```

---

## Systemd Service Example

```ini
[Unit]
Description=Bob Web Server
After=network.target

[Service]
Type=simple
User=www-data
Group=www-data
WorkingDirectory=/etc/bob
ExecStart=/usr/local/bin/bob run --config /etc/bob/config.yaml
Restart=always
RestartSec=5
Environment=BOB_LOG=info

[Install]
WantedBy=multi-user.target
```

Install and enable:

```bash
sudo cp bob.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable bob
sudo systemctl start bob
```

---

## Docker Compose Example

```yaml
version: '3.8'

services:
  bob:
    image: rust:latest
    command: /app/bob run --config /etc/bob/config.yaml
    volumes:
      - ./bob:/app/bob:ro
      - ./config.yaml:/etc/bob/config.yaml:ro
      - ./www:/var/www:ro
      - ./ssl:/etc/ssl:ro
    ports:
      - "80:80"
      - "443:443"
    environment:
      - BOB_LOG=info
    restart: unless-stopped

  php-fpm:
    image: php:8.2-fpm
    volumes:
      - ./www:/var/www
    expose:
      - "9000"

  api:
    image: node:20
    working_dir: /app
    command: npm start
    volumes:
      - ./api:/app
    expose:
      - "3000"
```

With Bob configuration:

```yaml
---
- listen:
    - port: 80

  directives:
    - location: /api
      construct:
        - module: rproxy
          resolve: http://api:3000

    - location: /
      construct:
        - module: fileserver
          next: [404]
        - module: fastcgi
          connect: php-fpm:9000
          root: /var/www
```
