use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use actix_web::http::Method;  // Use http types from actix_web
use futures::future::{ok, Ready};
use std::future::Future;
use std::pin::Pin;
use log::{debug, info};
use std::time::Instant;
use crate::config::Config;

pub struct CorsLogger {
    config: Config,
}

impl CorsLogger {
    pub fn new(config: Config) -> Self {
        CorsLogger { config }
    }
}

// Implement necessary traits for CorsLogger to work as middleware

impl<S, B> Transform<S, ServiceRequest> for CorsLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CorsLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CorsLoggerMiddleware { service, config: self.config.clone() })
    }
}

pub struct CorsLoggerMiddleware<S> {
    service: S,
    config: Config,
}

impl<S, B> Service<ServiceRequest> for CorsLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let origin = req.headers().get("Origin").and_then(|h| h.to_str().ok()).map(|s| s.to_owned());
        let is_cors = origin.is_some();
        let method = req.method().clone();
        let start_time = Instant::now();

        let fut = self.service.call(req);
        let config = self.config.clone();

        Box::pin(async move {
            let res = fut.await?;
            let duration = start_time.elapsed();

            if is_cors {
                info!("CORS request: Method: {:?}, Origin: {:?}, Duration: {:?}", method, origin, duration);
                if method == Method::OPTIONS {
                    debug!("CORS preflight request");
                }
                if let Some(allow_origin) = res.headers().get("Access-Control-Allow-Origin") {
                    info!("CORS response: Access-Control-Allow-Origin: {:?}", allow_origin);
                }
                if config.log_level <= log::LevelFilter::Debug {
                    if let Some(allow_methods) = res.headers().get("Access-Control-Allow-Methods") {
                        debug!("CORS response: Access-Control-Allow-Methods: {:?}", allow_methods);
                    }
                    if let Some(allow_headers) = res.headers().get("Access-Control-Allow-Headers") {
                        debug!("CORS response: Access-Control-Allow-Headers: {:?}", allow_headers);
                    }
                }
            } else {
                debug!("Non-CORS request: Method: {:?}, Duration: {:?}", method, duration);
            }
            Ok(res)
        })
    }
}
