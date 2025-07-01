//! Available Modules for Bob

use anyhow::Result;
use axum::Router;

use crate::config::{Config, DirectiveCfg};

#[cfg(feature = "fs")]
mod file_server;

#[cfg(feature = "rev_proxy")]
mod rev_proxy;

#[cfg(feature = "fs")]
pub use file_server::FSModule;

#[cfg(feature = "rev_proxy")]
pub use rev_proxy::RevProxyModule;

pub trait Module {
    fn enable(&self, cfg: &Config, dir: &DirectiveCfg, router: Router) -> Result<Router>;
}
