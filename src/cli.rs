use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

use crate::config::modules::*;
use crate::config::*;

pub type Config = Vec<ServerConfig>;

/// The greatest of all reverse proxies, and
/// written in 🦀 (so you KNOW ITS GOOD 👌)
#[derive(Debug, Parser)]
pub struct Cli {
    /// Sanitize inputs if enabled
    #[clap(short, long)]
    sanitize: Option<bool>,
    /// Log requests if enabled
    #[clap(short, long, default_value = "true")]
    log: Option<bool>,
    /// Command for bob to run
    #[clap(subcommand)]
    command: Option<Command>,
}

impl TryInto<Config> for Cli {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        let mut config: Config = match self.command.unwrap_or_default() {
            Command::Run(cfg) => cfg.try_into(),
            #[cfg(feature = "fileserver")]
            Command::FileServer(cfg) => cfg.try_into(),
            #[cfg(feature = "fastcgi")]
            Command::Fastcgi(cfg) => cfg.try_into(),
            #[cfg(feature = "rproxy")]
            Command::ReverseProxy(cfg) => cfg.try_into(),
        }?;
        config.iter_mut().for_each(|config| {
            config.sanitize_errors = config.sanitize_errors.or(self.sanitize);
            config.log_requests = self.log;
        });
        Ok(config)
    }
}

#[derive(Debug, Subcommand)]
enum Command {
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
}

impl Default for Command {
    #[inline]
    fn default() -> Self {
        Self::Run(RunCmd::default())
    }
}

#[derive(Args, Debug)]
struct RunCmd {
    /// Path of configuration to load (default: ./config.yaml).
    #[clap(short, long, default_value = "./config.yaml")]
    config: PathBuf,
}

impl Default for RunCmd {
    fn default() -> Self {
        Self {
            config: PathBuf::from("./config.yaml"),
        }
    }
}

impl TryInto<Config> for RunCmd {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        read_config(&self.config)
    }
}

#[cfg(any(feature = "fileserver", feature = "rproxy"))]
#[inline]
fn convert_addr(addr: &str) -> Result<Vec<ListenCfg>, anyhow::Error> {
    use std::net::ToSocketAddrs;
    Ok(addr.to_socket_addrs()?.map(|addr| addr.into()).collect())
}

#[cfg(feature = "fileserver")]
#[derive(Args, Debug)]
struct FileServerCmd {
    /// Toggle directory browsing
    #[clap(short, long, default_value = "true")]
    browse: Option<bool>,
    /// Supported index files when browsing is disabled
    #[clap(short, long, default_value = "index.html")]
    index: Vec<String>,
    /// The address to which to bind the listener
    #[clap(short, long, default_value = "localhost:8000")]
    listen: String,
    /// The path to the root of the site
    #[clap(short, long, default_value = ".")]
    root: PathBuf,
    /// Show hidden files if enabled
    #[clap(short, long)]
    show_hidden: bool,
}

#[cfg(feature = "fileserver")]
impl TryInto<Config> for FileServerCmd {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        Ok(vec![ServerConfig {
            index: self.index,
            listen: convert_addr(&self.listen).context("invalid listen address")?,
            directives: vec![
                ModuleConfig::FileServer(fileserver::Config {
                    root: Some(self.root),
                    hidden_files: self.show_hidden,
                    index_files: self.browse.unwrap_or_default(),
                    async_threshold: None,
                })
                .into(),
            ],
            ..Default::default()
        }])
    }
}

#[cfg(feature = "fastcgi")]
#[derive(Args, Debug)]
struct FastCgiCmd {
    /// FastCGI Connection Address
    connect: String,
    /// Supported index files when accessing directory
    #[clap(short, long, default_value = "index.php")]
    index: Vec<String>,
    /// The address to which to bind the listener
    #[clap(short, long, default_value = "localhost:8000")]
    listen: String,
    /// The path to the root of the site
    #[clap(short, long, default_value = ".")]
    root: PathBuf,
}

#[cfg(feature = "fastcgi")]
impl TryInto<Config> for FastCgiCmd {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        Ok(vec![ServerConfig {
            index: self.index,
            listen: convert_addr(&self.listen).context("invalid listen address")?,
            sanitize_errors: Some(false),
            directives: vec![
                ModuleConfig::FastCGI(fastcgi::Config {
                    connect: self.connect,
                    root: Some(self.root),
                })
                .into(),
            ],
            ..Default::default()
        }])
    }
}

#[cfg(feature = "rproxy")]
#[derive(Args, Debug)]
struct RevProxyCmd {
    /// Set upstream Host header to address of upstream
    #[clap(short, long)]
    change_host_header: bool,
    /// Address used to recieve traffic
    #[clap(short, long, default_value = "localhost:8000")]
    from: String,
    /// Disable TLS verification
    #[clap(long)]
    insecure: bool,
    /// Upstream address to resolve to
    #[clap(short, long)]
    to: crate::config::Uri,
    /// Upstream request timeout.
    #[clap(long, default_value = "5s")]
    timeout: Duration,
    /// Set a response header for downstream
    #[clap(short = 'd', long)]
    header_down: Vec<Header>,
    /// Set a request header for upstream
    #[clap(short = 'u', long)]
    header_up: Vec<Header>,
}

#[cfg(feature = "rproxy")]
#[derive(Clone, Debug)]
struct Header(String, String);

#[cfg(feature = "rproxy")]
impl FromStr for Header {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s.trim().split_once(':').context("invalid header")?;
        Ok(Self(name.trim().to_owned(), value.trim().to_owned()))
    }
}

#[cfg(feature = "rproxy")]
impl TryInto<Config> for RevProxyCmd {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        let downstream = self.header_down.into_iter().map(|h| (h.0, h.1)).collect();
        let upstream = self.header_up.into_iter().map(|h| (h.0, h.1)).collect();
        Ok(vec![ServerConfig {
            listen: convert_addr(&self.from).context("invalid from address")?,
            directives: vec![
                ModuleConfig::ReverseProxy(rproxy::Config {
                    resolve: self.to,
                    timeout: Some(self.timeout),
                    verify_ssl: Some(self.insecure),
                    change_host: self.change_host_header,
                    upstream_headers: upstream,
                    downstream_headers: downstream,
                    max_redirects: None,
                    initial_conn_size: None,
                    initial_window_size: None,
                })
                .into(),
            ],
            ..Default::default()
        }])
    }
}
