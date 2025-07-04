//!

use std::rc::Rc;

use actix_service::{IntoServiceFactory, ServiceFactory, ServiceFactoryExt, boxed};
use actix_web::{
    Error,
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    guard::Guard,
};
use futures_core::future::LocalBoxFuture;

use super::service::{ModuleService, ModuleServiceInner};
use crate::modules::service::*;

#[derive(Clone)]
pub struct ModuleSvc {
    mount_path: String,
    modules: Vec<Rc<HttpNewService>>,
    guards: Vec<Rc<dyn Guard>>,
}

impl ModuleSvc {
    pub fn new(mount_path: &str) -> Self {
        Self {
            mount_path: mount_path.to_owned(),
            modules: Vec::new(),
            guards: Vec::new(),
        }
    }
    pub fn add_guard<G: Guard + 'static>(&mut self, guards: G) {
        self.guards.push(Rc::new(guards));
    }
    pub fn add_module<F, U>(&mut self, f: F)
    where
        F: IntoServiceFactory<U, ServiceRequest>,
        U: ServiceFactory<ServiceRequest, Config = (), Response = ServiceResponse, Error = Error>
            + 'static,
    {
        // create and configure default resource
        let module = Rc::new(boxed::factory(f.into_factory().map_init_err(|_| ())));
        self.modules.push(module);
    }
}

impl HttpServiceFactory for ModuleSvc {
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

impl ServiceFactory<ServiceRequest> for ModuleSvc {
    type Response = ServiceResponse;
    type Error = Error;
    type Config = ();
    type Service = ModuleService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        let mut inner = ModuleServiceInner { modules: vec![] };
        let futures: Vec<_> = self.modules.iter().map(|m| m.new_service(())).collect();
        Box::pin(async {
            let mut modules = vec![];
            for fut in futures {
                match fut.await {
                    Ok(module) => modules.push(module),
                    Err(_) => return Err(()),
                }
            }
            inner.modules = modules;
            Ok(ModuleService(Rc::new(inner)))
        })
    }
}
