use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use http::Response;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::authentication::{auth, delete_user, login, signup, AuthState};
use crate::errors::{LoginError, NoUser, NotLoggedIn, SignupError};
use crate::{Database, Random, COOKIE_MAX_AGE, USER_COOKIE_NAME};

pub(crate) async fn user_api(
    Path(username): Path<String>,
    Extension(mut auth_state): Extension<AuthState>,
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    const QUERY: &str = "SELECT username FROM users WHERE username = $1;";

    let user: Option<(String,)> = sqlx::query_as(QUERY)
        .bind(&username)
        .fetch_optional(&database)
        .await
        .unwrap();

    if let Some((username,)) = user {
        let user_is_self = auth_state
            .get_user()
            .await
            .map(|logged_in_user| logged_in_user.username == username)
            .unwrap_or_default();

        let resp = json!({
            "code": 200,
            "username": &username,
            "is_self": &user_is_self
        });
        (StatusCode::OK, Json(resp)).into_response()
    } else {
        let resp = json!({
            "code": 401,
        });
        (StatusCode::OK, Json(resp)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    username: String,
    password: String,
    confirm_password: String,

    email: String,
    phone: String,
    role_id: u64,
}

pub(crate) async fn post_signup_api(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    Json(user): Json<CreateUser>,
) -> impl IntoResponse {
    if user.password != user.confirm_password {
        let resp = json!({
            "code": 400,
            "error": "passowrd not match",
        });
        return (StatusCode::OK, Json(resp)).into_response();
    }

    match signup(&database, random, &user.username, &user.password).await {
        Ok(session_token) => {
            let resp = json!({
                "code": 200,
                "session_key": USER_COOKIE_NAME,
                "session_value": session_token.into_cookie_value(),
            });
            (StatusCode::OK, Json(resp)).into_response()
        },
        Err(error) => {
            let resp = json!({
                "code": 400,
                "error": format!("{}", error),
            });
            (StatusCode::OK, Json(resp)).into_response()
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    username: String,
    password: String,
}

pub(crate) async fn post_login_api(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    Json(user): Json<User>,
) -> impl IntoResponse {
    match login(&database, random, &user.username, &user.password).await {
        Ok(session_token) => {
            let resp = json!({
                "code": 200,
                "session_key": USER_COOKIE_NAME,
                "session_value": session_token.into_cookie_value(),
            });
            (StatusCode::OK, Json(resp)).into_response()
        },
        Err(error) => {
            let resp = json!({
                "code": 400,
                "error": format!("{}", error),
            });
            (StatusCode::OK, Json(resp)).into_response()
        },
    }
}

pub(crate) async fn post_delete_api(
    Extension(current_user): Extension<AuthState>
) -> impl IntoResponse {
    if !current_user.logged_in() {
        let resp = json!({
            "code": 400,
            "error": format!("{}", NotLoggedIn),
        });
        return (StatusCode::OK, Json(resp)).into_response();
    }

    delete_user(current_user).await;

    let resp = json!({
        "code": 200,
        "session_key": USER_COOKIE_NAME,
        "session_value": "_",
    });
    (StatusCode::OK, Json(resp)).into_response()
}

pub(crate) async fn me_api(
    Extension(mut current_user): Extension<AuthState>,
) -> impl IntoResponse {
    if let Some(user) = current_user.get_user().await {
        let resp = json!({
            "code": 200,
            "error": "TODO",
        });
        (StatusCode::OK, Json(resp)).into_response()
    } else {
        let resp = json!({
            "code": 400,
            "error": format!("{}", &NotLoggedIn),
        });
        (StatusCode::OK, Json(resp)).into_response()
    }
}

pub(crate) async fn users_api(
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    const QUERY: &str = "SELECT username FROM users LIMIT 100;";

    let users: Vec<(String,)> = sqlx::query_as(QUERY).fetch_all(&database).await.unwrap();

    // This should be a no op right :)
    let users = users.into_iter().map(|(value,)| value).collect::<Vec<_>>();

    let resp = json!({
        "code": 200,
        "users": &users,
    });
    (StatusCode::OK, Json(resp)).into_response()
}

pub(crate) async fn logout_response_api(
) -> impl IntoResponse {
    let resp = json!({
        "code": 200,
        "session_key": USER_COOKIE_NAME,
        "session_value": "_",
    });
    (StatusCode::OK, Json(resp)).into_response()
}

