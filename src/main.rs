use std::path::PathBuf;

use actix_web::{App, HttpServer};
use anyhow::Context;
use clap::Parser;

mod config;
mod modules;

use config::{Config, ListenCfg};

#[derive(Debug, Parser)]
struct Cli {
    config: Option<PathBuf>,
}

async fn server(config: Config, listen: ListenCfg) -> anyhow::Result<()> {
    HttpServer::new(move || {
        let svc = modules::build_modules(&config);
        App::new().service(svc)
    })
    .bind((listen.host(), listen.port))
    .context("listener bind failed")?
    .run()
    .await
    .context("http server failed")
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
