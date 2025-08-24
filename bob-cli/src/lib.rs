use std::{path::PathBuf, str::FromStr};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

use clap::{Args, Parser, Subcommand};
use serde::de::Error;

/// The greatest of all reverse proxies, and
/// written in 🦀 (so you KNOW ITS GOOD 👌)
#[derive(Debug, Parser)]
#[clap(name = "bob", author = "Andrew Scott <imgurbot12@gmail.com>")]
pub struct Cli {
    /// Sanitize inputs if enabled
    #[clap(short, long)]
    pub sanitize: Option<bool>,
    /// Log requests if enabled
    #[clap(short, long, default_value = "true")]
    pub log: Option<bool>,
    /// Command for bob to run
    #[clap(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Starts Bob and blocks indefinitely
    Run(RunCmd),
    /// A simple file server
    #[cfg(feature = "fileserver")]
    FileServer(FileServerCmd),
    /// A simple fastcgi client server
    #[cfg(feature = "fastcgi")]
    Fastcgi(FastCgiCmd),
    /// A quick reverse proxy
    #[cfg(feature = "rproxy")]
    ReverseProxy(RevProxyCmd),
    /// Generate a hashed password for basic-auth
    #[cfg(feature = "authn")]
    Passwd(GenPasswdCmd),
    /// Generate json schema for documentation
    #[cfg(feature = "schema")]
    Schema(SchemaCmd),
}

impl Default for Command {
    #[inline]
    fn default() -> Self {
        Self::Run(RunCmd::default())
    }
}

#[derive(Args, Debug)]
pub struct RunCmd {
    /// Path of configuration to load (default: ./config.yaml).
    #[clap(short, long, default_value = "./config.yaml")]
    pub config: PathBuf,
}

impl Default for RunCmd {
    fn default() -> Self {
        Self {
            config: PathBuf::from("./config.yaml"),
        }
    }
}

#[cfg(feature = "schema")]
#[derive(Args, Debug)]
pub struct SchemaCmd {
    #[clap(short, long, default_value = "schema.json")]
    pub output: PathBuf,
}

#[cfg(feature = "authn")]
#[derive(Args, Debug)]
pub struct GenPasswdCmd {
    /// Username to attach to passwd record
    pub username: String,
    /// Password to apply to passwd generation
    #[clap(short, long)]
    pub password: Option<String>,
    /// Output for passwd generation
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

#[cfg(feature = "fastcgi")]
#[derive(Args, Debug)]
pub struct FastCgiCmd {
    /// FastCGI Connection Address
    pub connect: String,
    /// Supported index files when accessing directory
    #[clap(short, long, default_value = "index.php")]
    pub index: Vec<String>,
    /// The address to which to bind the listener
    #[clap(short, long, default_value = "localhost:8000")]
    pub listen: String,
    /// The path to the root of the site
    #[clap(short, long, default_value = ".")]
    pub root: PathBuf,
}

#[cfg(feature = "fileserver")]
#[derive(Args, Debug)]
pub struct FileServerCmd {
    /// Toggle directory browsing
    #[clap(short, long, default_value = "true")]
    pub browse: Option<bool>,
    /// Supported index files when browsing is disabled
    #[clap(short, long, default_value = "index.html")]
    pub index: Vec<String>,
    /// The address to which to bind the listener
    #[clap(short, long, default_value = "localhost:8000")]
    pub listen: String,
    /// The path to the root of the site
    #[clap(short, long, default_value = ".")]
    pub root: PathBuf,
    /// Show hidden files if enabled
    #[clap(short, long)]
    pub show_hidden: bool,
    /// Open server in browser
    #[clap(long)]
    pub open: bool,
}

#[cfg(feature = "rproxy")]
#[derive(Args, Debug)]
pub struct RevProxyCmd {
    /// Set upstream Host header to address of upstream
    #[clap(short, long)]
    pub change_host_header: bool,
    /// Address used to recieve traffic
    #[clap(short, long, default_value = "localhost:8000")]
    pub from: String,
    /// Disable TLS verification
    #[clap(long)]
    pub insecure: bool,
    /// Upstream address to resolve to
    #[clap(short, long)]
    pub to: Uri,
    /// Upstream request timeout.
    #[clap(long, default_value = "5s")]
    pub timeout: Duration,
    /// Set a response header for downstream
    #[clap(short = 'd', long)]
    pub header_down: Vec<Header>,
    /// Set a request header for upstream
    #[clap(short = 'u', long)]
    pub header_up: Vec<Header>,
    /// Open server in browser
    #[clap(long)]
    pub open: bool,
}

/// Header key/value pair parsed from a string
#[cfg(feature = "rproxy")]
#[derive(Clone, Debug)]
pub struct Header(pub String, pub String);

#[cfg(feature = "rproxy")]
impl FromStr for Header {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s
            .trim()
            .split_once(':')
            .ok_or(std::io::Error::other("invalid header"))?;
        Ok(Self(name.trim().to_owned(), value.trim().to_owned()))
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for Header {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Header".into()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::Header").into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// Time duration parsed from human-readable format.
///
/// Example: `1h5m2s`
#[derive(Clone, Debug)]
pub struct Duration(pub std::time::Duration);

impl FromStr for Duration {
    type Err = humantime::DurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(humantime::parse_duration(s)?))
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for Duration {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Duration".into()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::Duration").into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

/// Resource URI object and parser.
#[derive(Clone, Debug)]
pub struct Uri(pub actix_http::Uri);

impl FromStr for Uri {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            actix_http::Uri::from_str(s).map_err(|e| e.to_string())?,
        ))
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for Uri {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Uri".into()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::Uri").into()
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

#[macro_export]
macro_rules! de_fromstr {
    ($s:ident) => {
        impl<'de> serde::Deserialize<'de> for $s {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                $s::from_str(&s).map_err(D::Error::custom)
            }
        }
    };
}

de_fromstr!(Duration);
de_fromstr!(Uri);
