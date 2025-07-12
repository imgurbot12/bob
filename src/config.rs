//! Configuration Controls for Bob

use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, de::Error};

use crate::middleware::MiddlewareConfig;
use crate::modules::ModulesConfig;

//TODO: add defined ssl controls like prefered protocols/timeouts/ciphers

pub fn read_config(path: &PathBuf) -> Result<Vec<Config>> {
    if !path.exists() {
        return Err(anyhow!("config: {path:?} does not exist"));
    }
    let s = std::fs::read_to_string(path).context("failed to read config")?;
    let configs: Vec<Config> = serde_yaml::from_str(&s).context("invalid config")?;
    if configs.is_empty() {
        return Err(anyhow!("config: {path:?} is empty"));
    }
    Ok(configs)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub listen: Vec<ListenCfg>,
    pub server_name: Vec<DomainMatch>,
    pub middleware: MiddlewareConfig,
    pub directives: Vec<DirectiveCfg>,
    // file server global options
    pub root: Option<PathBuf>,
    pub index: Option<Vec<PathBuf>>,
    // body buffering options
    body_buffer_size: Option<usize>,
    max_body_size: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenCfg {
    pub port: u16,
    pub host: Option<String>,
    pub ssl: Option<SSLCfg>,
}

#[derive(Debug, Clone)]
pub struct DomainMatch(pub glob::Pattern);

#[derive(Debug, Clone, Deserialize)]
pub struct SSLCfg {
    pub certificate: PathBuf,
    pub certificate_key: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DirectiveCfg {
    pub location: Option<String>,
    pub locations: Vec<String>,
    pub modules: Vec<ModulesConfig>,
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

impl DirectiveCfg {
    pub fn locations(&self) -> Vec<String> {
        let mut locations = self.locations.clone();
        if let Some(location) = self.location.clone() {
            locations.insert(0, location);
        }
        locations
    }
}

impl FromStr for DomainMatch {
    type Err = glob::PatternError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let glob = glob::Pattern::new(s)?;
        Ok(Self(glob))
    }
}

#[derive(Clone, Debug)]
pub struct Duration(pub(crate) std::time::Duration);

impl Duration {
    #[inline]
    pub fn from_secs(secs: u64) -> std::time::Duration {
        std::time::Duration::from_secs(secs)
    }
}

impl FromStr for Duration {
    type Err = humantime::DurationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(humantime::parse_duration(s)?))
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

pub(crate) use de_fromstr;

de_fromstr!(DomainMatch);
de_fromstr!(Duration);
