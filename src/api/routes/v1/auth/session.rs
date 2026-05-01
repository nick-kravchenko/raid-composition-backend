use actix_web::{HttpRequest, HttpResponse, web};
use uuid::Uuid;

use crate::{auth::service, state::AppState};

pub async fn current(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    Ok(service::current_session_response(state, req).await?)
}

pub async fn list(state: web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    Ok(service::list_sessions_response(state, req).await?)
}

pub async fn logout(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    Ok(service::logout_response(state, req).await?)
}

pub async fn logout_all_other_sessions(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    Ok(service::logout_all_other_sessions_response(state, req).await?)
}

pub async fn revoke(
    state: web::Data<AppState>,
    req: HttpRequest,
    session_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::revoke_session_response(state, req, session_id.into_inner()).await?)
}

pub async fn csrf(state: web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    Ok(service::refresh_csrf_response(state, req).await?)
}
