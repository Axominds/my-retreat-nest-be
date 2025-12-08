use std::any::Any;

use axum::{body::Body, http::{Response, StatusCode}};

use crate::utils::response::CustomResponse;

pub fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response<Body> {
    let message: &str = if let Some(s) = err.downcast_ref::<String>() {
        &s
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s
    } else {
        "Something went wrong."
    };

    return CustomResponse::<(), ()>::builder({}).message(message).status_code(StatusCode::INTERNAL_SERVER_ERROR).build();
}