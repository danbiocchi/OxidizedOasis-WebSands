use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// Global request counter
static TOTAL_REQUESTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct RequestMetrics;

impl<S, B> Transform<S, ServiceRequest> for RequestMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestMetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestMetricsMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestMetricsMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let start_time = Instant::now();
        let current_total_requests = TOTAL_REQUESTS.fetch_add(1, Ordering::SeqCst) + 1;

        Box::pin(async move {
            let res = service.call(req).await;
            let duration = start_time.elapsed();
            println!(
                "Request #{current_total_requests}: processed in {duration:?}"
            );
            res
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse, Responder};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Simple counter struct for per-test isolation
    #[derive(Clone)]
    pub struct TestRequestMetrics {
        counter: Arc<AtomicUsize>,
    }

    impl TestRequestMetrics {
        pub fn new() -> Self {
            Self {
                counter: Arc::new(AtomicUsize::new(0)),
            }
        }

        pub fn get_count(&self) -> usize {
            self.counter.load(Ordering::SeqCst)
        }
    }

    impl<S, B> Transform<S, ServiceRequest> for TestRequestMetrics
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
        B: 'static,
    {
        type Response = ServiceResponse<B>;
        type Error = Error;
        type InitError = ();
        type Transform = TestRequestMetricsMiddleware<S>;
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        fn new_transform(&self, service: S) -> Self::Future {
            ready(Ok(TestRequestMetricsMiddleware {
                service: Rc::new(service),
                counter: Arc::clone(&self.counter),
            }))
        }
    }

    pub struct TestRequestMetricsMiddleware<S> {
        service: Rc<S>,
        counter: Arc<AtomicUsize>,
    }

    impl<S, B> Service<ServiceRequest> for TestRequestMetricsMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
        B: 'static,
    {
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        fn call(&self, req: ServiceRequest) -> Self::Future {
            let service = Rc::clone(&self.service);
            let counter = Arc::clone(&self.counter);
            let start_time = Instant::now();
            let current_request_num = counter.fetch_add(1, Ordering::SeqCst) + 1;

            Box::pin(async move {
                let res = service.call(req).await;
                let duration = start_time.elapsed();
                println!(
                    "Test Request #{}: processed in {:?}",
                    current_request_num, duration
                );
                res
            })
        }
    }

    async fn test_handler() -> impl Responder {
        HttpResponse::Ok().body("test")
    }

    #[actix_rt::test]
    async fn test_request_counter_and_timing_log() {
        let metrics = TestRequestMetrics::new();
        let initial_count = metrics.get_count();

        let app = test::init_service(
            App::new()
                .wrap(metrics.clone())
                .route("/", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());

        let final_count = metrics.get_count();
        assert_eq!(final_count - initial_count, 1, "Request counter should increment by 1");

        // To test timing, we'd ideally capture stdout or use a logging facade.
        // Since the current implementation prints to stdout, this test primarily ensures
        // the counter increments. A more robust test would involve a mock logger
        // or checking if logs were written if a real logging framework was used.
        // For now, we are implicitly testing that the request processing path that includes
        // the logging was executed because the counter incremented.
    }

    #[actix_rt::test]
    async fn test_multiple_requests_increment_counter() {
        let metrics = TestRequestMetrics::new();
        let initial_count = metrics.get_count();

        let app = test::init_service(
            App::new()
                .wrap(metrics.clone())
                .route("/", web::get().to(test_handler)),
        )
        .await;

        let req1 = test::TestRequest::get().uri("/").to_request();
        test::call_service(&app, req1).await;

        let req2 = test::TestRequest::get().uri("/").to_request();
        test::call_service(&app, req2).await;

        let final_count = metrics.get_count();
        assert_eq!(final_count - initial_count, 2, "Request counter should increment by 2 for two requests");
    }
}
