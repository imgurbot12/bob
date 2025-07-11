use serde::Deserialize;

mod payload;

#[cfg(feature = "mod_security")]
pub mod modsecurity;

macro_rules! impl_init {
    ($attr:ident, $feature:literal, $type:ty, $default:expr) => {
        #[cfg(feature = $feature)]
        pub fn $attr(&self) -> actix_web::middleware::Condition<$type> {
            match self.$attr.as_ref() {
                Some(attr) => actix_web::middleware::Condition::new(true, attr.clone()),
                None => actix_web::middleware::Condition::new(false, $default),
            }
        }
        #[cfg(not(feature = $feature))]
        pub fn $attr(&self) -> actix_web::middleware::Identity {
            actix_web::middleware::Identity::default()
        }
    };
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiddlewareConfig {
    #[cfg(feature = "mod_security")]
    #[serde(alias = "modsecurity")]
    modsecurity: Option<modsecurity::ModSecurity>,
}

impl MiddlewareConfig {
    impl_init!(
        modsecurity,
        "mod_security",
        modsecurity::ModSecurity,
        modsecurity::ModSecurity::default()
    );
}
