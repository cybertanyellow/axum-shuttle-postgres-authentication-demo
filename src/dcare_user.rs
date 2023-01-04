use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use http::Response;

use serde::{Deserialize, Serialize, serde_if_integer128};
use serde_json::json;
use bit_vec::BitVec;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};

use crate::authentication::{auth, delete_user, login, signup2, AuthState};
use crate::errors::{LoginError, NoUser, NotLoggedIn, SignupError};
use crate::{Database, Random, COOKIE_MAX_AGE, USER_COOKIE_NAME};

async fn title_id_or_insert(
    database: &Database,
    name: &str,
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM titles WHERE name = $1;";
    let title: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(&name)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = title {
        Ok(id)
    } else {
        const INSERT_QUERY: &str =
            "INSERT INTO titles (name) VALUES ($1) RETURNING id;";
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(name)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((title_id,)) => Ok(title_id),
            Err(err) => Err(anyhow!("insert title fail - {err}")),
        }
    }
}

async fn department_id_or_insert(
    database: &Database,
    name: &str,
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE name = $1;";
    let title: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(&name)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = title {
        Ok(id)
    } else {
        const INSERT_QUERY: &str =
            "INSERT INTO departments (name) VALUES ($1) RETURNING id;";
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(name)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((department_id,)) => Ok(department_id),
            Err(err) => Err(anyhow!("insert department fail - {err}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbUser {
    account: String,
    permission: Option<BitVec>,
    username: Option<String>,
    worker_id: Option<String>,
    title_id: String,
    department_id: String,
    phone: String,
    email: String,
    create_at: DateTime<Local>,
    login_at: Option<DateTime<Local>>,
}

async fn query_user(
    account: &str,
    database: &Database,
) -> Option<DbUser> {
    const QUERY: &str = "SELECT u.account, u.permission, u.username, u.worker_id, t.name title_id, d.name department_id, phone, u.email, u.create_at, u.login_at FROM users u INNER JOIN titles t ON t.id = u.title_id INNER JOIN departments d ON d.id = u.department_id WHERE u.account = $1";

    if let Ok(user) = sqlx::query_as::<_, DbUser>(QUERY)
        .bind(account)
        .fetch_optional(database)
        .await {
            user
    } else {
        None
    }
}

pub(crate) async fn user_api(
    Path(account): Path<String>,
    //Extension(_auth_state): Extension<AuthState>,
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    /* TODO, limit with auth_state's pemission */
    if let Some(user) = query_user(&account, &database).await {
        let resp = json!({
            "code": 200,
            "user": &user
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
    account: String,
    password: String,
    confirm_password: String,
    permission: BitVec,
    username: String,
    worker_id: String,
    title: Option<String>,
    department: Option<String>,
    phone: String,
    email: String,
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

    let title_id = match user.title {
        Some(title) => {
            match title_id_or_insert(&database, &title).await {
                Ok(id) => id,
                Err(e) => {
                    let resp = json!({
                        "code": 400,
                        "error": "title non-exist",
                    });
                    return (StatusCode::OK, Json(resp)).into_response();
                }
            }
        },
        None => 0,
    };


    let department_id = match user.department {
        Some(department) => {
            match department_id_or_insert(&database, &department).await {
                Ok(id) => id,
                Err(e) => {
                    let resp = json!({
                        "code": 400,
                        "error": "department non-exist",
                    });
                    return (StatusCode::OK, Json(resp)).into_response();
                }
            }
        },
        None => 0,
    };

    match signup2(&database, random, &user.account, &user.password,
                 &user.permission, &user.username, &user.worker_id,
                 title_id, department_id, &user.phone, &user.email).await {
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
    account: String,
    password: String,
}

pub(crate) async fn post_login_api(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    Json(user): Json<User>,
) -> impl IntoResponse {
    match login(&database, random, &user.account, &user.password).await {
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
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    if let Some(user) = current_user.get_user().await {
        if let Some(user) = query_user(&user.account, &database).await {
            let resp = json!({
                "code": 200,
                "user": &user
            });
            (StatusCode::OK, Json(resp)).into_response()
        } else {
            let resp = json!({
                "code": 400,
                "error": "user non-exist"
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
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
    //const QUERY: &str = "SELECT username FROM users LIMIT 100;";
    const QUERY: &str = "SELECT u.account, u.permission, u.username, u.worker_id, t.name title_id, d.name department_id, phone, u.email, u.create_at, u.login_at FROM users u INNER JOIN titles t ON t.id = u.title_id INNER JOIN departments d ON d.id = u.department_id";

    if let Ok(users) = sqlx::query_as::<_, DbUser>(QUERY)
        .fetch_all(&database)
        .await {

        let resp = json!({
            "code": 200,
            "users": &users
        });
        (StatusCode::OK, Json(resp)).into_response()
    } else {
        let resp = json!({
            "code": 401,
        });
        (StatusCode::OK, Json(resp)).into_response()
    }
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

pub(crate) async fn update_myself_api(
    Extension(mut current_user): Extension<AuthState>,
    Path(account): Path<String>,
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    let resp = json!({
        "code": 400,
        "error": "TODO"
    });
    (StatusCode::OK, Json(resp)).into_response()
}

pub(crate) async fn update_user_api(
    Extension(mut current_user): Extension<AuthState>,
    Path(account): Path<String>,
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    let resp = json!({
        "code": 400,
        "error": "TODO"
    });
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    permission: BitVec,
}

#[test]
fn test_json_create_user() {
    let a_user = TestUser {
        permission: BitVec::from_bytes(&[0b10100000]),
    };
    println!("serdes as {}", serde_json::to_string(&a_user).unwrap());

    let data = r#"{
        "permission": 0011
    }"#;
    let b_user: TestUser = serde_json::from_str(data).unwrap();
    assert_eq!(a_user, b_user);
}
