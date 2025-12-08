use axum::{response::IntoResponse, routing::get, Router};
use crate::{state::AppState, utils::response::CustomResponse};

async fn health_check() -> impl IntoResponse  {
    return CustomResponse::<(), ()>::builder({}).message("Server is running!!!").build();
}

pub fn health_check_router() -> Router<AppState> {
    let router = Router::new().route("/", get(health_check));
    return router;
}