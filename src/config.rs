//! Configuration Controls for Bob

use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, de::Error};

use crate::modules::*;

//TODO: add dynamic server-name match using glob
//TODO: add dynamic location matcher that compiles to tower routes?
//TODO: add dynamic string deserialization for listen-cfg
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

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen: Vec<ListenCfg>,
    pub server_name: Vec<DomainMatch>,
    pub directives: Vec<DirectiveCfg>,
    // file server global options
    pub root: Option<PathBuf>,
    pub index: Option<Vec<PathBuf>>,
}

#[derive(Debug, Deserialize)]
pub struct ListenCfg {
    pub port: u16,
    pub host: Option<String>,
    pub ssl: SSLCfg,
}

#[derive(Debug, Clone)]
pub struct DomainMatch {
    pub pattern: String,
    pub glob: glob::Pattern,
}

#[derive(Debug, Deserialize)]
pub struct SSLCfg {
    pub certificate: PathBuf,
    pub certificate_key: PathBuf,
    pub dhparam: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct DirectiveCfg {
    pub location: Option<String>,
    pub locations: Vec<String>,
    pub modules: Vec<ModuleCfg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "module")]
pub enum ModuleCfg {
    #[cfg(feature = "fs")]
    #[serde(alias = "file_server")]
    FileServer(FSModule),
    #[cfg(feature = "rev_proxy")]
    #[serde(alias = "proxy")]
    ReverseProxy(RevProxyModule),
}

impl ListenCfg {
    pub fn address(&self) -> String {
        format!(
            "{}:{}",
            self.host.as_ref().map(|s| s.as_str()).unwrap_or("0.0.0.0"),
            self.port
        )
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

impl ModuleCfg {
    pub fn module(self) -> Box<dyn Module> {
        match self {
            #[cfg(feature = "fs")]
            Self::FileServer(fs) => Box::new(fs),
            #[cfg(feature = "rev_proxy")]
            Self::ReverseProxy(rp) => Box::new(rp),
        }
    }
}

impl FromStr for DomainMatch {
    type Err = glob::PatternError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let glob = glob::Pattern::new(s)?;
        Ok(Self {
            pattern: s.to_owned(),
            glob,
        })
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
