//! Actix Service Implementation for File Server

use std::{ops::Deref, rc::Rc};

use actix_web::{
    FromRequest,
    body::{self, BoxBody},
    dev::{self, Service, ServiceRequest, ServiceResponse},
    error::Error,
    guard::Guard,
};
use futures_core::future::LocalBoxFuture;

use crate::modules::guard::Location;
use crate::modules::utils::{check_guards, check_locations, default_response};

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
        // skip processing if locations/guards do not match
        let ctx = req.guard_ctx();
        let url_path = check_locations!(req, &ctx, self.locations);
        check_guards!(req, &ctx, self.guards);

        let this = self.clone();
        Box::pin(async move {
            println!("rev_proxy {url_path:?}");

            let (req, mut payload) = req.into_parts();
            let pl = actix_web::web::Payload::from_request(&req, &mut payload)
                .await
                .unwrap();

            let content = pl.to_bytes().await;
            println!("content: {content:?}");

            let req = ServiceRequest::from_parts(req, payload);
            Ok(default_response(req))
        })
    }
}
