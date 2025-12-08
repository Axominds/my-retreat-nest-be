use axum::{
    Json,
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize, Clone)]
struct ResponseData<T: Serialize, M: Serialize> {
    data: T,
    message: String,
    meta: Option<M>,
}

#[derive(Clone)]
pub struct CustomResponse<T: Serialize, M: Serialize> {
    status_code: StatusCode,
    data: T,
    message: String,
    meta: Option<M>,
}

// ---------------- IntoResponse -------------------

impl<T: Serialize, M: Serialize> IntoResponse for CustomResponse<T, M> {
    fn into_response(self) -> Response<Body> {
        let response_data = ResponseData {
            data: self.data,
            message: self.message,
            meta: self.meta,
        };
        (self.status_code, Json(&response_data)).into_response()
    }
}

// ---------------- Builder methods ----------------

impl<T: Serialize + Clone, M: Serialize + Clone> CustomResponse<T, M> {
    pub fn builder(data: T) -> CustomResponse<T, M> {
        Self {
            status_code: StatusCode::OK,
            data,
            message: String::new(),
            meta: None,
        }
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }
    
    pub fn meta(mut self, meta: M) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    pub fn build(&self) -> Response<Body> {
        let response = Self {
            status_code: self.status_code,
            data: self.data.clone(),
            message: self.message.clone(),
            meta: self.meta.clone(),
        };
        response.into_response()
    }
}

// ---------------- Error helpers ----------------

pub fn to_error_response<E: std::fmt::Display>(e: E, status: StatusCode) -> Response<Body> {
    CustomResponse::<(), ()>::builder(())
        .message(&e.to_string())
        .status_code(status)
        .build()
}

pub fn to_error_response_with_message(message: &str, status: StatusCode) -> Response<Body> {
    CustomResponse::<(), ()>::builder(())
        .message(message)
        .status_code(status)
        .build()
}
