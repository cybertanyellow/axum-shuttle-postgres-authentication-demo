//use std::sync::{Arc, Mutex};

use std::{os::unix::prelude::PermissionsExt, fs::Permissions};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};
use http::Response;
use http_body::Full;

use anyhow::{anyhow, Result};
use bit_vec::BitVec;
use chrono::{DateTime, Utc};
use serde::{/*serde_if_integer128, */ Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info};
use utoipa::{IntoParams, ToSchema};

use crate::authentication::{
    /*auth, */ delete_user2, login, password_hashed, signup2, AuthState, CurrentUser, SessionToken,
};
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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct UserInfo {
    account: String,
    #[schema(example = json!({"storage": [1], "nbits": 8}))]
    permission: BitVec,
    username: Option<String>,
    worker_id: Option<String>,
    title: String,
    department: String,
    phone: String,
    email: String,
    create_at: DateTime<Utc>,
    login_at: Option<DateTime<Utc>>,
}

async fn query_user(account: &str, database: &Database) -> Option<UserInfo> {
    const QUERY: &str = "SELECT u.account, u.permission, u.username, u.worker_id, t.name title, d.name department, phone, u.email, u.create_at, u.login_at FROM users u INNER JOIN titles t ON t.id = u.title_id INNER JOIN departments d ON d.id = u.department_id WHERE u.account = $1";

    if let Ok(user) = sqlx::query_as::<_, UserInfo>(QUERY)
        .bind(account)
        .fetch_optional(database)
        .await
    {
        user
    } else {
        None
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResponseUser {
    code: u16,
    user: UserInfo,
}

#[utoipa::path(
    get,
    path = "/api/v1/user/{account}",
    params(
        ("account" = String, Path, description = "user account")
    ),
    responses(
        (status = 200, description = "get detail user information", body = ResponseUser)
    )
)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct UserNew {
    account: String,
    password: String,
    #[schema(example = json!({"storage": [1], "nbits": 8}))]
    permission: BitVec,
    username: String,
    worker_id: String,
    title: Option<String>,
    department: Option<String>,
    phone: String,
    email: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/user",
    request_body = UserNew,
    responses(
        (status = 200, description = "add user success", body = ApiResponse, example = json!(ApiResponse {
            code: 200,
            message: Some(String::from("success")),
        })),
        (status = 400, description = "user exist, ", body = ApiResponse, example = json!(ApiResponse {
            code: 400,
            message: Some(String::from("..."))
        })),
        (status = 500, description = "server DB error, ", body = ApiResponse, example = json!(ApiResponse {
            code: 500,
            message: Some(String::from("..."))
        })),
    ),
)]
pub(crate) async fn post_signup_api(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    Json(user): Json<UserNew>,
) -> impl IntoResponse {
    let mut resp = ApiResponse {
        code: 200,
        message: Some(String::from("success")),
    };

    if query_user(&user.account, &database).await.is_some() {
        resp.code = 400;
        resp.message = Some("user exist".to_string());
        return (StatusCode::OK, Json(resp)).into_response();
    }

    let title_id = match user.title {
        Some(title) => match title_id_or_insert(&database, &title).await {
            Ok(id) => id,
            Err(e) => {
                resp.message = Some(format!("{e}"));
                resp.code = 500;
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => 0,
    };

    let department_id = match user.department {
        Some(department) => match department_id_or_insert(&database, &department).await {
            Ok(id) => id,
            Err(e) => {
                resp.message = Some(format!("{e}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
        Ok(_session_token) => (StatusCode::OK, Json(resp)).into_response(),
        Err(error) => {
            resp.message = Some(format!("{error}"));
            resp.code = 500;
            error!("{:?}", &resp);
            (StatusCode::OK, Json(resp)).into_response()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct UserLogin {
    account: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams, Default)]
pub struct ResponseUserLogin {
    code: u16,
    session_key: Option<String>,
    session_value: Option<String>,
    message: Option<String>,
    permission: Option<BitVec>,
}

impl  ResponseUserLogin {
    fn new(code: u16,
           skey: Option<String>, sval: Option<String>,
           msg: Option<String>, permission: Option<BitVec>,
           ) -> Self {
        Self {
            code,
            session_key: skey,
            session_value: sval,
            message: msg,
            permission,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/login",
    request_body = UserLogin,
    responses(
        (status = 200, description = "login success, return cookie session key/value", body = ResponseUserLogin,
             example = json!(ResponseUserLogin::new(200, Some(String::from("cookie key")), Some(String::from("cookie value")), Some(String::from("...")), None))),
        (status = 404, description = "user not found, ", body = ResponseUserLogin,
            example = json!(ResponseUserLogin::new(404, None, None, Some(String::from("...")), None))),
    )
)]
pub(crate) async fn post_login_api(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    Json(user): Json<UserLogin>,
) -> impl IntoResponse {
    match login(&database, random, &user.account, &user.password).await {
        Ok((session_token, permission)) => {
            let _ = update_login_at(&database, &user.account).await;

            let token = session_token.into_cookie_value();
            /*let resp = ResponseUserLogin::new(200, Some(USER_COOKIE_NAME.to_string()), Some(token.clone()), None, Some(permission));
            (StatusCode::OK, Json(resp)).into_response()*/
            let resp = json!({
                "code": 200,
                "session_key": USER_COOKIE_NAME,
                "session_value": &token,
                "permission": permission,
            });
            let cookie = format!("{}={}; Max-Age=COOKIE_MAX_AGE",
                                 USER_COOKIE_NAME,
                                 token);
            Response::builder()
                .status(http::StatusCode::OK)
                .header("Location", "/")
                .header("content-type", "application/json")
                .header("Set-Cookie", cookie)
                .body(resp.to_string())
                .unwrap()
        }
        Err(error) => {
            /*let resp = ResponseUserLogin::new(404, None, None, Some(format!("{}", error)), None);
            (StatusCode::NOT_FOUND, Json(resp)).into_response()*/
            Response::builder()
                .status(http::StatusCode::OK)
                .header("Location", "/")
                .header("content-type", "application/json")
                .body(format!("{{\"code\": 404, \"message\": \"{error}\"}}"))
                .unwrap()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams, Default)]
pub struct ApiResponse {
    code: u16,
    message: Option<String>,
}

impl ApiResponse {
    pub fn new(code: u16, message: Option<String>) -> Self {
        Self {
            code,
            message,
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/user/{account}",
    params(
        ("account" = String, Path, description = "user to delete")
    ),
    responses(
        (status = 200, description = "delete success", body = ApiResponse, example = json!(ApiResponse {
            code: 200,
            message: Some(String::from("success")),
        })),
        (status = 404, description = "user not found, ", body = ApiResponse, example = json!(ApiResponse {
            code: 404,
            message: Some(String::from("..."))
        })),
        (status = 405, description = "permission deny", body = ApiResponse, example = json!(ApiResponse {
            code: 405,
            message: Some(String::from("..."))
        })),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn post_delete_api(
    Extension(mut current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(account): Path<String>,
) -> impl IntoResponse {
    let mut resp = ApiResponse {
        code: 200,
        message: Some(String::from("success")),
    };

    let target = match query_user(&account, &database).await {
        Some(u) => u,
        None => {
            resp.code = 404;
            resp.message = Some("user not found".to_string());
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };
    let allow = permission_check(current_user.get_user().await, &target);

    if allow == false {
        resp.code = 405;
        resp.message = Some(String::from("permission deny"));
        return (StatusCode::OK, Json(resp)).into_response();
    }

    match delete_user2(&database, &account).await {
        Ok(_) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => {
            resp.code = 500;
            resp.message = Some(format!("{e}"));
            (StatusCode::OK, Json(resp)).into_response()
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/me",
    responses(
        (status = 200, description = "get detail user information", body = ResponseUser),
        (status = 400, description = "not login, ", body = ApiResponse, example = json!(ApiResponse {
            code: 400,
            message: Some(String::from("..."))
        })),
        (status = 404, description = "user not found, ", body = ApiResponse, example = json!(ApiResponse {
            code: 404,
            message: Some(String::from("..."))
        })),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn me_api(
    Extension(mut current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
) -> impl IntoResponse {
    if let Some(user) = current_user.get_user().await {
        if let Some(user) = query_user(&user.account, &database).await {
            let resp = ResponseUser { code: 200, user };
            (StatusCode::OK, Json(resp)).into_response()
        } else {
            let resp = ApiResponse {
                code: 404,
                message: Some("user not found?".to_string()),
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
    } else {
        let resp = ApiResponse {
            code: 400,
            message: Some(format!("{}", &NotLoggedIn)),
        };
        (StatusCode::OK, Json(resp)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResponseUsers {
    code: u16,
    users: Option<Vec<UserInfo>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/user",
    responses(
        (status = 200, description = "get user list", body = ResponseUsers)
    )
)]
pub(crate) async fn users_api(Extension(database): Extension<Database>) -> impl IntoResponse {
    //const QUERY: &str = "SELECT username FROM users LIMIT 100;";
    const QUERY: &str = "SELECT u.account, u.permission, u.username, u.worker_id, t.name title, d.name department, phone, u.email, u.create_at, u.login_at FROM users u INNER JOIN titles t ON t.id = u.title_id INNER JOIN departments d ON d.id = u.department_id";

    if let Ok(users) = sqlx::query_as::<_, UserInfo>(QUERY)
        .fetch_all(&database)
        .await
    {
        let resp = ResponseUsers {
            code: 200,
            users: Some(users),
        };
        (StatusCode::OK, Json(resp)).into_response()
    } else {
        let resp = ResponseUsers {
            code: 404,
            users: None,
        };
        (StatusCode::OK, Json(resp)).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/logout",
    responses(
        (status = 200, description = "logout success", body = ResponseUserLogin),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn logout_response_api(
    Extension(_current_user): Extension<AuthState>,
) -> impl IntoResponse {
    let resp = json!({
        "code": 200,
        "session_key": USER_COOKIE_NAME,
        "session_value": "_",
    });
    //(StatusCode::OK, Json(resp)).into_response()
    Response::builder()
        .status(http::StatusCode::OK)
        .header("Location", "/")
        .header("content-type", "application/json")
        .header("Set-Cookie", format!("{}=_; Max-Age=0", USER_COOKIE_NAME,))
        .body(resp.to_string())
        .unwrap()
}

#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct UpdateMe {
    password: Option<String>,
    username: Option<String>,
    phone: Option<String>,
    email: Option<String>,
}

#[utoipa::path(
    put,
    path = "/api/v1/me",
    request_body = UpdateMe,
    responses(
        (status = 200, description = "delete success", body = ApiResponse, example = json!(ApiResponse {
            code: 200,
            message: Some(String::from("success")),
        })),
        (status = 405, description = "permission deny, ", body = ApiResponse, example = json!(ApiResponse {
            code: 405,
            message: Some(String::from("..."))
        })),
        (status = 500, description = "server error, ", body = ApiResponse, example = json!(ApiResponse {
            code: 500,
            message: Some(String::from("..."))
        })),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn update_myself_api(
    Extension(mut current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Json(user): Json<UpdateMe>,
) -> impl IntoResponse {
    let mut resp = ApiResponse {
        code: 200,
        message: None,
    };
    let account = if let Some(myself) = current_user.get_user().await {
        myself.account.clone()
    } else {
        resp.code = 405;
        resp.message = Some("permission deny".to_string());
        return (StatusCode::OK, Json(resp)).into_response();
    };

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
                        resp.code = 500;
                        resp.message = Some(format!("update password fail {err}"));
                    }
                }
            } else {
                resp.message = Some(format!("password hashed fail"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update username fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update phone fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update email fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
            }
        }
    }

    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
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

#[derive(Debug)]
enum PermissionRole {
    Admin(String),
    Gm(String),
    Maintainer(String),
    Comissioner(String),
    Jshall(String),
    Other(String),
}

impl From<&BitVec> for PermissionRole {
    fn from(p: &BitVec) -> Self {
        if let Some(r) = p.get(0) {
            if r == true {
                return Self::Admin("admin".to_string());
            }
        }
        if let Some(r) = p.get(1) {
            if r == true {
                return Self::Gm("GM".to_string());
            }
        }
        if let Some(r) = p.get(2) {
            if r == true {
                return Self::Maintainer("Maintainer".to_string());
            }
        }
        if let Some(r) = p.get(3) {
            if r == true {
                return Self::Comissioner("Comissioner".to_string());
            }
        }
        if let Some(r) = p.get(4) {
            if r == true {
                return Self::Jshall("JSHall".to_string());
            }
        }
        Self::Other("other".to_string())
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/user/{account}",
    params(
        ("account" = String, Path, description = "user to update")
    ),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "delete success", body = ApiResponse, example = json!(ApiResponse {
            code: 200,
            message: Some(String::from("success")),
        })),
        (status = 404, description = "user not found, ", body = ApiResponse, example = json!(ApiResponse {
            code: 404,
            message: Some(String::from("..."))
        })),
        (status = 405, description = "permission deny, ", body = ApiResponse, example = json!(ApiResponse {
            code: 405,
            message: Some(String::from("..."))
        })),
        (status = 500, description = "server error, ", body = ApiResponse, example = json!(ApiResponse {
            code: 500,
            message: Some(String::from("..."))
        })),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn update_user_api(
    Extension(mut current_user): Extension<AuthState>,
    Path(account): Path<String>,
    Extension(database): Extension<Database>,
    Json(user): Json<UpdateUser>,
) -> impl IntoResponse {
    let mut resp = ApiResponse {
        code: 200,
        message: Some(String::from("success")),
    };

    let target = match query_user(&account, &database).await {
        Some(u) => u,
        None => {
            resp.code = 404;
            resp.message = Some("user not found".to_string());
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    /*let allow = if let Some(current) = current_user.get_user().await {
        let current_role = PermissionRole::from(&current.permission);
        let target_role = PermissionRole::from(&target.permission);

        info!("TODO, user-{}/{:?} is {:?} try to change {:?}",
              current.account, current.permission,
              current_role, target_role);

        match (current_role, target_role) {
            (PermissionRole::Admin(role), _) => {
                info!("{role} change anything");
                true
            },
            (PermissionRole::Gm(role), PermissionRole::Admin(_)) => {
                error!("{role} can't change admin");
                false
            },
            (PermissionRole::Gm(role), PermissionRole::Gm(_)) => {
                if target.account != current.account {
                    error!("{role} can't change other GM");
                    false
                } else {
                    info!("{role} change herself");
                    true
                }
            },
            (PermissionRole::Gm(role), _) => {
                info!("{role} change other");
                true
            },
            (_, _) => {
                error!("staff can't change each other");
                false
            }
        }

    } else {
        error!("TODO, not login");
        false
    };*/
    let allow = permission_check(current_user.get_user().await, &target);

    if allow == false {
        resp.code = 405;
        resp.message = Some(String::from("permission deny"));
        return (StatusCode::OK, Json(resp)).into_response();
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

                        resp.code = 500;
                        resp.message = Some(format!("update password fail {err}"));
                    }
                }
            } else {
                resp.message = Some(format!("password hashed wrong"));
                resp.code = 400;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update permission fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update username fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update worker_id fail {err}"));
                resp.code = 400;
                error!("{:?}", &resp);
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
                        resp.message = Some(format!("update title fail {err}"));
                        resp.code = 500;
                        error!("{:?}", &resp);
                    }
                }
            }
            Err(e) => {
                //error!("title-id fail - {e}")
                resp.message = Some(format!("title-id fail {e}"));
                resp.code = 500;
                error!("{:?}", &resp);
            }
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
                        resp.message = Some(format!("update department fail {err}"));
                        resp.code = 500;
                        error!("{:?}", &resp);
                    }
                }
            }
            Err(e) => {
                resp.message = Some(format!("department-id fail {e}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update phone fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
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
                resp.message = Some(format!("update email fail {err}"));
                resp.code = 500;
                error!("{:?}", &resp);
            }
        }
    }

    (StatusCode::OK, Json(resp)).into_response()
}

fn permission_check(current: Option<&CurrentUser>, target: &UserInfo) -> bool {
    if let Some(current) = current {
        let current_role = PermissionRole::from(&current.permission);
        let target_role = PermissionRole::from(&target.permission);

        info!(
            "TODO, user-{}/{:?} is {:?} try to change {:?}",
            current.account, current.permission, current_role, target_role
        );

        if target.account == current.account {
            debug!("{} modify myself is OK", current.account);
            return true;
        }

        match (current_role, target_role) {
            (PermissionRole::Admin(role), _) => {
                info!("{role} change anything");
                true
            }
            (PermissionRole::Gm(role), PermissionRole::Admin(_)) => {
                error!("{role} can't change admin");
                false
            }
            (PermissionRole::Gm(role), PermissionRole::Gm(_)) => {
                if target.account != current.account {
                    error!("{role} can't change other GM");
                    false
                } else {
                    info!("{role} change herself");
                    true
                }
            }
            (PermissionRole::Gm(role), _) => {
                info!("{role} change other");
                true
            }
            (_, _) => {
                error!("staff can't change each other");
                false
            }
        }
    } else {
        error!("TODO, not login");
        false
    }
}

async fn update_login_at(
    database: &Database,
    account: &str,
) -> Result<()> {
    let fetch_one: Result<(i32,), _> = sqlx::query_as(
        "UPDATE users SET login_at = $1 WHERE account = $2 RETURNING id;",
        )
        .bind(&Utc::now())
        .bind(account)
        .fetch_one(database)
        .await;
    match fetch_one {
        Ok((id,)) => {
            debug!("update users/login_at ok {id}");
            Ok(())
        },
        Err(err) => {
            error!("update users/login_at fail {err}");
            Err(anyhow!("{err}"))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    permission: BitVec,
}
