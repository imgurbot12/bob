# Bob Middleware Reference

Middleware in Bob wraps request handlers to provide cross-cutting concerns like authentication, rate limiting, security filtering, and request transformation. Middleware can be applied at the server level (affecting all directives) or within a specific directive's construct chain.

## Middleware System Overview

### Configuration Locations

**Server-wide Middleware:**
```yaml
middleware:
  - middleware: modsecurity
    rules: "SecRuleEngine On"

directives:
  - location: /
    construct:
      - module: fileserver
```

**Directive-level Middleware:**
```yaml
directives:
  - location: /admin
    construct:
      - module: fileserver
      - middleware: basic_auth
        htpasswd: [./admin.htpasswd]
```

### Middleware Order

Middleware wraps handlers from outside-in:
1. Server-wide middleware wraps the entire directive chain
2. Within a directive, middleware wraps modules/middleware defined before it
3. Later middleware in the list wraps earlier entries

```yaml
construct:
  - module: fileserver           # innermost
  - middleware: ratelimit        # wraps fileserver
  - middleware: basic_auth       # wraps ratelimit + fileserver (outermost)
```

---

## AuthBasic Middleware

**Feature Flag**: `authn`

HTTP Basic Authentication using htpasswd-format password files.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `htpasswd` | `list<path>` | No | `[]` | List of htpasswd file paths |
| `cache_size` | `usize` | No | `65535` | Authentication cache size |

### Example

```yaml
middleware:
  - middleware: basic_auth
    htpasswd:
      - /etc/bob/users.htpasswd
      - /etc/bob/admin.htpasswd
    cache_size: 1000
```

### Creating Password Files

Use the built-in `passwd` command:

```bash
# Interactive password prompt
bob passwd admin

# Direct password (less secure)
bob passwd admin --password secret123

# Output to file
bob passwd admin --output /etc/bob/users.htpasswd
```

**htpasswd Format:**
```
username:$2b$12$bcrypt-hashed-password
```

### Implementation Details

**Source**: `config/middleware.rs::auth_basic`

**Underlying Service**: `actix_authn::Authn<BasicAuth>`

**Behavior:**
- Prompts for credentials via WWW-Authenticate header
- Validates against bcrypt-hashed passwords
- Caches successful authentications for performance
- Multiple htpasswd files are merged

---

## AuthSession Middleware

**Feature Flag**: `authn`

HTTP Basic Authentication with cookie-based session persistence.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `htpasswd` | `list<path>` | No | `[]` | List of htpasswd file paths |
| `cookie_name` | `string` | No | `authn` | Session cookie name |
| `cache_size` | `usize` | No | `65535` | Authentication cache size |

### Example

```yaml
middleware:
  - middleware: basic_auth_session
    htpasswd:
      - /etc/bob/users.htpasswd
    cookie_name: bob_session
    cache_size: 500
```

### Implementation Details

**Source**: `config/middleware.rs::auth_session`

**Underlying Services:**
- `actix_authn::Authn<BasicAuthSession>`
- `actix_session::SessionMiddleware`
- `actix_session::storage::CookieSessionStore`

**Session Behavior:**
- Browser session lifecycle (expires when browser closes)
- Session state TTL: 24 hours
- Cookie key is auto-generated per server instance
- Secure cookies with cryptographic signing

**Security Notes:**
- Cookie key is generated at configuration load time
- Key is shared across all workers for the same config
- Restarting the server invalidates all sessions

---

## IpWare Middleware

**Feature Flag**: `ipware`

Determines the real client IP address from proxy headers.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `strict` | `bool` | No | `true` | Reject malformed IPs in trusted headers |
| `trusted_headers` | `list<string>` | No | `[]` | Headers to trust for client IP |
| `proxy_count` | `u16` | No | - | Expected number of trusted proxies |
| `trusted_proxies` | `list<string>` | No | `[]` | Trusted proxy IP patterns (glob) |
| `allow_untrusted` | `bool` | No | `false` | Allow untrusted IP assignments |

### Example

```yaml
middleware:
  - middleware: ipware
    strict: true
    trusted_headers:
      - X-Forwarded-For
      - X-Real-IP
    proxy_count: 2
    trusted_proxies:
      - "10.0.0.*"
      - "192.168.1.*"
```

### Implementation Details

**Source**: `config/middleware.rs::ipware`

**Underlying Service**: `actix_ipware::Middleware`

