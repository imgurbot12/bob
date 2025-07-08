//! Custom Actix-Web Guard Implementations

//!

use actix_web::{
    dev::RequestHead,
    guard::Guard,
    http::{Version, header},
};

use crate::config::DomainMatch;

pub(crate) trait Location {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> Option<String>;
}

#[inline]
fn get_host_uri(req: &RequestHead) -> Option<&str> {
    req.headers
        .get(header::HOST)
        .and_then(|host_value| host_value.to_str().ok())
        .filter(|_| req.version < Version::HTTP_2)
        .or_else(|| req.uri.host())
}

pub struct GlobHostGuards(Vec<glob::Pattern>);

impl GlobHostGuards {
    pub fn new(domains: &Vec<DomainMatch>) -> Self {
        Self(domains.iter().map(|d| d.0.clone()).collect())
    }
}

impl Guard for GlobHostGuards {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        match get_host_uri(ctx.head()) {
            Some(host) => self.0.iter().any(|g| g.matches(host)),
            None => false,
        }
    }
}

pub struct LocationMatches(Vec<String>);

impl LocationMatches {
    pub fn new(locations: Vec<String>) -> Self {
        Self(locations)
    }
}

impl Location for LocationMatches {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> Option<String> {
        let path = ctx.head().uri.path();
        self.0
            .iter()
            .find(|prefix| path.starts_with(*prefix))
            .map(|prefix| {
                path.trim_start_matches(prefix)
                    .trim_start_matches('/')
                    .to_owned()
            })
    }
}
