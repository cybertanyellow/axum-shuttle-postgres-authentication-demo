use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};

use anyhow::{anyhow, Result};
use bit_vec::BitVec;
use chrono::{DateTime, Utc};
use serde::{/*serde_if_integer128, */ Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info};
use utoipa::{IntoParams, ToSchema};

use crate::authentication::{
    AuthState, CurrentUser,
};
use crate::{Database, Random};
use crate::dcare_user::ApiResponse;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderInfo {
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderUpdate {
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderNew {
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    code: u16,
    order: Option<OrderInfo>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrdersResponse {
    code: u16,
    order: Option<Vec<OrderInfo>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/order/{id}",
    params(
        ("id" = u32, Path, description = "order ID")
    ),
    responses(
        (status = 200, description = "get detail order information", body = OrderResponse)
    )
)]
pub(crate) async fn order_request(
    Extension(_auth_state): Extension<AuthState>,
    Extension(_database): Extension<Database>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let resp = OrderResponse {
        code: 400,
        order: None,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    put,
    path = "/api/v1/order/{id}",
    params(
        ("id" = u32, Path, description = "order ID")
    ),
    request_body = OrderUpdate,
    responses(
        (status = 200, description = "delete success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
        (status = 404, description = "order not found, ", body = ApiResponse, example = json!(ApiResponse::new(404, Some(String::from("..."))))),
        (status = 405, description = "permission deny, ", body = ApiResponse, example = json!(ApiResponse::new(405, Some(String::from("..."))))),
        (status = 500, description = "server error, ", body = ApiResponse, example = json!(ApiResponse::new(500, Some(String::from("..."))))),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn order_update(
    Extension(mut current_user): Extension<AuthState>,
    Path(id): Path<u32>,
    Extension(database): Extension<Database>,
    Json(order): Json<OrderUpdate>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, Some(String::from("TODO")));
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    delete,
    path = "/api/v1/order/{id}",
    params(
        ("id" = u32, Path, description = "order ID to delete")
    ),
    responses(
        (status = 200, description = "delete success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
        (status = 404, description = "order not found, ", body = ApiResponse, example = json!(ApiResponse::new(404, Some(String::from("..."))))),
        (status = 405, description = "permission deny", body = ApiResponse, example = json!(ApiResponse::new(405, Some(String::from("..."))))),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn order_delete(
    Extension(mut _current_user): Extension<AuthState>,
    Extension(_database): Extension<Database>,
    Path(_id): Path<u32>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, Some(String::from("TODO")));
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/order",
    responses(
        (status = 200, description = "get order list", body = OrdersResponse)
    )
)]
pub(crate) async fn order_list_request(
    Extension(_database): Extension<Database>
) -> impl IntoResponse {
    let resp = OrdersResponse {
        code: 400,
        order: None,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/order",
    request_body = OrderNew,
    responses(
        (status = 200, description = "add order success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
        (status = 400, description = "order exist, ", body = ApiResponse, example = json!(ApiResponse::new(400, Some(String::from("..."))))),
        (status = 500, description = "server DB error, ", body = ApiResponse, example = json!(ApiResponse::new(500, Some(String::from("..."))))),
    ),
)]
pub(crate) async fn order_create(
    Extension(_database): Extension<Database>,
    Extension(_random): Extension<Random>,
    Json(_order): Json<OrderNew>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, Some(String::from("TODO")));

    (StatusCode::OK, Json(resp)).into_response()
}
