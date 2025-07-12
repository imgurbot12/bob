//! PHP FPM Module

use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, de::Error};

use crate::config::{Config, Duration, ListenCfg, de_fromstr};

mod factory;
mod pool;
mod service;

#[derive(Clone, Debug)]
struct Regex(regex::Regex);

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct FastCGIConfig {
    root: Option<PathBuf>,
    connect: String,
    path_param: Option<Regex>,
    idle_timeout: Option<Duration>,
    conn_timeout: Option<Duration>,
    max_lifetime: Option<Duration>,
    max_pool_size: Option<u32>,
    min_idle: Option<u32>,
}

impl FastCGIConfig {
    pub fn into_factory(&self, cfg: &Config, lsn: &ListenCfg) -> factory::FastCGI {
        let manager = pool::ConnectionManager::new(&self.connect);
        let pool = bb8::Pool::builder()
            .idle_timeout(
                self.idle_timeout
                    .clone()
                    .map(|d| d.0)
                    .unwrap_or_else(|| Duration::from_secs(30)),
            )
            .connection_timeout(
                self.conn_timeout
                    .clone()
                    .map(|d| d.0)
                    .unwrap_or_else(|| Duration::from_secs(5)),
            )
            .max_lifetime(self.max_lifetime.clone().map(|d| d.0))
            .max_size(self.max_pool_size.unwrap_or(10))
            .min_idle(self.min_idle)
            .max_lifetime(self.max_lifetime.clone().map(|d| d.0))
            .build_unchecked(manager);
        let root = self
            .root
            .clone()
            .or(cfg.root.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        factory::FastCGI::new("", root, lsn.address(), pool)
            .path_param(self.path_param.clone().map(|r| r.0))
    }
}

impl FromStr for Regex {
    type Err = regex::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(regex::Regex::new(s)?))
    }
}

de_fromstr!(Regex);