**IP Resolution Priority:**
1. Check trusted headers for client IP
2. Apply proxy count to find the real client (skip N proxies)
3. Verify against trusted proxy patterns
4. Fall back to connection peer address

**Logging Integration:**
When `logging.use_ipware: true` (default), the resolved IP is used in access logs.

---

## IpFilter Middleware

**Feature Flag**: `ipfilter`

IP-based access control with whitelist and blacklist support.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `whitelist` / `allow` | `list<string>` | No | `[]` | Always-allowed IP patterns |
| `blacklist` / `block` / `deny` | `list<string>` | No | `[]` | Always-blocked IP patterns |
| `protect` / `include` / `limit` | `list<string>` | No | `[]` | Path globs to protect |
| `exclude` | `list<string>` | No | `[]` | Path globs to exclude from protection |

### Example

```yaml
middleware:
  - middleware: filter
    allow:
      - "10.0.0.*"
      - "192.168.1.*"
    deny:
      - "*.tor-exit.example.com"
    limit:
      - "/admin/*"
      - "/api/internal/*"
    exclude:
      - "/api/public/*"
```

### IP Pattern Syntax

- `*` - Wildcard matching
- `10.0.0.*` - Subnet wildcard
- `192.168.1.100` - Exact IP
- CIDR notation support varies by implementation

### Implementation Details

**Source**: `config/middleware.rs::ipfilter`

**Underlying Service**: `actix_ip_filter::IPFilter`

**Evaluation Order:**
1. Check if path matches `exclude` patterns (skip filtering)
2. Check if path matches `protect` patterns (apply filtering)
3. Check IP against `whitelist` (allow if match)
4. Check IP against `blacklist` (deny if match)
5. Default: allow

**Best Practice**: Use with `ipware` middleware to ensure correct client IP detection:

```yaml
middleware:
  - middleware: ipware
    trusted_headers: [X-Forwarded-For]
  - middleware: filter
    allow: ["10.0.0.*"]
```

---

## ModSecurity Middleware

**Feature Flag**: `modsecurity`

OWASP ModSecurity Web Application Firewall integration.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `rules` | `string` | No | `""` | Inline ModSecurity rules |
| `rule_files` | `list<path>` | No | `[]` | Rule files to load |
| `max_request_body_size` | `usize` | No | - | Max request body to scan |
| `max_response_body_size` | `usize` | No | - | Max response body to scan |

### Example

```yaml
middleware:
  - middleware: modsecurity
    rules: |
      SecRuleEngine On
      SecRule REQUEST_URI "@rx /admin" "id:1,phase:1,deny,status:403"
      SecRule ARGS "@rx <script" "id:2,phase:2,deny,status:403,msg:'XSS Detected'"
    rule_files:
      - /etc/modsecurity/crs-setup.conf
      - /etc/modsecurity/rules/*.conf
    max_request_body_size: 1048576
```

### Common Rules

**Block Admin Access:**
```
SecRule REQUEST_URI "@rx ^/admin" "id:1,phase:1,deny,status:401"
```

**Block SQL Injection:**
```
SecRule ARGS "@rx (?i)(select|union|insert|update|delete|drop)" "id:2,phase:2,deny,status:403"
```

**Enable OWASP Core Rule Set:**
```yaml
rule_files:
  - /etc/modsecurity/crs-setup.conf
  - /etc/modsecurity/rules/REQUEST-*.conf
  - /etc/modsecurity/rules/RESPONSE-*.conf
```

### Implementation Details

**Source**: `config/middleware.rs::modsecurity`

**Underlying Service**: `actix_modsecurity::Middleware`

**Rule Processing Phases:**
1. Request Headers
2. Request Body
3. Response Headers
4. Response Body
5. Logging

---

## Rewrite Middleware

**Feature Flag**: `rewrite`

Apache mod_rewrite-inspired URL rewriting engine.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `rules` | `string` | No | `""` | Inline rewrite rules |
| `rule_files` | `list<path>` | No | `[]` | Rule files to load |
| `max_iterations` | `usize` | No | `10` | Max loop iterations |

### Example

```yaml
root: /var/www/html

middleware:
  - middleware: rewrite
    rules: |
      RewriteEngine On
      RewriteCond %{REQUEST_URI} !-f
      RewriteCond %{REQUEST_URI} !-d
      RewriteRule ^(.*)$ /index.php?route=$1 [L,QSA]
    max_iterations: 5
```

