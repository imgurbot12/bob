use actix_chain::Wrappable;
use serde::Deserialize;

use super::Spec;

/// Server specific middleware configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Middleware {
    /// Configuration for [`actix_modsecurity`] Middleware.
    #[cfg(feature = "modsecurity")]
    #[serde(alias = "modsecurity")]
    modsecurity: Option<modsecurity::Config>,
    /// Configuration for [`actix_rewrite`] Middleware.
    #[cfg(feature = "rewrite")]
    #[serde(alias = "rewrite")]
    rewrite: Option<rewrite::Config>,
}

macro_rules! impl_init {
    ($attr:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        #[inline]
        #[doc = concat!("Wrap an existing link with ", $feature, " middleware.")]
        pub fn $attr<W: Wrappable>(&self, wrap: W, spec: &Spec) -> W {
            match self.$attr.as_ref() {
                Some(attr) => wrap.wrap_with(attr.factory(spec)),
                None => wrap,
            }
        }
        #[cfg(not(feature = $feature))]
        #[inline]
        #[doc = concat!("Identity function for disabled middleware: ", $feature)]
        pub fn $attr<W: Wrappable>(&self, wrap: W, _spec: &Spec) -> W {
            wrap
        }
    };
}

impl Middleware {
    impl_init!(modsecurity, "modsecurity");
    impl_init!(rewrite, "rewrite");

    /// Wrap Chain/Link in all of the established middleware.
    pub fn wrap<W: Wrappable>(&self, mut wrap: W, spec: &Spec) -> W {
        wrap = self.modsecurity(wrap, &spec);
        wrap = self.rewrite(wrap, &spec);
        wrap
    }
}

#[cfg(feature = "modsecurity")]
mod modsecurity {
    use std::path::PathBuf;

    use super::*;
    use actix_modsecurity::{Middleware, ModSecurity};

    /// Modsecurity middleware configuration.
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        /// Plaintext rules contained within a single string.
        ///
        /// See [`actix_modsecurity::ModSecurity::add_rules`] for more info.
        rules: Option<String>,
        /// List of additional files to load rules from.
        rule_files: Vec<PathBuf>,
        /// Max request body size allowed to be read into memory for scanning.
        max_request_body_size: Option<usize>,
        /// Max response body size allowed to be read into memory for scanning.
        max_response_body_size: Option<usize>,
    }

    impl Config {
        /// Produce [`actix_modsecurity::Middleware`] from config.
        pub fn factory(&self, _spec: &Spec) -> Middleware {
            let modsec = ModSecurity::builder()
                .max_request_size(self.max_request_body_size)
                .max_response_size(self.max_response_body_size)
                .rules(&self.rules.clone().unwrap_or_default())
                .expect("failed load rules");
            self.rule_files
                .iter()
                .try_fold(modsec, |msec, path| msec.rules_file(path))
                .expect("failed to load rules file")
                .into()
        }
    }
}

#[cfg(feature = "rewrite")]
mod rewrite {
    use std::path::PathBuf;

    use super::*;
    use actix_rewrite::{Engine, Middleware, ServerCtx};

    const SERVER_SOFTWARE: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

    /// `mod_rewrite` middleware configuration.
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        /// Plaintext rules contained within a single string.
        ///
        /// See [`actix_rewrite::Engine::add_rules`] for more info.
        rules: Option<String>,
        /// List of additional files to load rules from.
        rule_files: Vec<PathBuf>,
        /// Max number of iterations allowed for looping rulesets.
        ///
        /// Default is 10.
        max_iterations: Option<usize>,
    }

    impl Config {
        /// Produce [`actix_rewrite::Middleware`] from config.
        pub fn factory(&self, spec: &Spec) -> Middleware {
            let root = spec
                .config
                .root
                .clone()
                .and_then(|s| s.to_str().map(|s| s.to_owned()))
                .unwrap_or_default();
            let ctx = ServerCtx::default()
                .document_root(root)
                .server_software(SERVER_SOFTWARE);
            let rewrite = Engine::new()
                .server_context(ctx)
                .rules(&self.rules.clone().unwrap_or_default())
                .expect("failed to load rules");
            self.rule_files
                .iter()
                .try_fold(rewrite, |rw, path| rw.rules_file(path))
                .expect("failed to load rules file")
                .middleware()
        }
    }
}
