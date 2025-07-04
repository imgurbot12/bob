//!

use std::path::PathBuf;

use serde::Deserialize;

use crate::config::Config;

mod error;
mod factory;
mod path_buf;
mod service;

//TODO: directive/module controls over passing to next for specified status-codes

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct FileServerConfig {
    root: Option<PathBuf>,
    hidden_files: bool,
}

impl FileServerConfig {
    pub fn into_factory(&self, cfg: &Config) -> factory::FileServer {
        let index = cfg.index.clone().unwrap_or_default();
        let root = self
            .root
            .clone()
            .or(cfg.root.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        factory::FileServer::new("", root)
            .directory_index(index)
            .hidden_files(self.hidden_files)
    }
}
