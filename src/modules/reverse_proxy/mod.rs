//!

use serde::Deserialize;

mod factory;
mod service;

#[derive(Clone, Debug, Deserialize)]
pub struct ReverseProxyConfig {
    resolve: Vec<String>,
}

impl ReverseProxyConfig {
    pub fn into_factory(&self) -> factory::ReverseProxy {
        factory::ReverseProxy::new("")
    }
}
