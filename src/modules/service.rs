//! Actix Service Abstraction to Support Running Multiple Modules in Sequence

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

use super::payload::{PayloadBuffer, PayloadRef};

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
    pub(crate) body_buffer_size: usize,
    pub(crate) body_max_size: usize,
}

impl Service<ServiceRequest> for ModuleService {
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::always_ready!();

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // simplify processing for single module
        let this = self.clone();
        if self.modules.len() == 1 {
            return Box::pin(async move { (&this.modules[0]).call(req).await });
        }
        // support handling multiple modules
        Box::pin(async move {
            // body needs to be buffered to be re-sent across modules
            let (http_req, payload) = req.into_parts();
            let buffer = PayloadBuffer::new(payload, this.body_buffer_size);
            let pref = PayloadRef::new(buffer);
            // iterate modules and pass copy of service-request
            for module in this.modules.iter() {
                let req = ServiceRequest::from_parts(http_req.clone(), pref.into_payload());
                let res = module.call(req).await?;
                if res.status() != StatusCode::NOT_FOUND {
                    return Ok(res);
                }
                // reset buffered payload for next module
                pref.get_mut().reset_stream();
            }
            let req = ServiceRequest::from_parts(http_req, Payload::None);
            Ok(req.into_response(
                HttpResponse::NotFound()
                    .insert_header(header::ContentType(mime::TEXT_PLAIN_UTF_8))
                    .body("Not Found"),
            ))
        })
    }
}
