//! Configuration Serializer/Deserializer Types

use std::{net::SocketAddr, path::PathBuf, str::FromStr};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

use actix_chain::Chain;
use actix_web::{guard::Guard, http::header};
use anyhow::{Context, Result, anyhow};
use bob_cli::{Duration, Uri, de_fromstr};
use serde::{
    Deserialize,
    de::{self, Error, Unexpected},
};

pub mod middleware;
pub mod modules;

pub use middleware::Middleware;
pub use modules::{Module, ModuleConfig};

/// Read all server configurations from a config file.
pub fn read_config(path: &PathBuf) -> Result<Vec<ServerConfig>> {
    let s = std::fs::read_to_string(path).context("failed to read config")?;
    let configs: Vec<ServerConfig> = serde_yaml::from_str(&s).context("invalid config")?;
    match configs.is_empty() {
        true => Err(anyhow!("config: {path:?} is empty")),
        false => Ok(configs),
    }
}

/// Server specific configuration settings.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
    pub middleware: Vec<Middleware>,
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

/// Compilation of references to config specifications
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

#[cfg(feature = "schema")]
impl JsonSchema for DomainMatch {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DomainMatch".into()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::DomainMatch").into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// TLS Configuration for server listener.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SSLCfg {
    /// TLS Certificate public key.
    pub certificate: PathBuf,
    /// TLS Certificate private key.
    pub certificate_key: PathBuf,
}

/// Server listener bindings configuration.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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

impl From<SocketAddr> for ListenCfg {
    fn from(value: SocketAddr) -> Self {
        Self {
            port: value.port(),
            host: Some(value.ip().to_string()),
            ssl: None,
        }
    }
}

/// Module or Middleware Component
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Debug, Clone)]
pub enum Component {
    Middleware(Middleware),
    Module(Module),
}

impl Component {
    /// Apply component to Chain.
    pub fn apply(&self, chain: Chain, spec: &Spec) -> Chain {
        match &self {
            Component::Module(m) => chain.link(m.link(spec)),
            Component::Middleware(m) => m.wrap(chain, spec),
        }
    }
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_yaml::Value::deserialize(deserializer)?;
        Ok(match value.get("module").is_some() {
            true => Component::Module(
                serde_yaml::from_value::<Module>(value).map_err(D::Error::custom)?,
            ),
            false => Component::Middleware(
                serde_yaml::from_value::<Middleware>(value).map_err(D::Error::custom)?,
            ),
        })
    }
}

/// Group of request modules bound to a specific uri path prefix.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectiveCfg {
    /// List of additional web components bound to directive.
    ///
    /// Items are constructed in the order they're given
    /// meaning middlewares only wrap elements defined before them.
    pub construct: Components,
    /// Location associated with modules
    ///
    /// Default is `/`
    pub location: Option<String>,
}

impl From<ModuleConfig> for DirectiveCfg {
    fn from(value: ModuleConfig) -> Self {
        Self {
            location: None,
            construct: Components(vec![Component::Module(Module {
                module: value,
                next: None,
            })]),
        }
    }
}

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Debug, Clone)]
pub struct Components(Vec<Component>);

impl Components {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Component> {
        self.0.iter()
    }
}

impl<'de> Deserialize<'de> for Components {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <Vec<Component> as de::Deserialize>::deserialize(deserializer).and_then(|inner| {
            if inner.is_empty() {
                return Err(de::Error::invalid_length(
                    inner.len(),
                    &"must contain a module",
                ));
            }
            if !matches!(inner[0], Component::Module(_)) {
                return Err(de::Error::invalid_type(
                    Unexpected::StructVariant,
                    &"first component must be a module",
                ));
            }
            Ok(Components(inner))
        })
    }
}

de_fromstr!(DomainMatch);

/// Return option or generate default duration from seconds
#[inline]
pub fn default_duration(d: &Option<Duration>, default_secs: u64) -> std::time::Duration {
    d.as_ref()
        .map(|d| d.0)
        .unwrap_or_else(|| std::time::Duration::from_secs(default_secs))
}
