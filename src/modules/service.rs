//! Actix Service Implementation for Reverse Proxy

use std::{ops::Deref, rc::Rc};

use actix_service::boxed::{BoxService, BoxServiceFactory};
use actix_web::{
    HttpResponse,
    body::BoxBody,
    dev::{self, Payload, Service, ServiceRequest, ServiceResponse},
    error::Error,
    http::{StatusCode, header},
    mime,
};
use futures_core::future::LocalBoxFuture;

pub type HttpService = BoxService<ServiceRequest, ServiceResponse, Error>;
pub type HttpNewService = BoxServiceFactory<(), ServiceRequest, ServiceResponse, Error, ()>;

#[derive(Clone)]
pub struct ModuleService(pub(crate) Rc<ModuleServiceInner>);

impl Deref for ModuleService {
    type Target = ModuleServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ModuleServiceInner {
    pub(crate) modules: Vec<HttpService>,
}

impl Service<ServiceRequest> for ModuleService {
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::always_ready!();

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            let (http_req, payload) = req.into_parts();

            for module in this.modules.iter() {
                let req = ServiceRequest::from_parts(http_req.clone(), Payload::None);
                let res = module.call(req).await?;
                if res.status() != StatusCode::NOT_FOUND {
                    return Ok(res);
                }
            }
            let req = ServiceRequest::from_parts(http_req, payload);
            Ok(req.into_response(
                HttpResponse::NotFound()
                    .insert_header(header::ContentType(mime::TEXT_PLAIN_UTF_8))
                    .body("Not Found"),
            ))
        })
    }
}
