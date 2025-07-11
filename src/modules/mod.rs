use serde::Deserialize;

use crate::config::{Config, DirectiveCfg};

mod factory;
mod guard;
mod payload;
mod service;
mod utils;

use guard::*;

#[cfg(feature = "fs")]
mod file_server;

#[cfg(feature = "rev_proxy")]
mod reverse_proxy;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "module", deny_unknown_fields)]
pub enum ModulesConfig {
    #[cfg(feature = "fs")]
    #[serde(alias = "file_server")]
    FileServer(file_server::FileServerConfig),
    #[cfg(feature = "rev_proxy")]
    #[serde(alias = "rev_proxy")]
    ReverseProxy(reverse_proxy::ReverseProxyConfig),
}

impl ModulesConfig {
    fn add_service(&self, svc: &mut factory::ModuleSvc, cfg: &Config, dir: &DirectiveCfg) {
        let loc = LocationMatches::new(dir.locations());
        match self {
            #[cfg(feature = "fs")]
            Self::FileServer(config) => {
                let mut factory = config.into_factory(cfg);
                factory.add_location(loc);
                svc.add_module(factory)
            }
            #[cfg(feature = "rev_proxy")]
            Self::ReverseProxy(config) => {
                let mut factory = config.into_factory();
                factory.add_location(loc);
                svc.add_module(factory);
            }
        }
    }
}

pub fn build_modules(cfg: &Config) -> factory::ModuleSvc {
    let mut svc = factory::ModuleSvc::new("");
    if !cfg.server_name.is_empty() {
        let guard = GlobHostGuards::new(&cfg.server_name);
        svc.add_guard(guard);
    }
    // add submodules to module-svc for each directive
    for dir in cfg.directives.iter() {
        for module in dir.modules.iter() {
            module.add_service(&mut svc, cfg, dir);
        }
    }
    svc
}
