#![doc = include_str!("../../README.md")]
#![cfg_attr(feature = "doc", feature(doc_auto_cfg))]

use actix_chain::{Chain, Link};
use actix_web::{App, HttpServer, middleware::Logger};
use anyhow::{Context, Result};
use clap::Parser;

mod cli;
mod config;
mod tls;

use crate::config::{ServerConfig, Spec};

//TODO: existing logging middleware does not log errors.
// look into alternatives or make a PR?
// https://github.com/actix/actix-web/issues/1051

//TODO: integrate ipware directly as real-ip extractor?
// can u overwrite remote-addr in service?
//
// would it be better to use the `extra_data` method?
// (that would likely require a feature for all services to support)

//TODO: confirm fastcgi has its own timeout (allow config??)
//TODO: confirm rev-proxy has its own timeout (allow config.)

//TODO: look into logging configuration for config,
// but also to see if u can speed up operations to avoid slowdown.

//TODO: ratelimitter middleware
//TODO: simple bot detector/challenger system? - anubis lite
//TODO: configurable static-response module
// (status, headers, body)

//TODO: metrics/healthcheck module
// (with configurable secure access)

//TODO: cli sub-commands intended for simple configurations
// like `caddy fileserver` / `caddy reverse-proxy` / etc...
// - fileserver [DONE]
// - revproxy   [DONE]
// - fastcgi    [DONE]
// - static
// - redirect
//  (all the modules basically...)
//  (fileserver should auto-open browser when tty)
//  (info logging should probably be enabled by default)

//TODO: hot-reload option for when config changes?
//TODO: daemonize option?

#[inline]
fn logger(config: &ServerConfig) -> Logger {
    #[cfg(not(feature = "ipware"))]
    let log = Logger::default();

    #[cfg(feature = "ipware")]
    let log = match config.logging.use_ipware.unwrap_or(true) {
        false => Logger::default(),
        true => Logger::new(r#"%{ip}xo "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#)
            .custom_response_replace("ip", |res| {
                res.request()
                    .peer_addr()
                    .map(|r| r.ip().to_string())
                    .unwrap_or_default()
            }),
    };

    log.log_level(
        config
            .logging
            .log_level
            .clone()
            .map(|l| l.0)
            .unwrap_or(log::Level::Info),
    )
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

        let link: Link = directive
            .construct
            .iter()
            .fold(Chain::new(prefix), |chain, c| c.apply(chain, &spec))
            .into();

        chain.push_link(link);
    }

    chain = config
        .middleware
        .iter()
        .fold(chain, |chain, m| m.wrap(chain, &spec));
    if config.sanitize_errors.unwrap_or(true) {
        chain = chain.wrap(actix_sanitize::Sanitizer::default());
    }
    if !config.logging.disable {
        chain = chain.wrap(logger(config));
    }

    chain
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_target(false)
        .filter(None, log::LevelFilter::Warn)
        .filter(Some("bob"), log::LevelFilter::Info)
        .filter(
            Some("actix_web::middleware::logger"),
            log::LevelFilter::Info,
        )
        .parse_env("BOB_LOG")
        .init();

    let cli = bob_cli::Cli::parse();
    let config = cli::build_config(cli)?;

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
        .try_fold(server, |s, addr| {
            log::info!("spawning listener {addr:?}");
            s.bind(addr)
        })?;

    let sslcfg = tls::server::build_tls_config(&config)?;
    server = config
        .iter()
        .filter(|cfg| !cfg.disable)
        .flat_map(|cfg| cfg.listen.iter())
        .filter(|listen| listen.ssl.is_some())
        .map(|addr| addr.address())
        .try_fold(server, |s, addr| {
            log::info!("spawning tls listener {addr:?}");
            s.bind_rustls_0_23(addr, sslcfg.clone())
        })?;

    log::info!("server listening and ready!");
    server.run().await.context("server spawn failed")
}
