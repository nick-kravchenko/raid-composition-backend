use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/health")
        .route("", web::get().to(app::health))
        .route("/postgres", web::get().to(postgres::health))
        .route("/redis", web::get().to(redis::health))
}

mod app;
mod postgres;
mod redis;

#[cfg(test)]
mod tests {
    use actix_web::{App, http::StatusCode, test};

    use super::scope;

    #[actix_web::test]
    async fn app_health_route_is_registered() {
        let app = test::init_service(App::new().service(scope())).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/health").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
