use std::path::PathBuf;

use actix_web::{App, HttpServer};
use anyhow::Context;
use clap::Parser;

mod config;
mod middleware;
mod modules;

use config::{Config, ListenCfg, SSLCfg};

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
    config: Option<PathBuf>,
}

//DONE: libmodsecurity middleware

//TODO: ip whitelist/blacklist middleware
//TODO: bot challenge middleware
//TODO: ratelimit middleware
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
    let lcfg = listen.clone();
    let server = HttpServer::new(move || {
        let svc = modules::build_modules(&config, &lcfg);

        App::new()
            .wrap(config.middleware.modsecurity(&lcfg))
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