### Rule Syntax

**RewriteRule Format:**
```
RewriteRule Pattern Substitution [Flags]
```

**Common Flags:**
- `L` - Last rule (stop processing)
- `R` - Redirect (external)
- `R=301` - Permanent redirect
- `QSA` - Query String Append
- `NC` - No Case (case-insensitive)
- `PT` - Pass Through (internal redirect)

**RewriteCond Format:**
```
RewriteCond TestString CondPattern [Flags]
```

### Implementation Details

**Source**: `config/middleware.rs::rewrite`

**Underlying Service**: `actix_rewrite::Middleware`

**Server Variables Available:**
- `%{DOCUMENT_ROOT}` - From server config `root`
- `%{SERVER_SOFTWARE}` - "bob {version}"
- Standard Apache variables

---

## Ratelimit Middleware

**Feature Flag**: `ratelimit`

Request rate limiting with in-memory backend.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `limit` | `u64` | Yes | - | Request limit per period |
| `period` | `duration` | No | `1s` | Rate limit time window |
| `use_path` | `bool` | No | `false` | Discriminate by IP + path |
| `fail_open` | `bool` | No | `false` | Allow requests on backend failure |
| `response_headers` | `bool` | No | `false` | Include rate limit headers |

### Example

```yaml
middleware:
  - middleware: ratelimit
    limit: 100
    period: 1m
    use_path: false
    response_headers: true
```

### Response Headers

When `response_headers: true`:
- `X-RateLimit-Limit` - Request limit
- `X-RateLimit-Remaining` - Requests remaining
- `X-RateLimit-Reset` - Window reset time

### Implementation Details

**Source**: `config/middleware.rs::ratelimit`

**Underlying Service**: `actix_extensible_rate_limit::RateLimiter`

**Backend**: In-memory storage shared across workers

**Key Generation:**
- Default: Client IP address
- With `use_path: true`: IP + request path

**Failure Behavior:**
- `fail_open: false` (default): Reject on backend error
- `fail_open: true`: Allow on backend error

---

## Timeout Middleware

**Feature Flag**: `timeout`

Request processing timeout enforcement.

### Configuration

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `duration` | `u64` | Yes | - | Timeout in milliseconds |

### Example

```yaml
middleware:
  - middleware: timeout
    duration: 30000  # 30 seconds
```

### Implementation Details

**Source**: `config/middleware.rs::timeout`

**Underlying Service**: `actix_timeout::Timeout`

**Behavior:**
- Times out request processing after specified duration
- Returns 408 Request Timeout on expiry
- Includes time in middleware processing

---

## Middleware Combinations

### Production Web Server

```yaml
middleware:
  # Real IP detection
  - middleware: ipware
    trusted_headers: [X-Forwarded-For, CF-Connecting-IP]
    proxy_count: 1

  # Rate limiting
  - middleware: ratelimit
    limit: 1000
    period: 1m
    response_headers: true

  # Security filtering
  - middleware: modsecurity
    rule_files:
      - /etc/modsecurity/crs-setup.conf
      - /etc/modsecurity/rules/*.conf

  # Request timeout
  - middleware: timeout
    duration: 60000
```

### Protected Admin Area

```yaml
directives:
  - location: /admin
    construct:
      - module: fileserver
        root: /var/www/admin
      # Admin-specific middleware
      - middleware: filter
        allow: ["10.0.0.*"]
      - middleware: basic_auth_session
        htpasswd: [/etc/bob/admin.htpasswd]

  - location: /
    construct:
      - module: fileserver
        root: /var/www/public
```

### API Rate Limiting by Endpoint

```yaml
directives:
  - location: /api/heavy
    construct:
      - module: rproxy
        resolve: http://backend:8080
      - middleware: ratelimit
        limit: 10
        period: 1m
        use_path: true

  - location: /api
    construct:
      - module: rproxy
        resolve: http://backend:8080
      - middleware: ratelimit
        limit: 100
        period: 1m
```

### PHP Application with WAF

```yaml
root: /var/www/html

middleware:
  - middleware: modsecurity
    rules: |
      SecRuleEngine On
      SecRequestBodyLimit 10485760
      Include /etc/modsecurity/crs/*.conf

directives:
  - location: /
    construct:
      - module: fastcgi
        connect: /run/php/php-fpm.sock
```
