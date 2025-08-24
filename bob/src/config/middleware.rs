//! Middleware Configuration

#[cfg(feature = "schema")]
use schemars::JsonSchema;

use actix_chain::Wrappable;
use serde::Deserialize;

use super::Spec;

/// Middleware configuration for request processing.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "middleware", deny_unknown_fields)]
pub enum Middleware {
    /// Configuration for [`actix_authn::basic::BasicAuthSession`] Middleware.
    #[cfg(feature = "authn")]
    #[serde(alias = "basic_auth")]
    AuthBasic(auth_basic::Config),
    /// Configuration for [`actix_authn::basic::BasicAuthSession`] Middleware.
    #[cfg(feature = "authn")]
    #[serde(alias = "basic_auth_session")]
    AuthSession(auth_session::Config),
    /// Configuration for [`actix_ipware`] Middleware.
    #[cfg(feature = "ipware")]
    #[serde(alias = "ipware")]
    Ipware(ipware::Config),
    /// Configuration for [`actix_ip_filter`] Middleware.
    #[cfg(feature = "ipfilter")]
    #[serde(alias = "filter")]
    Ipfilter(ipfilter::Config),
    /// Configuration for [`actix_modsecurity`] Middleware.
    #[cfg(feature = "modsecurity")]
    #[serde(alias = "modsecurity")]
    ModSecurity(modsecurity::Config),
    /// Configuration for [`actix_rewrite`] Middleware.
    #[cfg(feature = "rewrite")]
    #[serde(alias = "rewrite")]
    Rewrite(rewrite::Config),
    /// Configuration for [`actix_extensible_rate_limit`] Middleware
    #[cfg(feature = "ratelimit")]
    #[serde(alias = "ratelimit")]
    Ratelimit(ratelimit::Config),
    /// Configuration for [`actix_timeout`] Middleware
    #[cfg(feature = "timeout")]
    #[serde(alias = "timeout")]
    Timeout(timeout::Config),
}

impl Middleware {
    /// Wrap Chain/Link in all of the established middleware.
    pub fn wrap<W: Wrappable>(&self, wrap: W, spec: &Spec) -> W {
        match self {
            #[cfg(feature = "authn")]
            Self::AuthBasic(config) => config.wrap(wrap, spec),
            #[cfg(feature = "authn")]
            Self::AuthSession(config) => config.wrap(wrap, spec),
            #[cfg(feature = "ipware")]
            Self::Ipware(config) => config.wrap(wrap, spec),
            #[cfg(feature = "ipfilter")]
            Self::Ipfilter(config) => config.wrap(wrap, spec),
            #[cfg(feature = "modsecurity")]
            Self::ModSecurity(config) => config.wrap(wrap, spec),
            #[cfg(feature = "rewrite")]
            Self::Rewrite(config) => config.wrap(wrap, spec),
            #[cfg(feature = "ratelimit")]
            Self::Ratelimit(config) => config.wrap(wrap, spec),
            #[cfg(feature = "timeout")]
            Self::Timeout(config) => config.wrap(wrap, spec),
        }
    }
}

/// HTTP Basic Authorization Middleware
#[cfg(feature = "authn")]
mod auth_basic {
    use std::{fmt::Debug, path::PathBuf};

    use super::*;
    use actix_authn::{
        Authn,
        basic::{Basic, BasicAuth},
    };

    #[cfg_attr(feature = "schema", derive(JsonSchema))]
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

/// HTTP Basic Authorization with Cookie Session Middleware
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

    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Htpasswd filepaths to load credentials from.
        htpasswd: Vec<PathBuf>,
        /// Cookie name associated with session.
        cookie_name: Option<String>,
        /// Cache size linked to authentication lookup
        ///
        /// Default is u16::MAX
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

/// IpWare Client-IP Translation Middleware.
#[cfg(feature = "ipware")]
mod ipware {
    use std::str::FromStr;

    use super::*;
    use actix_ipware::{IpWare, Middleware};
    use actix_web::http::header::HeaderName;

    /// IpWare middleware configuration.
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        /// Allow fake/broken ips in trusted headers if false.
        ///
        /// Default is true
        strict: Option<bool>,
        /// Trusted headers to parse client IP address from.
        trusted_headers: Vec<String>,
        /// Number of expected proxy jumps to be trusted.
        proxy_count: Option<u16>,
        /// List of trusted upstream proxy globs.
        trusted_proxies: Vec<String>,
        /// Allow untrusted client IP assignments.
        ///
        /// Default is false
        allow_untrusted: bool,
    }

    impl Config {
        /// Produce [`actix_ipware::Middleware`] from config.
        pub fn factory(&self, _spec: &Spec) -> Middleware {
            let mut ipware = IpWare::empty();
            self.trusted_headers
                .iter()
                .filter_map(|header| HeaderName::from_str(header).ok())
                .fold(&mut ipware, |ipw, header| ipw.trust_header(header));
            self.trusted_proxies
                .iter()
                .fold(&mut ipware, |ipw, proxy| ipw.trust_proxy(proxy));
            ipware.proxy_count(self.proxy_count);
            Middleware::new(ipware)
                .strict(self.strict.unwrap_or(true))
                .allow_untrusted(self.allow_untrusted)
        }

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            w.wrap_with(self.factory(spec))
        }
    }
}

