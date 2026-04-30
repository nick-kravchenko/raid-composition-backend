use actix_web::{HttpRequest, HttpResponse};

pub async fn logout(_req: HttpRequest) -> actix_web::Result<impl actix_web::Responder> {
    Ok(HttpResponse::NoContent().finish())
}
