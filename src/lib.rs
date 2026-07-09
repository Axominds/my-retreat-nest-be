pub mod entities;
pub mod entities_helper;
pub mod env;
pub mod routes;
pub mod serializers;
pub mod state;
pub mod utils;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::{catch_panic::CatchPanicLayer, compression::CompressionLayer, cors::{Any, CorsLayer}};

use crate::{state::AppState, utils::middlewares::panic::handle_panic};

pub async fn run() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app_state: AppState = AppState::new().await;
    let router = Router::new()
        .merge(routes::health::health_check_router())
        .merge(routes::auth::auth_router())
        .merge(routes::users::users_router())
        .merge(routes::categories::category_router())
        .merge(routes::retreats::retreat_router())
        .merge(routes::retreat_reviews::retreat_review_router())
        .merge(routes::gallery_categories::gallery_category_router())
        .merge(routes::retreat_galleries::retreat_gallery_router())
        .merge(routes::wishlists::wishlist_router())
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(app_state);

    let server_host = &env::ENV.server_host;
    let server_port = &env::ENV.server_port;
    let server_address: String = format!("{}:{}", server_host, server_port);

    let listener: TcpListener = TcpListener::bind(server_address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
