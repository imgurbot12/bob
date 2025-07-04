//! Reusable Service Construction Macros

use actix_web::{
    HttpResponse,
    dev::{ServiceRequest, ServiceResponse},
    http::header,
    mime,
};

#[inline]
pub fn default_response(req: ServiceRequest) -> ServiceResponse {
    req.into_response(
        HttpResponse::NotFound()
            .insert_header(header::ContentType(mime::TEXT_PLAIN_UTF_8))
            .body("Not Found"),
    )
}

macro_rules! impl_http_service {
    ($factory:ident) => {
        impl HttpServiceFactory for $factory {
            fn register(mut self, config: &mut AppService) {
                let guards = if self.guards.is_empty() {
                    None
                } else {
                    let guards = std::mem::take(&mut self.guards);
                    Some(
                        guards
                            .into_iter()
                            .map(|guard| -> Box<dyn Guard> { Box::new(guard) })
                            .collect::<Vec<_>>(),
                    )
                };

                let rdef = if config.is_root() {
                    ResourceDef::root_prefix(&self.mount_path)
                } else {
                    ResourceDef::prefix(&self.mount_path)
                };

                config.register_service(rdef, guards, self, None)
            }
        }
    };
}

pub(crate) use impl_http_service;
