//! Actix Service Implementation for File Server

use std::{ops::Deref, rc::Rc};

use actix_web::{
    body::BoxBody,
    dev::{self, Service, ServiceRequest, ServiceResponse},
    error::Error,
    guard::Guard,
};
use futures_core::future::LocalBoxFuture;

use crate::modules::{guard::Location, utils::default_response};

#[derive(Clone)]
pub struct ProxyService(pub(crate) Rc<ProxyServiceInner>);

impl Deref for ProxyService {
    type Target = ProxyServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ProxyServiceInner {
    pub(crate) guards: Vec<Rc<dyn Guard>>,
    pub(crate) locations: Vec<Rc<dyn Location>>,
}

impl Service<ServiceRequest> for ProxyService {
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::always_ready!();

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let ctx = req.guard_ctx();
        let allow = self.guards.is_empty() || self.guards.iter().any(|g| (**g).check(&ctx));
        let location = self.locations.iter().find_map(|l| (**l).check(&ctx));

        let this = self.clone();
        Box::pin(async move {
            if !allow {
                return Ok(default_response(req));
            }
            let url_path = match location {
                Some(loc) => loc,
                None if this.locations.is_empty() => req.path().to_owned(),
                None => return Ok(default_response(req)),
            };

            println!("rev_proxy {url_path:?}");
            Ok(default_response(req))
        })
    }
}
