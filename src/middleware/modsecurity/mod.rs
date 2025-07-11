//! ModSecurity Middleware Implementation

use std::future::{Ready, ready};
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

use actix_web::dev::Payload;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::{self, BoxBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::{StatusCode, Version},
};
use futures_core::future::LocalBoxFuture;
use serde::Deserialize;

use super::payload::BytesPayload;

#[inline]
fn version_str(v: Version) -> &'static str {
    match v {
        Version::HTTP_09 => "0.9",
        Version::HTTP_10 => "1.0",
        Version::HTTP_11 => "1.1",
        Version::HTTP_2 => "2",
        Version::HTTP_3 => "3",
        _ => panic!("unexpected http version!"),
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModSecurity {
    rules: Option<String>,
    rule_files: Vec<PathBuf>,
    max_request_body_size: Option<usize>,
    max_response_body_size: Option<usize>,
}

impl<S> Transform<S, ServiceRequest> for ModSecurity
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = ModSecurityMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let modsec = modsecurity::ModSecurity::default();
        let mut rules = modsecurity::Rules::new();
        if let Some(rules_str) = self.rules.as_ref() {
            rules
                .add_plain(rules_str)
                .expect("modsecurity failed to load rules");
        }
        self.rule_files
            .iter()
            .try_for_each(|p| rules.add_file(&p))
            .expect("modsecurity failed to load file rules");
        ready(Ok(ModSecurityMiddleware(Rc::new(ModSecurityInner {
            service: Rc::new(service),
            modsec,
            rules,
            max_request_body_size: self.max_request_body_size.unwrap_or(u16::MAX as usize),
            max_response_body_size: self.max_response_body_size.unwrap_or(u16::MAX as usize),
        }))))
    }
}

#[derive(Clone)]
pub struct ModSecurityMiddleware<S>(Rc<ModSecurityInner<S>>);

impl<S> Deref for ModSecurityMiddleware<S> {
    type Target = ModSecurityInner<S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ModSecurityInner<S> {
    service: Rc<S>,
    modsec: modsecurity::ModSecurity,
    rules: modsecurity::Rules,
    max_request_body_size: usize,
    max_response_body_size: usize,
}

impl<S> Service<ServiceRequest> for ModSecurityMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let this = Rc::clone(&self.0);
        Box::pin(async move {
            let mut transaction = this
                .modsec
                .transaction_builder()
                .with_rules(&this.rules)
                .build()
                .expect("modsecurity transaction build failed");

            // process request uri
            let uri = req.uri().to_string();
            let method = req.method().as_str();
            let version = version_str(req.version());
            transaction
                .process_uri(&uri, method, version)
                .expect("modsecurity failed to process uri");

            // scan request headers
            req.headers()
                .iter()
                .filter_map(|(k, v)| Some((k.as_str(), v.to_str().ok()?)))
                .try_for_each(|(k, v)| transaction.add_request_header(k, v))
                .expect("modsecurity request headers failed to scan");
            transaction
                .process_request_headers()
                .expect("modsecurity failed to process request headers");

            // load request body into memory from payload with max-size
            let stream = body::BodyStream::new(req.take_payload());
            let http_body = match body::to_bytes_limited(stream, this.max_request_body_size).await {
                Ok(body) => match body {
                    Ok(body) => body,
                    Err(err) => return Ok(req.error_response(err)),
                },
                Err(_) => return Ok(req.into_response(HttpResponse::PayloadTooLarge())),
            };

            // process request body
            transaction
                .append_request_body(&http_body)
                .expect("modsecurity failed to process request body");

            // put in-memory body back into payload
            let buf = BytesPayload::new(http_body);
            req.set_payload(buf.into_payload());

            let res = this.service.call(req).await?;

            // process status-line and response headers
            let code: u16 = res.status().into();
            let version = format!("HTTP {}", version_str(res.response().head().version));
            res.headers()
                .iter()
                .filter_map(|(k, v)| Some((k.as_str(), v.to_str().ok()?)))
                .try_for_each(|(k, v)| transaction.add_response_header(k, v))
                .expect("modsecurity request headers failed to scan");
            transaction
                .process_response_headers(code as i32, &version)
                .expect("modsecurity failed to process response headers");

            // repackage request for re-use in response generation
            let (http_req, http_res) = res.into_parts();
            let req = ServiceRequest::from_parts(http_req, Payload::None);

            // load response body into memory from payload with max-size
            let (http_res, stream) = http_res.into_parts();
            let http_body = match body::to_bytes_limited(stream, this.max_response_body_size).await
            {
                Ok(body) => match body {
                    Ok(body) => body,
                    Err(err) => return Ok(req.error_response(err)),
                },
                Err(_) => return Ok(req.into_response(HttpResponse::InsufficientStorage())),
            };

            // process response body
            transaction
                .append_request_body(&http_body)
                .expect("modsecurity failed to process request body");

            // send custom response on intervention
            if let Some(intv) = transaction.intervention() {
                if let Some(msg) = intv.log() {
                    log::warn!("{msg}");
                }
                if let Some(url) = intv.url() {
                    let mut res = HttpResponse::TemporaryRedirect();
                    res.insert_header(("Location", url));
                    return Ok(req.into_response(res));
                }
                let code = StatusCode::from_u16(intv.status() as u16)
                    .expect("invalid intervention status");
                return Ok(req.into_response(HttpResponse::new(code)));
            }

            // place in-memory body back into response
            let boxed = body::BoxBody::new(http_body);
            let http_res = http_res.set_body(boxed);

            Ok(req.into_response(http_res))
        })
    }
}
