use actix_chain::Wrappable;
use serde::Deserialize;

use super::Spec;

/// Server specific middleware configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Middleware {
    /// Configuration for [`actix_authn::basic::BasicAuthSession`] Middleware.
    #[cfg(feature = "authn")]
    #[serde(alias = "basic_auth")]
    auth_basic: Option<auth_basic::Config>,
    /// Configuration for [`actix_authn::basic::BasicAuthSession`] Middleware.
    #[cfg(feature = "authn")]
    #[serde(alias = "basic_auth_session")]
    auth_session: Option<auth_session::Config>,
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
                Some(attr) => attr.wrap(wrap, spec),
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
    impl_init!(auth_basic, "authn");
    impl_init!(auth_session, "authn");
    impl_init!(modsecurity, "modsecurity");
    impl_init!(rewrite, "rewrite");

    /// Wrap Chain/Link in all of the established middleware.
    pub fn wrap<W: Wrappable>(&self, mut wrap: W, spec: &Spec) -> W {
        wrap = self.modsecurity(wrap, spec);
        wrap = self.auth_basic(wrap, spec);
        wrap = self.auth_session(wrap, spec);
        wrap = self.rewrite(wrap, spec);
        wrap
    }
}

#[cfg(feature = "authn")]
mod auth_basic {
    use std::{fmt::Debug, path::PathBuf};

    use super::*;
    use actix_authn::{
        Authn,
        basic::{Basic, BasicAuth},
    };

    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Cache size linked to authentication lookup
        cache_size: Option<usize>,
        /// Htpasswd filepaths to load credentials from.
        htpasswd: Vec<PathBuf>,
    }

    impl Config {
        /// Produce [`actix_authn::Authn`] from config.
        pub fn factory(&self, _spec: &Spec) -> Authn<BasicAuth> {
            let mut auth =
                Basic::default().cache_size(self.cache_size.unwrap_or(u16::MAX as usize));
            auth = self
                .htpasswd
                .iter()
                .fold(auth, |auth, path| auth.htpasswd(path));
            Authn::new(auth.build())
        }

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            w.wrap_with(self.factory(spec))
        }
    }
}

#[cfg(feature = "authn")]
mod auth_session {
    use std::{fmt::Debug, path::PathBuf};

    use super::*;
    use actix_authn::{
        Authn,
        basic::{Basic, BasicAuthSession},
    };
    use actix_session::config::BrowserSession;
    use actix_web::cookie::Key;

    /// Derivation wrapper around [`actix_web::cookie::Key`]
    #[derive(Clone)]
    struct CookieKey(Key);

    impl Debug for CookieKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "CookieKey {{}}")
        }
    }

    impl Default for CookieKey {
        fn default() -> Self {
            Self(Key::generate())
        }
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Htpasswd filepaths to load credentials from.
        htpasswd: Vec<PathBuf>,
        /// Cookie name associated with session.
        cookie_name: Option<String>,
        /// Cache size linked to authentication lookup
        cache_size: Option<usize>,

        // global initialization for cookie-key via config.
        // avoids recreating the key for every worker actix-web creates.
        #[serde(default, skip)]
        key: CookieKey,
    }

    impl Config {
        /// Produce [`actix_authn::Authn`] from config.
        pub fn factory(&self, _spec: &Spec) -> Authn<BasicAuthSession> {
            let mut auth =
                Basic::default().cache_size(self.cache_size.unwrap_or(u16::MAX as usize));
            auth = self
                .htpasswd
                .iter()
                .fold(auth, |auth, path| auth.htpasswd(path));
            Authn::new(auth.build_session())
        }

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            use actix_session::SessionMiddleware;
            use actix_session::config::SessionLifecycle;
            use actix_session::storage::CookieSessionStore;
            use actix_web::cookie::time::Duration;

            let cookie_name = self
                .cookie_name
                .clone()
                .unwrap_or_else(|| "authn".to_owned());
            let lifecycle = SessionLifecycle::BrowserSession(
                BrowserSession::default().state_ttl(Duration::HOUR * 24),
            );

            let store = CookieSessionStore::default();
            let session = SessionMiddleware::builder(store, self.key.0.clone())
                .cookie_name(cookie_name)
                .session_lifecycle(lifecycle)
                .build();
            w.wrap_with(self.factory(spec)).wrap_with(session)
        }
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

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            w.wrap_with(self.factory(spec))
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

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            w.wrap_with(self.factory(spec))
        }
    }
}
