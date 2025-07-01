use std::{collections::BTreeMap, convert::Infallible, path::PathBuf};

use anyhow::Context;
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
};
use clap::Parser;
use hyper::header::HOST;
use tower::Service;

mod config;
mod modules;

//DONE: create listeners based on configuration
//DONE: assign domain mappings to axum::Router instances built with configs
//DONE: build middleware routing functions to use domain-mappings per listener
//DONE: spawn multiple axum::serve instances for each unique listener

#[derive(Debug, Parser)]
struct Cli {
    config: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct RouteCfg {
    domains: Vec<config::DomainMatch>,
    router: Router,
}

#[derive(Clone, Debug)]
struct AppState {
    routers: Vec<RouteCfg>,
}

async fn server(addr: String, mut routers: Vec<RouteCfg>) -> anyhow::Result<()> {
    // spawn listener
    log::info!("spawning listener for {addr:?}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("failed to bind address")?;
    // sort routers by number of server-names from highest to lowest
    routers.sort_by_key(|rc| rc.domains.len());
    routers.reverse();
    // build router function
    let state = AppState { routers };
    let router = middleware::from_fn_with_state(
        state.clone(),
        async |State(mut state): State<AppState>,
               req: Request<_>,
               next: Next|
               -> Result<Response, Infallible> {
            let host = req
                .headers()
                .get(HOST)
                .and_then(|h| h.to_str().map(|s| s.to_lowercase()).ok())
                .unwrap_or_default();
            let best = state
                .routers
                .iter_mut()
                .find(|rcfg| rcfg.domains.iter().any(|d| d.glob.matches(&host)))
                .map(|rcfg| &mut rcfg.router);
            let uri = req.uri().to_string();
            let method = req.method().to_string();
            let res = match best {
                None => next.run(req).await,
                Some(router) => router.call(req).await?,
            };
            log::info!("{method} {uri} {}", res.status());
            Ok(res)
        },
    );
    // spawn new axum app and serve
    let app = Router::new().layer(router).with_state(state);
    axum::serve(listener, app)
        .await
        .context("axum server failed")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let path = cli.config.unwrap_or_else(|| PathBuf::from("./config.yaml"));
    let config = config::read_config(&path)?;

    // build configurations for listener instances
    let mut routers: BTreeMap<String, Vec<RouteCfg>> = BTreeMap::new();
    for cfg in config.iter() {
        for listener in cfg.listen.iter() {
            // build routing from configured directives
            let mut router = Router::new();
            for dir in cfg.directives.iter() {
                let modules: Vec<Box<dyn modules::Module>> = dir
                    .modules
                    .clone()
                    .into_iter()
                    .map(|m| m.module())
                    .collect();
                for module in modules {
                    router = module.enable(cfg, dir, router)?;
                }
            }
            // export associated domains and router
            let addr = listener.address();
            let domains = cfg.server_name.clone();
            let names: Vec<&String> = domains.iter().map(|d| &d.pattern).collect();
            log::debug!("listener {addr:?} supported domains {names:?}");
            let routecfg = RouteCfg { domains, router };
            match routers.get_mut(&addr) {
                Some(routers) => routers.push(routecfg),
                None => {
                    routers.insert(addr, vec![routecfg]);
                }
            }
        }
    }

    // spawn server instances
    let handles: Vec<tokio::task::JoinHandle<anyhow::Result<()>>> = routers
        .into_iter()
        .map(|(addr, routers)| tokio::spawn(server(addr, routers)))
        .collect();

    // wait for server instances to complete
    for handle in handles {
        handle.await??;
    }
    Ok(())
}
