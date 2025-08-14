//! Modules Configuration

use actix_chain::{Link, next};
use actix_web::http::StatusCode;
use serde::Deserialize;

use super::Spec;

/// Server specific configuration modules for request processing.
#[derive(Clone, Debug, Deserialize)]
pub struct Module {
    /// Module specific configuration.
    #[serde(flatten)]
    module: ModuleConfig,
    /// Override of [`actix_chain::Link::next`] behavior.
    #[serde(default)]
    next: Option<Vec<u16>>,
}

impl Module {
    /// Build [`actix_chain::Link`] from the module configuration.
    #[inline]
    pub fn link(&self, spec: &Spec) -> Link {
        let mut link = self.module.link(spec);
        if let Some(next) = self.next.as_ref() {
            println!("next! {next:?}");
            link = next
                .iter()
                .filter_map(|code| StatusCode::from_u16(*code).ok())
                .map(|code| next::IsStatus(code))
                .fold(link, |link, code| link.next(code));
        }
        link
    }
}

/// Configuration modules for request processing.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "module", deny_unknown_fields)]
pub enum ModuleConfig {
    /// Configuration for buitltin redirect service.
    #[serde(alias = "redirect")]
    Redirect(redirect::Config),
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

impl ModuleConfig {
    /// Build [`actix_chain::Link`] from the module configuration.
    pub fn link(&self, spec: &Spec) -> Link {
        match self {
            Self::Redirect(cfg) => cfg.link(spec),
            #[cfg(feature = "fileserver")]
            Self::FileServer(cfg) => cfg.link(spec),
            #[cfg(feature = "rproxy")]
            Self::ReverseProxy(cfg) => cfg.link(spec),
            #[cfg(feature = "fastcgi")]
            Self::FastCGI(cfg) => cfg.link(spec),
        }
    }
}

mod redirect {
    use super::*;

    use actix_web::{
        HttpResponse, Route,
        http::{StatusCode, header},
    };

    /// Redirect module configuration
    #[derive(Clone, Debug, Default, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Redirect URI
        redirect: String,
        /// Redirect status code
        ///
        /// Default is 302
        status_code: Option<u16>,
    }

    impl Config {
        /// Produce [`actix_web::Route`] from config.
        pub fn factory(&self) -> Route {
            let status_code = self.status_code.unwrap_or(302);

            let uri = self.redirect.to_owned();
            let status = StatusCode::from_u16(status_code).expect("invalid redirect status");
            actix_web::web::get().to(move || {
                let mut builder = HttpResponse::build(status);
                builder.insert_header((header::LOCATION, uri.clone()));
                builder
            })
        }

        /// Produce [`actix_chain::Link`] from config.
        #[inline]
        pub fn link(&self, _spec: &Spec) -> Link {
            Link::new(self.factory())
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
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        /// Root filepath for serving files
        ///
        /// Overrides [`crate::config::ServerConfig::root`]
        root: Option<PathBuf>,
        /// Allow serving hidden files that begin with a `.`
        ///
        /// Default is false.
        hidden_files: bool,
        /// Size Threshold for Asyncly Processing Files
        ///
        /// Default is u16::MAX (65_365)
        async_threshold: Option<u64>,
    }

    impl Config {
        /// Produce [`actix_files::Files`] from config.
        pub fn factory(&self, spec: &Spec) -> Files {
            let root = self
                .root
                .clone()
                .or(spec.config.root.clone())
                .unwrap_or_else(|| PathBuf::from("."));
            let mut files = Files::new("", root)
                .set_size_threshold(self.async_threshold.unwrap_or(u16::MAX as u64));
            if self.hidden_files {
                files = files.use_hidden_files();
            }
            spec.config
                .index
                .iter()
                .fold(files, |files, index| files.index_file(index))
        }

        /// Produce [`actix_chain::Link`] from config.
        #[inline]
        pub fn link(&self, spec: &Spec) -> Link {
            Link::new(self.factory(spec))
        }
    }
}

#[cfg(feature = "rproxy")]
mod rproxy {
    use std::{collections::BTreeMap, sync::Arc};

    use super::*;
    use crate::config::{Duration, Uri, default_duration};

    use crate::tls::client::build_tls_config;
    use actix_revproxy::RevProxy;

    /// Reverse-Proxy module configuration.
    #[derive(Clone, Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Proxy resolution URL.
        resolve: Uri,
        /// Max number of redirects allowed in client lookup.
        ///
        /// Default is 0.
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
        /// Verify SSL Configuration
        ///
        /// Default is true
        verify_ssl: Option<bool>,
        /// Upstream headers to send to server.
        #[serde(default)]
        upstream_headers: BTreeMap<String, String>,
        /// Downstream headers to send to client.
        #[serde(default)]
        downstream_headers: BTreeMap<String, String>,
    }

    impl Config {
        /// Produce [`actix_revproxy::RevProxy`] from config.
        pub fn factory(&self) -> RevProxy {
            let mut connector = awc::Connector::new();
            if !self.verify_ssl.unwrap_or(true) {
                let config = build_tls_config(false);
                connector = connector.rustls_0_23(Arc::new(config));
            }
            let client = awc::ClientBuilder::new()
                .connector(connector)
                .no_default_headers()
                .initial_connection_window_size(self.initial_conn_size.unwrap_or(u16::MAX as u32))
                .initial_window_size(self.initial_window_size.unwrap_or(u16::MAX as u32))
                .timeout(default_duration(&self.timeout, 5))
                .max_redirects(self.max_redirects.unwrap_or(0))
                .finish();
            let mut proxy = RevProxy::new("", &self.resolve.0).with_client(client);
            proxy = self
                .upstream_headers
                .iter()
                .fold(proxy, |proxy, (k, v)| proxy.upstream_header(k, v));
            proxy = self
                .downstream_headers
                .iter()
                .fold(proxy, |proxy, (k, v)| proxy.downstream_header(k, v));
            proxy
        }

        /// Produce [`actix_chain::Link`] from config.
        #[inline]
        pub fn link(&self, _spec: &Spec) -> Link {
            Link::new(self.factory())
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
    #[serde(deny_unknown_fields)]
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
            let fastcgi = FastCGI::new("", root, &self.connect);
            spec.config
                .index
                .iter()
                .fold(fastcgi, |fastcgi, index| fastcgi.index_file(index))
        }

        /// Produce [`actix_chain::Link`] from config.
        #[inline]
        pub fn link(&self, spec: &Spec) -> Link {
            Link::new(self.factory(spec))
        }
    }
}
