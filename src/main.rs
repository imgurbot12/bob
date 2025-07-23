use std::path::PathBuf;

use actix_chain::Chain;
use actix_web::{App, HttpServer};
use anyhow::{Context, Result};
use clap::Parser;

mod config;
mod tls;

use crate::config::{ServerConfig, Spec};

#[derive(Debug, Parser)]
struct Cli {
    config: Option<PathBuf>,
}

fn assemble_chain(config: &ServerConfig) -> Chain {
    let mut chain = Chain::new("");
    chain = config
        .server_name
        .clone()
        .into_iter()
        .fold(chain, |chain, domain| chain.guard(domain));

    for directive in config.directives.iter() {
        let spec = Spec { directive, config };
        for module in directive.modules.iter() {
            let link = module.link(&spec);
            chain.push_link(link);
        }
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
        let app = sconfig
            .iter()
            .map(assemble_chain)
            .fold(App::new(), |app, chain| app.service(chain));
        app
    });

    server = config
        .iter()
        .map(|cfg| cfg.listen.iter())
        .flatten()
        .filter(|listen| listen.ssl.is_none())
        .map(|addr| addr.address())
        .try_fold(server, |s, addr| s.bind(addr))?;

    let sslcfg = tls::build_tls_config(&config)?;
    server = config
        .iter()
        .map(|cfg| cfg.listen.iter())
        .flatten()
        .filter(|listen| listen.ssl.is_some())
        .map(|addr| addr.address())
        .try_fold(server, |s, addr| s.bind_rustls_0_23(addr, sslcfg.clone()))?;

    log::info!("spawning server");
    server.run().await.context("server spawn failed")
}
