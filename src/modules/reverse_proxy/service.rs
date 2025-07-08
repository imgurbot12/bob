//! Actix Service Implementation for File Server

use std::{ops::Deref, rc::Rc};

use actix_web::{
    HttpMessage, HttpResponseBuilder,
    body::BoxBody,
    dev::{self, Service, ServiceRequest, ServiceResponse},
    error::Error,
    guard::Guard,
};
use futures_core::future::LocalBoxFuture;

use super::utils::combine_uri;
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
    pub(crate) client: Rc<awc::Client>,
    pub(crate) resolve: awc::http::Uri,
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
            let (http_req, payload) = req.into_parts();

            // combine resolution uri with request-uri
            let uri = match combine_uri(&this.resolve, &url_path, http_req.uri()) {
                Ok(uri) => uri,
                Err(err) => {
                    log::error!("request error: {err:?}");
                    let req = ServiceRequest::from_parts(http_req, dev::Payload::None);
                    return Ok(default_response(req));
                }
            };

            // build forwarded-request and send, then retrieve response
            let mut forward_res = match this
                .client
                .request(http_req.method().clone(), uri)
                .no_decompress()
                .send_stream(payload)
                .await
            {
                Ok(res) => res,
                Err(err) => {
                    log::error!("request error: {err:?}");
                    let req = ServiceRequest::from_parts(http_req, dev::Payload::None);
                    return Ok(default_response(req));
                }
            };

            // wrap response payload into body-stream
            let payload = forward_res.take_payload();
            let body = actix_web::body::BodyStream::new(payload);

            // transfer client response details to web-service http-response
            let mut builder = HttpResponseBuilder::new(forward_res.status());
            for header in forward_res.headers() {
                builder.append_header(header);
            }

            // build final response and send
            let http_res = builder.body(body);
            Ok(ServiceResponse::new(http_req, http_res))
        })
    }
}
