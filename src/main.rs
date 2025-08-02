use std::path::PathBuf;

use actix_chain::Chain;
use actix_web::{App, HttpServer, middleware::Logger};
use anyhow::{Context, Result};
use clap::Parser;

pub mod config;
pub mod tls;

use crate::config::{ServerConfig, Spec};

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
