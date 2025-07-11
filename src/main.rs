use std::path::PathBuf;

use actix_web::{App, HttpServer};
use anyhow::Context;
use clap::Parser;

mod config;
mod middleware;
mod modules;

use config::{Config, ListenCfg, SSLCfg};

#[derive(Debug, Parser)]
struct Cli {
    config: Option<PathBuf>,
}

//TODO: ip whitelist/blacklist middleware
//TODO: bot challenge middleware
//TODO: ratelimit middleware
//TODO: libmodsecurity middleware
//TODO: php-fpm module (https://crates.io/crates/fastcgi-client)

//TODO: make ssl feature trait, add dependant feature for actix-web

fn build_tls_config(cfg: &SSLCfg) -> anyhow::Result<rustls::ServerConfig> {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};

    let certs = CertificateDer::pem_file_iter(&cfg.certificate)
        .context("failed to read tls certificate")?
        .map(|pem| pem.expect("invalid pem"))
        .collect();
    let private_key =
        PrivateKeyDer::from_pem_file(&cfg.certificate_key).context("invalid private tls key")?;

    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)
        .context("failed to build rustls server config")
}

async fn server(config: Config, listen: ListenCfg) -> anyhow::Result<()> {
    let server = HttpServer::new(move || {
        let svc = modules::build_modules(&config);

        App::new()
            .wrap(config.middleware.modsecurity())
            .service(svc)
    });

    let addr = (listen.host(), listen.port);
    let bind = match listen.ssl.as_ref() {
        None => server.bind(addr).context("listener bind failed")?,
        Some(cfg) => {
            let tls = build_tls_config(cfg)?;
            server
                .bind_rustls_0_23(addr, tls)
                .context("tls listener bind failed")?
        }
    };

    bind.run().await.context("http server failed")
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let path = cli.config.unwrap_or_else(|| PathBuf::from("./config.yaml"));
    let config = config::read_config(&path)?;

    let tasks: Vec<actix_web::rt::task::JoinHandle<anyhow::Result<()>>> = config
        .into_iter()
        .map(|cfg| {
            cfg.listen
                .clone()
                .into_iter()
                .map(|l| (cfg.clone(), l))
                .collect::<Vec<(Config, ListenCfg)>>()
        })
        .flatten()
        .map(|(cfg, lcfg)| actix_web::rt::spawn(server(cfg, lcfg)))
        .collect();

    for task in tasks {
        task.await??
    }
    Ok(())
}
