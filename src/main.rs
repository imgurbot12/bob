use std::path::PathBuf;

use actix_chain::Chain;
use actix_web::{App, HttpServer, middleware::Logger};
use anyhow::{Context, Result};
use clap::Parser;

pub mod config;
pub mod tls;

use crate::config::{ServerConfig, Spec};

//TODO: integrate ipware directly as real-ip extractor?
// can u overwrite remote-addr in service?
//
// would it be better to use the `extra_data` method?
// (that would likely require a feature for all services to support)

//TODO: modify middleware construction to control order of wrapping?
// (allows much tighter controls of construction and operation.)
// (mayhaps even co-mingle with modules, so they can be constructed in a flat list?)

//TODO: confirm fastcgi has its own timeout (allow config??)
//TODO: confirm rev-proxy has its own timeout (allow config.)

//TODO: look into logging configuration for config,
// but also to see if u can speed up operations to avoid slowdown.

//TODO: ip whitelist/blacklist middleware implementation
//TODO: ratelimitter middleware
//TODO: timeout middleware
//TODO: simple bot detector/challenger system? - anubis lite
//TODO: configurable static-response module
// (status, headers, body)

//TODO: metrics/healthcheck module
// (with configurable secure access)

//TODO: cli sub-commands intended for simple configurations
// like `caddy fileserver` / `caddy reverse-proxy` / etc...
// - fileserver
// - revproxy
// - fastcgi
// - static
// - redirect
//  (all the modules basically...)
//  (fileserver should auto-open browser when tty)
//  (info logging should probably be enabled by default)

//TODO: hot-reload option for when config changes?
//TODO: daemonize option?

/// The greatest of all reverse proxies, and
/// written in 🦀 (so you KNOW ITS GOOD 👌)
#[derive(Debug, Parser)]
struct Cli {
    /// Path of configuration to load (default: ./config.yaml).
    #[clap(short, long)]
    config: Option<PathBuf>,
}

/// Assemble [`actix_chain::Chain`] from server configuration instance.
fn assemble_chain(config: &ServerConfig) -> Chain {
    let mut chain = Chain::default();
    chain = config
        .server_name
        .clone()
        .into_iter()
        .fold(chain, |chain, domain| chain.guard(domain));

    let spec = Spec { config };
    for directive in config.directives.iter() {
        let location = directive.location.clone().unwrap_or_default();
        let prefix = location.trim_start_matches('/');

        let mut link: actix_chain::Link = directive
            .modules
            .iter()
            .fold(Chain::new(prefix), |chain, m| chain.link(m.link(&spec)))
            .into();

        link = directive.middleware.wrap(link, &spec);
        chain.push_link(link);
    }

    chain = config.middleware.wrap(chain, &spec);
    if config.sanitize_errors.unwrap_or(true) {
        chain = chain.wrap(actix_sanitize::Sanitizer::default());
    }
    if config.log_requests.unwrap_or(true) {
        chain = chain.wrap(Logger::default());
    }

    chain
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let path = cli.config.unwrap_or_else(|| PathBuf::from("./config.yaml"));
    let config = config::read_config(&path)?;

    let sconfig = config.clone();
    let mut server = HttpServer::new(move || {
        sconfig
            .iter()
            .map(assemble_chain)
            .fold(App::new(), |app, cfg| app.service(cfg))
    });

    server = config
        .iter()
        .filter(|cfg| !cfg.disable)
        .flat_map(|cfg| cfg.listen.iter())
        .filter(|listen| listen.ssl.is_none())
        .map(|addr| addr.address())
        .try_fold(server, |s, addr| s.bind(addr))?;

    let sslcfg = tls::server::build_tls_config(&config)?;
    server = config
        .iter()
        .filter(|cfg| !cfg.disable)
        .flat_map(|cfg| cfg.listen.iter())
        .filter(|listen| listen.ssl.is_some())
        .map(|addr| addr.address())
        .try_fold(server, |s, addr| s.bind_rustls_0_23(addr, sslcfg.clone()))?;

    log::info!("spawning server");
    server.run().await.context("server spawn failed")
}
