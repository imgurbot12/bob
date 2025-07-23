//! Modules Configuration

use actix_chain::Link;
use serde::Deserialize;

use super::{DirectiveCfg, ServerConfig};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "module", deny_unknown_fields)]
pub enum ModulesConfig {
    #[cfg(feature = "fileserver")]
    #[serde(alias = "fileserver")]
    FileServer(fileserver::Config),
    #[cfg(feature = "rproxy")]
    #[serde(alias = "rproxy")]
    ReverseProxy(rproxy::Config),
    #[cfg(feature = "fastcgi")]
    #[serde(alias = "fastcgi")]
    FastCGI(fastcgi::Config),
}

pub struct Spec<'a> {
    pub config: &'a ServerConfig,
    pub directive: &'a DirectiveCfg,
}

impl ModulesConfig {
    pub fn link(&self, spec: &Spec) -> Link {
        let link = match self {
            #[cfg(feature = "fileserver")]
            Self::FileServer(cfg) => Link::new(cfg.factory(spec)),
            #[cfg(feature = "rproxy")]
            Self::ReverseProxy(cfg) => Link::new(cfg.factory()),
            #[cfg(feature = "fastcgi")]
            Self::FastCGI(cfg) => Link::new(cfg.factory(spec)),
        };
        link.prefix(
            spec.directive
                .location
                .clone()
                .unwrap_or_default()
                .trim_start_matches('/'),
        )
    }
}

#[cfg(feature = "fileserver")]
mod fileserver {
    use super::*;

    use actix_files::Files;
    use std::path::PathBuf;

    #[derive(Clone, Debug, Default, Deserialize)]
    #[serde(default)]
    pub struct Config {
        root: Option<PathBuf>,
        hidden_files: bool,
    }

    impl Config {
        pub fn factory(&self, spec: &Spec) -> Files {
            let root = self
                .root
                .clone()
                .or(spec.config.root.clone())
                .unwrap_or_else(|| PathBuf::from("."));
            let mut files = Files::new("", root);
            if self.hidden_files {
                files = files.use_hidden_files();
            }
            files
        }
    }
}

#[cfg(feature = "rproxy")]
mod rproxy {
    use super::*;
    use crate::config::{Duration, Uri, default_duration};

    use actix_revproxy::RevProxy;

    #[derive(Clone, Debug, Deserialize)]
    pub struct Config {
        resolve: Uri,
        max_redirects: Option<u8>,
        initial_conn_size: Option<u32>,
        initial_window_size: Option<u32>,
        timeout: Option<Duration>,
    }

    impl Config {
        pub fn factory(&self) -> RevProxy {
            let client = awc::ClientBuilder::new()
                .initial_connection_window_size(self.initial_conn_size.unwrap_or(u16::MAX as u32))
                .initial_window_size(self.initial_window_size.unwrap_or(u16::MAX as u32))
                .max_redirects(self.max_redirects.unwrap_or(10))
                .timeout(default_duration(&self.timeout, 5))
                .finish();
            RevProxy::new("", &self.resolve.0).with_client(client)
        }
    }
}

#[cfg(feature = "fastcgi")]
mod fastcgi {
    use super::*;

    use actix_fastcgi::FastCGI;
    use std::path::PathBuf;

    #[derive(Clone, Debug, Deserialize)]
    pub struct Config {
        root: Option<PathBuf>,
        connect: String,
    }

    impl Config {
        pub fn factory(&self, spec: &Spec) -> FastCGI {
            let root = self
                .root
                .clone()
                .or(spec.config.root.clone())
                .unwrap_or_else(|| PathBuf::from("."));

            FastCGI::new("", root, &self.connect)
        }
    }
}
