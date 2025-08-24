//! CLI actions and [`Config`] compilation

use anyhow::{Context, Result};
use bob_cli::*;

use crate::config::modules::*;
use crate::config::*;

/// Compilation of [`ServerConfig`] instances
pub type Config = Vec<ServerConfig>;

macro_rules! run_and_exit {
    ($fn:expr) => {{
        $fn?;
        std::process::exit(0);
    }};
}

/// Build configuration or run command based on cli settings.
pub fn build_config(cli: Cli) -> Result<Config> {
    let mut config: Config = match cli.command.unwrap_or_default() {
        Command::Run(cfg) => run_cmd(cfg),
        #[cfg(feature = "fileserver")]
        Command::FileServer(cfg) => fileserver_cmd(cfg),
        #[cfg(feature = "fastcgi")]
        Command::Fastcgi(cfg) => fastcgi_cmd(cfg),
        #[cfg(feature = "rproxy")]
        Command::ReverseProxy(cfg) => rproxy_cmd(cfg),
        #[cfg(feature = "authn")]
        Command::Passwd(cfg) => run_and_exit!(execute_passwd(cfg)),
        #[cfg(feature = "schema")]
        Command::Schema(cfg) => run_and_exit!(build_schema(cfg)),
    }?;
    config.iter_mut().for_each(|config| {
        config.sanitize_errors = config.sanitize_errors.or(cli.sanitize);
        config.logging.disable = cli.log.map(|b| !b).unwrap_or_default();
    });
    Ok(config)
}

/// Read config specified in [`RunCmd`]
fn run_cmd(cmd: RunCmd) -> Result<Config> {
    read_config(&cmd.config)
}

/// Convert string into [`Vec<ListenCfg>`]
#[cfg(any(feature = "fileserver", feature = "rproxy"))]
#[inline]
fn convert_addr(addr: &str) -> Result<Vec<ListenCfg>> {
    use std::net::ToSocketAddrs;
    Ok(addr.to_socket_addrs()?.map(|addr| addr.into()).collect())
}

/// Run password hash generation and exit.
#[cfg(feature = "authn")]
fn execute_passwd(cmd: GenPasswdCmd) -> Result<()> {
    use actix_authn::basic::crypt::bcrypt;
    use rpassword::prompt_password;
    use std::io::Write;

    let password = if let Some(password) = cmd.password {
        password
    } else {
        let password = prompt_password("Password: ").context("failed to read password")?;
        let confirm =
            prompt_password("Confirm Password: ").context("failed to confirm password")?;
        if password != confirm {
            return Err(anyhow::anyhow!("passwords do not match"));
        }
        password
    };

    let passwd = bcrypt::hash(password).context("failed to hash password")?;
    let passwd = format!("{}:{}", cmd.username, passwd.as_str());
    match cmd.output {
        Some(output) => std::fs::write(output, passwd).context("failed to write password")?,
        None => {
            std::io::stdout()
                .write(passwd.as_bytes())
                .context("failed to write stdout")?;
        }
    };
    Ok(())
}

/// Build JSON schema for configuration
#[cfg(feature = "schema")]
fn build_schema(cmd: SchemaCmd) -> Result<()> {
    use std::io::Write;

    let schema = schemars::schema_for!(Config);
    let data = serde_json::to_string_pretty(&schema)?;
    let mut file = std::fs::File::create(cmd.output)?;
    write!(file, "{data}").context("failed to write schema")?;
    Ok(())
}

/// Fileserver config generation
#[cfg(feature = "fileserver")]
fn fileserver_cmd(cmd: FileServerCmd) -> Result<Config> {
    if cmd.open {
        let _ = open::that(format!("http://{}", cmd.listen))
            .inspect_err(|err| log::error!("failed to open browser: {err:?}"));
    }
    Ok(vec![ServerConfig {
        index: cmd.index,
        listen: convert_addr(&cmd.listen).context("invalid listen address")?,
        directives: vec![
            ModuleConfig::FileServer(fileserver::Config {
                root: Some(cmd.root),
                hidden_files: cmd.show_hidden,
                index_files: cmd.browse.unwrap_or_default(),
                async_threshold: None,
            })
            .into(),
        ],
        ..Default::default()
    }])
}

/// FastCGI config generation
#[cfg(feature = "fastcgi")]
fn fastcgi_cmd(cmd: FastCgiCmd) -> Result<Config> {
    Ok(vec![ServerConfig {
        index: cmd.index,
        listen: convert_addr(&cmd.listen).context("invalid listen address")?,
        sanitize_errors: Some(false),
        directives: vec![
            ModuleConfig::FastCGI(fastcgi::Config {
                connect: cmd.connect,
                root: Some(cmd.root),
            })
            .into(),
        ],
        ..Default::default()
    }])
}

/// Reverse-Proxy config generation
#[cfg(feature = "rproxy")]
fn rproxy_cmd(cmd: RevProxyCmd) -> Result<Config> {
    if cmd.open {
        let _ = open::that(format!("http://{}", cmd.from))
            .inspect_err(|err| log::error!("failed to open browser: {err:?}"));
    }
    let downstream = cmd.header_down.into_iter().map(|h| (h.0, h.1)).collect();
    let upstream = cmd.header_up.into_iter().map(|h| (h.0, h.1)).collect();
    Ok(vec![ServerConfig {
        listen: convert_addr(&cmd.from).context("invalid from address")?,
        directives: vec![
            ModuleConfig::ReverseProxy(rproxy::Config {
                resolve: cmd.to,
                timeout: Some(cmd.timeout),
                verify_ssl: Some(cmd.insecure),
                change_host: cmd.change_host_header,
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
