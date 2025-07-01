//! Reverse Proxy Module

// use axum::body::Body;
// use hyper_util::client::legacy::connect::HttpConnector;
//
// type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;
//

use serde::Deserialize;

use super::Module;

#[derive(Debug, Clone, Deserialize)]
pub struct RevProxyModule {}

impl Module for RevProxyModule {
    fn enable(
        &self,
        cfg: &crate::config::Config,
        dir: &crate::config::DirectiveCfg,
        router: axum::Router,
    ) -> anyhow::Result<axum::Router> {
        todo!("")
    }
}
