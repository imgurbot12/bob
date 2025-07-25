//! Modules Configuration

use actix_chain::Link;
use serde::Deserialize;

use super::Spec;

/// Server specific configuration modules for request processing.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "module", deny_unknown_fields)]
pub enum ModulesConfig {
    /// Configuration for [`actix_files`] service.
    #[cfg(feature = "fileserver")]
    #[serde(alias = "fileserver")]
    FileServer(fileserver::Config),
    /// Configuration for [`actix_revproxy`] service.
    #[cfg(feature = "rproxy")]
    #[serde(alias = "rproxy")]
    ReverseProxy(rproxy::Config),
    /// Configuration for [`actix_fastcgi`] service.
    #[cfg(feature = "fastcgi")]
    #[serde(alias = "fastcgi")]
    FastCGI(fastcgi::Config),
}

impl ModulesConfig {
    /// Build [`actix_chain::Link`] from the module configuration.
    pub fn link(&self, spec: &Spec) -> Link {
        match self {
            #[cfg(feature = "fileserver")]
            Self::FileServer(cfg) => Link::new(cfg.factory(spec)),
            #[cfg(feature = "rproxy")]
            Self::ReverseProxy(cfg) => Link::new(cfg.factory()),
            #[cfg(feature = "fastcgi")]
            Self::FastCGI(cfg) => Link::new(cfg.factory(spec)),
        }
    }
}

#[cfg(feature = "fileserver")]
mod fileserver {
    use super::*;

    use actix_files::Files;
    use std::path::PathBuf;

    /// File-Server module configuration.
    #[derive(Clone, Debug, Default, Deserialize)]
    #[serde(default)]
    pub struct Config {
        /// Root filepath for serving files
        ///
        /// Overrides [`crate::config::ServerConfig::root`]
        root: Option<PathBuf>,
        /// Allow serving hidden files that begin with a `.`
        ///
        /// Default is false.
        hidden_files: bool,
    }

    impl Config {
        /// Produce [`actix_files::Files`] from config.
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

    /// Reverse-Proxy module configuration.
    #[derive(Clone, Debug, Deserialize)]
    pub struct Config {
        /// Proxy resolution URL.
        resolve: Uri,
        /// Max number of redirects allowed in client lookup.
        ///
        /// Default is 10.
        max_redirects: Option<u8>,
        /// Initial Connection Window Size
        ///
        /// Default is `u16::MAX`
        initial_conn_size: Option<u32>,
        /// Initial Window Size
        ///
        /// Default is `u16::MAX`
        initial_window_size: Option<u32>,
        /// Request timeout in seconds.
        ///
        /// Default is 5s
        timeout: Option<Duration>,
    }

    impl Config {
        /// Produce [`actix_revproxy::RevProxy`] from config.
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

    /// FastCGI module configuration.
    #[derive(Clone, Debug, Deserialize)]
    pub struct Config {
        /// FastCGI socket connection URI.
        connect: String,
        /// Document-Root assigned to FastCGI.
        ///
        /// Overrides [`crate::config::ServerConfig::root`].
        root: Option<PathBuf>,
    }

    impl Config {
        /// Produce [`actix_fastcgi::FastCGI`] from config.
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
