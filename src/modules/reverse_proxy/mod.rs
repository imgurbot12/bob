//!

use serde::Deserialize;

mod config;
mod factory;
mod service;
mod utils;

use config::*;

//TODO: add option to add X-Forwarded-For headers

#[derive(Clone, Debug, Deserialize)]
pub struct ReverseProxyConfig {
    resolve: Uri,
    max_redirects: Option<u8>,
    initial_connection_size: Option<u32>,
    initial_window_size: Option<u32>,
    timeout: Option<Duration>,
}

impl ReverseProxyConfig {
    pub fn into_factory(&self) -> factory::ReverseProxy {
        let client = awc::ClientBuilder::new()
            .initial_connection_window_size(self.initial_connection_size.unwrap_or(u16::MAX as u32))
            .initial_window_size(self.initial_window_size.unwrap_or(u16::MAX as u32))
            .max_redirects(self.max_redirects.unwrap_or(10))
            .timeout(
                self.timeout
                    .clone()
                    .map(|d| d.0)
                    .unwrap_or_else(|| Duration::from_secs(5)),
            )
            .finish();
        factory::ReverseProxy::new("", client, self.resolve.0.clone())
    }
}
