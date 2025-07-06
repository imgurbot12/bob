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

macro_rules! check_guards {
    ($req:expr, $ctx:expr, $guards:expr) => {
        if !$guards.iter().all(|g| (**g).check($ctx)) {
            return Box::pin(async move { Ok(default_response($req)) });
        }
    };
}

macro_rules! check_locations {
    ($req:expr, $ctx:expr, $locations:expr) => {{
        let location = $locations.iter().find_map(|l| (**l).check($ctx));
        match location {
            Some(loc) => loc,
            None if $locations.is_empty() => $req.path().to_owned(),
            None => return Box::pin(async move { Ok(default_response($req)) }),
        }
    }};
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

pub(crate) use check_guards;
pub(crate) use check_locations;
pub(crate) use impl_http_service;
