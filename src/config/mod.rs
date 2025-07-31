//! Configuration Serializer/Deserializer Types

use std::{path::PathBuf, str::FromStr};

use actix_web::{guard::Guard, http::header};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, de::Error};

mod middleware;
mod modules;

pub use middleware::MiddlewareConfig;
pub use modules::ModulesConfig;

/// Read all server configurations from a config file.
pub fn read_config(path: &PathBuf) -> Result<Vec<ServerConfig>> {
    let s = std::fs::read_to_string(path).context("failed to read config")?;
    let configs: Vec<ServerConfig> = serde_yaml::from_str(&s).context("invalid config")?;
    match configs.is_empty() {
        true => Err(anyhow!("config: {path:?} is empty")),
        false => Ok(configs),
    }
}

//TODO: implement index retry support middlware.

/// Server specific configuration settings.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    /// Disable configuration from initialization within server.
    pub disable: bool,
    /// List of configurations for binding server addresses.
    pub listen: Vec<ListenCfg>,
    /// List of domain-names matchers with the server.
    ///
    /// Once registered, the server will only respond to
    /// requests with `Host` set to the relevant matchers.
    pub server_name: Vec<DomainMatch>,
    /// Configuration settings for middlware within server instance.
    pub middleware: MiddlewareConfig,
    /// Request handling directives associated with server instance.
    pub directives: Vec<DirectiveCfg>,
    /// Default root filepath for various request handling modules.
    pub root: Option<PathBuf>,
    /// List of supported index file patterns when requesting resources.
    ///
    /// Default is [index.html, ]
    pub index: Vec<String>,
    /// Default maximum buffer-size when reading messages into memory.
    pub body_buffer_size: Option<usize>,
    /// Request logging toggle.
    ///
    /// Default is true
    pub log_requests: Option<bool>,
    /// Sanitizes error-messages produced by configured modules when enabled.
    ///
    /// Default is true
    pub sanitize_errors: Option<bool>,
}

pub struct Spec<'a> {
    pub config: &'a ServerConfig,
}

/// Domain matcher expression.
///
/// Uses glob syntax.
#[derive(Debug, Clone)]
pub struct DomainMatch(pub glob::Pattern);

impl Guard for DomainMatch {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        match ctx.head().headers.get(header::HOST) {
            Some(host) => self.0.matches(host.to_str().unwrap_or_default()),
            None => false,
        }
    }
}

impl FromStr for DomainMatch {
    type Err = glob::PatternError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let glob = glob::Pattern::new(s)?;
        Ok(Self(glob))
    }
}

/// TLS Configuration for server listener.
#[derive(Debug, Clone, Deserialize)]
pub struct SSLCfg {
    /// TLS Certificate public key.
    pub certificate: PathBuf,
    /// TLS Certificate private key.
    pub certificate_key: PathBuf,
}

/// Server listener bindings configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ListenCfg {
    /// Port server will bind to.
    pub port: u16,
    /// Host address server will bind to.
    pub host: Option<String>,
    /// SSL configuration for listener.
    pub ssl: Option<SSLCfg>,
}

impl ListenCfg {
    #[inline]
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("0.0.0.0")
    }
    #[inline]
    pub fn address(&self) -> (String, u16) {
        (self.host().to_owned(), self.port)
    }
}

/// Group of request modules bound to a specific uri path prefix.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DirectiveCfg {
    /// List of request modules configurations bound to directive.
    pub modules: Vec<ModulesConfig>,
    /// Location associated with modules
    ///
    /// Default is `/`
    pub location: Option<String>,
}

/// Time duration parsed from human-readable format.
///
/// Example: `1h5m2s`
#[derive(Clone, Debug)]
pub struct Duration(pub(crate) std::time::Duration);

impl FromStr for Duration {
    type Err = humantime::DurationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(humantime::parse_duration(s)?))
    }
}

/// Resource URI object and parser.
#[derive(Clone, Debug)]
pub struct Uri(pub(crate) actix_web::http::Uri);

impl FromStr for Uri {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            actix_web::http::Uri::from_str(s).map_err(|e| e.to_string())?,
        ))
    }
}

macro_rules! de_fromstr {
    ($s:ident) => {
        impl<'de> Deserialize<'de> for $s {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: String = Deserialize::deserialize(deserializer)?;
                $s::from_str(&s).map_err(D::Error::custom)
            }
        }
    };
}

de_fromstr!(DomainMatch);
de_fromstr!(Duration);
de_fromstr!(Uri);

#[inline]
pub fn default_duration(d: &Option<Duration>, default_secs: u64) -> std::time::Duration {
    d.as_ref()
        .map(|d| d.0)
        .unwrap_or_else(|| std::time::Duration::from_secs(default_secs))
}
