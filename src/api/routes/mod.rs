use actix_web::{Scope, web};

pub fn api_v1() -> Scope {
    web::scope("/api").service(v1::scope())
}

pub mod v1;

#[cfg(test)]
mod tests {
    use actix_web::{App, http::StatusCode, test};

    use super::api_v1;

    #[actix_web::test]
    async fn api_v1_health_route_is_registered() {
        let app = test::init_service(App::new().service(api_v1())).await;

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/api/v1/health").to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
