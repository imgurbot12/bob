//! Configuration Serializer/Deserializer Types

use std::{path::PathBuf, str::FromStr};

use actix_web::{guard::Guard, http::header};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, de::Error};

mod modules;

pub use modules::{ModulesConfig, Spec};

pub fn read_config(path: &PathBuf) -> Result<Vec<ServerConfig>> {
    let s = std::fs::read_to_string(path).context("failed to read config")?;
    let configs: Vec<ServerConfig> = serde_yaml::from_str(&s).context("invalid config")?;
    match configs.is_empty() {
        true => Err(anyhow!("config: {path:?} is empty")),
        false => Ok(configs),
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub disable: bool,
    pub listen: Vec<ListenCfg>,
    pub server_name: Vec<DomainMatch>,
    // pub middleware: MiddlewareConfig,
    pub directives: Vec<DirectiveCfg>,
    pub root: Option<PathBuf>,
    pub index: Option<Vec<PathBuf>>,
    pub body_buffer_size: Option<usize>,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct SSLCfg {
    pub certificate: PathBuf,
    pub certificate_key: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenCfg {
    pub port: u16,
    pub host: Option<String>,
    pub ssl: Option<SSLCfg>,
}

impl ListenCfg {
    #[inline]
    pub fn host(&self) -> &str {
        self.host.as_ref().map(|s| s.as_str()).unwrap_or("0.0.0.0")
    }
    #[inline]
    pub fn address(&self) -> (String, u16) {
        (self.host().to_owned(), self.port)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DirectiveCfg {
    pub location: Option<String>,
    pub modules: Vec<ModulesConfig>,
}

#[derive(Clone, Debug)]
pub struct Duration(pub(crate) std::time::Duration);

impl FromStr for Duration {
    type Err = humantime::DurationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(humantime::parse_duration(s)?))
    }
}

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
        .map(|d| d.0.clone())
        .unwrap_or_else(|| std::time::Duration::from_secs(default_secs))
}
