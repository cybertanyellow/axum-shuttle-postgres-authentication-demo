//use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};
//use http::Response;

use anyhow::{anyhow, Result};
use bit_vec::BitVec;
use chrono::{DateTime, Local};
use serde::{/*serde_if_integer128, */ Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info};

use crate::authentication::{/*auth, */ delete_user, login, password_hashed, signup2, AuthState,};
use crate::errors::NotLoggedIn;
//use crate::errors::{LoginError, NoUser, SignupError};
use crate::{Database, Random, /*COOKIE_MAX_AGE, */ USER_COOKIE_NAME};

async fn title_id_or_insert(database: &Database, name: &str) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM titles WHERE name = $1;";
    let title: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(&name)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = title {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = "INSERT INTO titles (name) VALUES ($1) RETURNING id;";
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

async fn department_id_or_insert(database: &Database, name: &str) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE name = $1;";
    let title: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(&name)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = title {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = "INSERT INTO departments (name) VALUES ($1) RETURNING id;";
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
    title: String,
    department: String,
    phone: String,
    email: String,
    create_at: DateTime<Local>,
    login_at: Option<DateTime<Local>>,
}

async fn query_user(account: &str, database: &Database) -> Option<DbUser> {
    const QUERY: &str = "SELECT u.account, u.permission, u.username, u.worker_id, t.name title, d.name department, phone, u.email, u.create_at, u.login_at FROM users u INNER JOIN titles t ON t.id = u.title_id INNER JOIN departments d ON d.id = u.department_id WHERE u.account = $1";

    if let Ok(user) = sqlx::query_as::<_, DbUser>(QUERY)
        .bind(account)
        .fetch_optional(database)
        .await
    {
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
            "error": "user not found",
        });
        (StatusCode::OK, Json(resp)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    account: String,
    password: String,
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
    let title_id = match user.title {
        Some(title) => match title_id_or_insert(&database, &title).await {
            Ok(id) => id,
            Err(e) => {
                let resp = json!({
                    "code": 400,
                    "error": &format!("title non-exist{e}"),
                });
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => 0,
    };

    let department_id = match user.department {
        Some(department) => match department_id_or_insert(&database, &department).await {
            Ok(id) => id,
            Err(e) => {
                let resp = json!({
                    "code": 400,
                    "error": &format!("department non-exist{e}"),
                });
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => 0,
    };

    match signup2(
        &database,
        random,
        &user.account,
        &user.password,
        &user.permission,
        &user.username,
        &user.worker_id,
        title_id,
        department_id,
        &user.phone,
        &user.email,
    )
    .await
    {
        Ok(session_token) => {
            let resp = json!({
                "code": 200,
                "session_key": USER_COOKIE_NAME,
                "session_value": session_token.into_cookie_value(),
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(error) => {
            let resp = json!({
                "code": 400,
                "error": format!("{}", error),
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
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
        }
        Err(error) => {
            let resp = json!({
                "code": 400,
                "error": format!("{}", error),
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
    }
}

pub(crate) async fn post_delete_api(
    Extension(current_user): Extension<AuthState>,
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

pub(crate) async fn users_api(Extension(database): Extension<Database>) -> impl IntoResponse {
    //const QUERY: &str = "SELECT username FROM users LIMIT 100;";
    const QUERY: &str = "SELECT u.account, u.permission, u.username, u.worker_id, t.name title, d.name department, phone, u.email, u.create_at, u.login_at FROM users u INNER JOIN titles t ON t.id = u.title_id INNER JOIN departments d ON d.id = u.department_id";

    if let Ok(users) = sqlx::query_as::<_, DbUser>(QUERY)
        .fetch_all(&database)
        .await
    {
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

pub(crate) async fn logout_response_api() -> impl IntoResponse {
    let resp = json!({
        "code": 200,
        "session_key": USER_COOKIE_NAME,
        "session_value": "_",
    });
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMe {
    password: Option<String>,
    username: Option<String>,
    phone: Option<String>,
    email: Option<String>,
}

pub(crate) async fn update_myself_api(
    Extension(mut current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Json(user): Json<UpdateMe>,
) -> impl IntoResponse {
    let account = if let Some(myself) = current_user.get_user().await {
        myself.account.clone()
    } else {
        let resp = json!({
            "code": 400,
            "error": "myself don't exist",
        });
        return (StatusCode::OK, Json(resp)).into_response();
    };

    let mut code = 200;
    let mut error = String::from("none");

    match user.password {
        None => info!("passowrd no change"),
        Some(pwd) => {
            if let Ok(hashed_password) = password_hashed(&pwd) {
                let fetch_one: Result<(i32,), _> = sqlx::query_as(
                    "UPDATE users SET password = $1 WHERE account = $2 RETURNING id;",
                )
                .bind(&hashed_password)
                .bind(&account)
                .fetch_one(&database)
                .await;
                match fetch_one {
                    Ok((id,)) => debug!("update passowrd ok {id}"),
                    Err(err) => {
                        error!("update password fail {err}");

                        code = 400;
                        error = format!("update password fail {err}");
                    }
                }
            } else {
                error = format!("password hashed fail");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(username) = user.username {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET username = $1 WHERE account = $2 RETURNING id;")
                .bind(&username)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update username ok {id}"),
            Err(err) => {
                error = format!("update username fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(phone) = user.phone {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET phone = $1 WHERE account = $2 RETURNING id;")
                .bind(&phone)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update phone ok {id}"),
            Err(err) => {
                error = format!("update phone fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(email) = user.email {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET email = $1 WHERE account = $2 RETURNING id;")
                .bind(&email)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update email ok {id}"),
            Err(err) => {
                error = format!("update email fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    let resp = json!({
        "code": code,
        "error": error,
    });
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    password: Option<String>,
    permission: Option<BitVec>,
    username: Option<String>,
    worker_id: Option<String>,
    title: Option<String>,
    department: Option<String>,
    phone: Option<String>,
    email: Option<String>,
}

pub(crate) async fn update_user_api(
    Extension(mut current_user): Extension<AuthState>,
    Path(account): Path<String>,
    Extension(database): Extension<Database>,
    Json(user): Json<UpdateUser>,
) -> impl IntoResponse {
    let mut code = 200;
    let mut error = String::from("none");

    if let Some(user) = current_user.get_user().await {
        info!("TODO, user-{} is administrator?", user.account);
    }
    match user.password {
        None => info!("passowrd no change"),
        Some(pwd) => {
            if let Ok(hashed_password) = password_hashed(&pwd) {
                let fetch_one: Result<(i32,), _> = sqlx::query_as(
                    "UPDATE users SET password = $1 WHERE account = $2 RETURNING id;",
                )
                .bind(&hashed_password)
                .bind(&account)
                .fetch_one(&database)
                .await;
                match fetch_one {
                    Ok((id,)) => info!("update passowrd ok {id}"),
                    Err(err) => {
                        error!("update password fail {err}");

                        code = 400;
                        error = format!("update password fail {err}");
                    }
                }
            } else {
                error = format!("password hashed fail");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(permission) = user.permission {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET permission = $1 WHERE account = $2 RETURNING id;")
                .bind(&permission)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update permission ok {id}"),
            Err(err) => {
                error = format!("update permission fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(username) = user.username {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET username = $1 WHERE account = $2 RETURNING id;")
                .bind(&username)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update username ok {id}"),
            Err(err) => {
                error = format!("update username fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(worker_id) = user.worker_id {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET worker_id = $1 WHERE account = $2 RETURNING id;")
                .bind(&worker_id)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update worker_id ok {id}"),
            Err(err) => {
                error = format!("update worker_id fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(title) = user.title {
        match title_id_or_insert(&database, &title).await {
            Ok(tid) => {
                let return_one: Result<(i32,), _> = sqlx::query_as(
                    "UPDATE users SET title_id = $1 WHERE account = $2 RETURNING id;",
                )
                .bind(tid)
                .bind(&account)
                .fetch_one(&database)
                .await;
                match return_one {
                    Ok((id,)) => info!("update title ok {id}"),
                    Err(err) => {
                        error = format!("update title fail {err}");
                        code = 400;
                        error!("{error}");
                    }
                }
            }
            Err(e) => error!("title-id fail - {e}"),
        }
    }

    if let Some(department) = user.department {
        match department_id_or_insert(&database, &department).await {
            Ok(did) => {
                let return_one: Result<(i32,), _> = sqlx::query_as(
                    "UPDATE users SET department_id = $1 WHERE account = $2 RETURNING id;",
                )
                .bind(&did)
                .bind(&account)
                .fetch_one(&database)
                .await;
                match return_one {
                    Ok((id,)) => info!("update department ok {id}"),
                    Err(err) => {
                        error = format!("update department fail {err}");
                        code = 400;
                        error!("{error}");
                    }
                }
            }
            Err(e) => {
                error = format!("department-id fail - {e}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(phone) = user.phone {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET phone = $1 WHERE account = $2 RETURNING id;")
                .bind(&phone)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update phone ok {id}"),
            Err(err) => {
                error = format!("update phone fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    if let Some(email) = user.email {
        let return_one: Result<(i32,), _> =
            sqlx::query_as("UPDATE users SET email = $1 WHERE account = $2 RETURNING id;")
                .bind(&email)
                .bind(&account)
                .fetch_one(&database)
                .await;
        match return_one {
            Ok((id,)) => info!("update email ok {id}"),
            Err(err) => {
                error = format!("update email fail {err}");
                code = 400;
                error!("{error}");
            }
        }
    }

    let resp = json!({
        "code": code,
        "error": error,
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