/// IpFilter IP Whitelist/Blacklist Middleware.
///
/// It's highly recomended to use this middleware
/// in conjunction with [`ipware`].
#[cfg(feature = "ipfilter")]
mod ipfilter {
    use super::*;
    use actix_ip_filter::IPFilter;

    /// IP Filter middleware configuration.
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        /// Always allowed whitelist of IP Globs.
        #[serde(alias = "allow")]
        whitelist: Vec<String>,
        /// Always denied blacklist of IP Globs.
        #[serde(alias = "block", alias = "deny")]
        blacklist: Vec<String>,
    }

    impl Config {
        /// Produce [`actix_ip_filter::IPFilter`] from config.
        pub fn factory(&self, _spec: &Spec) -> IPFilter {
            IPFilter::new()
                .allow(self.whitelist.iter().map(|s| s.as_str()).collect())
                .block(self.blacklist.iter().map(|s| s.as_str()).collect())
        }

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            w.wrap_with(self.factory(spec))
        }
    }
}

/// OWASP ModSecurity Middleware
#[cfg(feature = "modsecurity")]
mod modsecurity {
    use std::path::PathBuf;

    use super::*;
    use actix_modsecurity::{Middleware, ModSecurity};

    /// Modsecurity middleware configuration.
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
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

/// Apache2 Inspired `mod_rewrite` module
#[cfg(feature = "rewrite")]
mod rewrite {
    use std::path::PathBuf;

    use super::*;
    use actix_rewrite::{Engine, Middleware, ServerCtx};

    const SERVER_SOFTWARE: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

    /// `mod_rewrite` middleware configuration.
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
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

/// Ratelimitting controls middleware.
#[cfg(feature = "ratelimit")]
mod ratelimit {
    use std::fmt::Debug;

    use super::*;
    use crate::config::default_duration;

    use actix_extensible_rate_limit::{
        RateLimiter,
        backend::{SimpleInputFunctionBuilder, memory::InMemoryBackend},
    };
    use bob_cli::Duration;

    /// Derivation wrapper around [`InMemoryBackend`]
    #[derive(Clone)]
    struct MemoryBackend(InMemoryBackend);

    impl Debug for MemoryBackend {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "MemoryBackend {{}}")
        }
    }

    impl Default for MemoryBackend {
        fn default() -> Self {
            Self(InMemoryBackend::builder().build())
        }
    }

    /// Ratelimitter middleware configuration.
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        /// Request limit
        limit: u64,
        /// Ratelimit control period
        ///
        /// Default is 1s
        #[serde(default)]
        period: Option<Duration>,
        /// Discriminate ratelimit by IP and Path if enabled
        ///
        /// Default is false
        #[serde(default)]
        use_path: bool,
        /// Allow request by default if backend fails to respond in time
        ///
        /// Default is false
        #[serde(default)]
        fail_open: bool,
        /// Include ratelimit explanation headers if enabled
        ///
        /// Default is false
        #[serde(default)]
        response_headers: bool,

        // global initialization for ratelimit backend.
        // avoids recreating the backend for every worker actix-web creates.
        #[serde(default, skip)]
        backend: MemoryBackend,
    }

    impl Config {
        // ratelimiter generics make it annoying to export as a type
        // from a function cause they cause type errors when passing it
        // into `wrap_with`. instead we go directly to wrap with builder
        // to avoid that nonsense.

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, _spec: &Spec) -> W {
            let period = default_duration(&self.period, 1);
            let mut input = SimpleInputFunctionBuilder::new(period, self.limit).peer_ip_key();
            if self.use_path {
                input = input.path_key();
            }

            let mut middleware = RateLimiter::builder(self.backend.0.clone(), input.build())
                .fail_open(self.fail_open);
            if self.response_headers {
                middleware = middleware.add_headers();
            }

            w.wrap_with(middleware.build())
        }
    }
}

/// Processing Timeout Middleware.
#[cfg(feature = "timeout")]
mod timeout {

    use super::*;
    use actix_timeout::Timeout;

    /// Timeout middleware configuration.
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub struct Config {
        /// Timeout duration in miliseconds
        duration: u64,
    }

    impl Config {
        /// Produce [`actix_timeout::Timeout`] from config.
        pub fn factory(&self, _spec: &Spec) -> Timeout {
            Timeout::from_millis(self.duration)
        }

        /// Wrap Chain/Link with configured middleware.
        pub fn wrap<W: Wrappable>(&self, w: W, spec: &Spec) -> W {
            w.wrap_with(self.factory(spec))
        }
    }
}
