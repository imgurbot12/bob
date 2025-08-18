bob
----

Bob is an Easy to use Web Server and Reverse Proxy Service

Why is it called Bob? I have no idea what else to call it for now.
_Bob gets the job done._

### Features

- **Blazingly Fast 🔥**
- **Simple configuration** with yaml
- **HTTP 1.1 and HTTP/2** supported by default
- **Builtin [`ModSecurity`](https://modsecurity.org/) support** (No plugins required!)
- **Written in Rust 🦀** (and powered by [actix-web](https://actix.rs/))

### Build from Source

```bash
$ git clone https://github.com/imgurbot12/bob.git
$ cd bob/bob
$ cargo install --path .
```

When you run Bob, you may wish to bind your server to low ports. If your
OS requires elevated privileges for this, you will need to give your new
binary permission to do so. On Linux, this can be done easily
with: `sudo setcap cap_net_bind_service=+ep ./bob`

### Quick Start

View all available options with the built-in help:

```bash
$ bob --help
```

Bob will try and run the server using a yaml configuration by
default called `config.yaml`.
See [`example-config.yaml`](https://github.com/imgurbot12/bob/blob/master/example-config.yaml)
as a basic reference on how to get started.

### Documentation

Further documentation is available at [here](https://imgurbot12.github.io/bob/).

### Modules and Middleware

Bob comes with a selection of builtin request processing modules and middleware.

##### Modules

| Name         | Description                                   |
| :----------: | :-------------------------------------------- |
| Fileserver   | HTTP Fileserver                               |
| ReverseProxy | HTTP Reverse Proxy                            |
| FastCGI      | FastCGI Client (useful for PHP frontend)      |
| Redirect     | Basic Configurable Static HTTP Redirect       |
| Static       | Wicked-Fast Configurable Static HTTP-Response |

##### Middleware

| Name        | Description                               |
| :---------: | :---------------------------------------- |
| AuthBasic   | HTTP BasicAuth                            |
| AuthSession | HTTP BasicAuth with Cookie Session        |
| ModSecurity | OWASP Modsecurity WAF Integration         |
| Rewrite     | Apache2 `mod_rewrite` inspired middleware |
