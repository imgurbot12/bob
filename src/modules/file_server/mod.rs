//! Static File Server Module

use std::path::PathBuf;

use anyhow::Result;
use axum::{Router, body::Body, extract::Request, response::Response};
use serde::Deserialize;
use tower_http::services::ServeDir;

mod resolve;

use super::Module;
use crate::config::{Config, DirectiveCfg};

//TODO: implement fallback handler for default?
//TODO: implement support for dynamic status-code handlers?

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FSModule {
    root: Option<PathBuf>,
    try_files: Vec<String>,
}

impl Module for FSModule {
    fn enable(&self, cfg: &Config, dir: &DirectiveCfg, mut router: Router) -> Result<Router> {
        let root = cfg
            .root
            .clone()
            .or(self.root.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        for location in dir.locations() {
            log::debug!("configuring file_server for {location:?} -> {root:?}");
            let indexes = cfg.index.clone().unwrap_or_default();
            let server = resolve::SmartServeDir::new(&root, &self.try_files, indexes);
            router = router.nest_service(&location, server);
        }
        Ok(router)
    }
}
