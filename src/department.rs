use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
//use serde_json::json;
use tracing::{
    //debug, info,
    error,
};
use utoipa::{IntoParams, ToSchema};

use crate::authentication::{
    //CurrentUser,
    AuthState,
};

use crate::errors::NotLoggedIn;
use crate::{
    Database,
    Pagination, ApiResponse,
};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct DepartmentDeleteRes {
    id: i32,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DepartmentRawInfo {
    id: i32,
    create_at: DateTime<Utc>,

    shorten: String,
    name: Option<String>,
    address: Option<String>,
}


#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DepartmentInfo {
    create_at: DateTime<Utc>,
    update_at: Option<DateTime<Utc>>,

    shorten: String,
    name: Option<String>,
    address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentUpdate {
    shorten: Option<String>,
    name: Option<String>,
    address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentNew {
    shorten: String,
    name: Option<String>,
    address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepartmentResponse {
    code: u16,
    department: Option<DepartmentInfo>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepartmentsResponse {
    code: u16,
    departments: Option<Vec<DepartmentInfo>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/department/{shorten}",
    params(
        ("shorten" = String, Path, description = "department shorten name")
    ),
    responses(
        (status = 200, description = "get detail department information", body = DepartmentResponse)
    )
)]
pub(crate) async fn department_request(
    Extension(_auth_state): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(shorten): Path<String>,
) -> impl IntoResponse {
    let mut resp = DepartmentResponse {
        code: 400,
        department: None,
    };

    match query_department(&database, &shorten).await {
        Some(o) => {
            resp.code = 200;
            resp.department = Some(o);
        },
        None => {},
    }
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    put,
    path = "/api/v1/department/{shorten}",
    params(
        ("shorten" = String, Path, description = "department shorten name")
    ),
    request_body = DepartmentUpdate,
    responses(
        (status = 200, description = "update success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
        (status = 404, description = "department not found, ", body = ApiResponse, example = json!(ApiResponse::new(404, Some(String::from("..."))))),
        (status = 405, description = "permission deny, ", body = ApiResponse, example = json!(ApiResponse::new(405, Some(String::from("..."))))),
        (status = 500, description = "server error, ", body = ApiResponse, example = json!(ApiResponse::new(500, Some(String::from("..."))))),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn department_update(
    Extension(mut current_user): Extension<AuthState>,
    Path(shorten): Path<String>,
    Extension(database): Extension<Database>,
    Json(department): Json<DepartmentUpdate>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, None);

    let _issuer = if let Some(user) = current_user.get_user().await {
        user
    } else {
        resp.update(400, Some(format!("{}", &NotLoggedIn)));
        return (StatusCode::OK, Json(resp)).into_response();
    };

    let orig = match query_raw_department(&database, &shorten).await {
        Some(orig) => orig,
        None => {
            resp.update(404, Some(format!("department{shorten} not found")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let name = department.name
        .or(orig.name);
    let address = department.address
        .or(orig.address);

    const UPDATE_QUERY: &str = r#"
        UPDATE departments SET 
            update_at = $1,
            name = $2,
            address = $3,
        WHERE shorten = $4 RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(UPDATE_QUERY)
        .bind(Utc::now())
        .bind(name)
        .bind(address)
        .bind(&shorten)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            resp.update(200, Some(format!("department{id} update success")));
        },
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
        }
    }
    return (StatusCode::OK, Json(resp)).into_response();
}

#[utoipa::path(
    delete,
    path = "/api/v1/department/{shorten}",
    params(
        ("shorten" = String, Path, description = "department ID to delete")
    ),
    responses(
        (status = 200, description = "delete success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
        (status = 404, description = "department not found, ", body = ApiResponse, example = json!(ApiResponse::new(404, Some(String::from("..."))))),
        (status = 405, description = "permission deny", body = ApiResponse, example = json!(ApiResponse::new(405, Some(String::from("..."))))),
    ),
    security(
        //(), // <-- make optional authentication
        ("logined cookie/session-id" = [])
    ),
)]
pub(crate) async fn department_delete(
    Extension(mut current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(shorten): Path<String>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, None);

    let _issuer = if let Some(user) = current_user.get_user().await {
        user
    } else {
        resp.update(400, Some(format!("{}", &NotLoggedIn)));
        return (StatusCode::OK, Json(resp)).into_response();
    };

    let _orig = match query_raw_department(&database, &shorten).await {
        Some(orig) => orig,
        None => {
            resp.update(404, Some(format!("department{shorten} not found")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    const QUERY: &str = r#"
        DELETE from departments WHERE shorten = $1
        RETURNING id;"#;

    if let Ok(_) = sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
        .bind(&shorten)
        .fetch_all(&database)
        .await
    {
        resp.update(200, Some("delete success".to_string()));
    }
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/department",
    params(
        Pagination
    ),
    responses(
        (status = 200, description = "get department list", body = DepartmentsResponse)
    )
)]
pub(crate) async fn department_list_request(
    Extension(database): Extension<Database>,
    pagination: Option<Query<Pagination>>,
) -> impl IntoResponse {
    let mut resp = DepartmentsResponse {
        code: 400,
        departments: None,
    };

    let (offset, entries) = Pagination::parse(pagination);

    const QUERY: &str = r#"
        SELECT
            create_at,
            update_at,
            shorten,
            name,
            address
        FROM departments
        LIMIT $1 OFFSET $2;
    "#;

    if let Ok(departments) = sqlx::query_as::<_, DepartmentInfo>(QUERY)
        .bind(entries)
        .bind(offset)
        .fetch_all(&database)
        .await
    {
        resp.departments = Some(departments);
        resp.code = 200;
    }
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/department",
    request_body = DepartmentNew,
    responses(
        (status = 200, description = "add department success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
        (status = 400, description = "department exist, ", body = ApiResponse, example = json!(ApiResponse::new(400, Some(String::from("..."))))),
        (status = 500, description = "server DB error, ", body = ApiResponse, example = json!(ApiResponse::new(500, Some(String::from("..."))))),
    ),
)]
pub(crate) async fn department_create(
    Extension(database): Extension<Database>,
    Extension(mut current_user): Extension<AuthState>,
    Json(department): Json<DepartmentNew>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(200, Some(String::from("success")));

    let _issuer = if let Some(user) = current_user.get_user().await {
        user
    } else {
        resp.update(400, Some(format!("{}", &NotLoggedIn)));
        return (StatusCode::OK, Json(resp)).into_response();
    };

    const INSERT_QUERY: &str = r#"
        INSERT INTO departments (
            shorten,
            name,
            address
        ) VALUES (
            $1, $2, $3
        ) RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
        .bind(department.shorten)
        .bind(department.name)
        .bind(department.address)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            resp.update(200, Some(format!("department{id} create success")));
        },
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
        }
    }
    return (StatusCode::OK, Json(resp)).into_response();
}

#[allow(dead_code)]
async fn query_department(
    database: &Database,
    shorten: &str,
) -> Option<DepartmentInfo> {
    const QUERY: &str = r#"
        SELECT
            create_at,
            update_at,
            shorten,
            name,
            address
        FROM departments
        WHERE shorten = $1;
    "#;

    match sqlx::query_as::<_, DepartmentInfo>(QUERY)
        .bind(shorten)
        .fetch_optional(database)
        .await {
            Ok(res) => res,
            _ => None,
        }
}

#[allow(dead_code)]
async fn query_raw_department(
    database: &Database,
    shorten: &str,
) -> Option<DepartmentRawInfo> {
    const QUERY: &str = "SELECT * FROM departments WHERE shorten = $1;";

    match sqlx::query_as::<_, DepartmentRawInfo>(QUERY)
        .bind(shorten)
        .fetch_optional(database)
        .await {
            Ok(res) => res,
            _ => None,
        }
}

pub(crate) async fn department_id_or_insert(
    database: &Database,
    shorten: &str
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE shorten = $1;";
    let department: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(shorten)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = department {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = r#"
            INSERT INTO departments (
                shorten
            ) VALUES (
                $1
            ) RETURNING id;
        "#;
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(shorten)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((department_id,)) => Ok(department_id),
            Err(err) => Err(anyhow!("insert department fail - {err}")),
        }
    }
}

