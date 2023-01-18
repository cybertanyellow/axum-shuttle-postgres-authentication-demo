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
    //debug,
    info,
    error,
};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use sqlx::{
    Row,
    //postgres::PgRow,
};

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
struct DepartmentTypeRaw {
    id: i32,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DepartmentRawInfo {
    id: i32,
    create_at: DateTime<Utc>,
    update_at: Option<DateTime<Utc>>,

    shorten: String,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_id: Option<i32>,
    parent_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DepartmentSummary {
    shorten: String,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DepartmentInfo {
    create_at: DateTime<Utc>,
    update_at: Option<DateTime<Utc>>,

    shorten: String,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_id: Option<String>,
    parent: Option<String>,
    childs: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentUpdate {
    shorten: Option<String>,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_id: Option<String>,
    parent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentNew {
    shorten: String,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_id: Option<String>,
    parent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepartmentResponse {
    code: u16,
    department: Option<DepartmentInfo>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepartmentsResponse {
    code: u16,
    departments: Option<Vec<DepartmentSummary>>,
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

    let shorten = department.shorten
        .or(Some(orig.shorten));
    let store_name = department.store_name
        .or(orig.store_name);
    let address = department.address
        .or(orig.address);
    let owner = department.owner
        .or(orig.owner);
    let telephone = department.telephone
        .or(orig.telephone);
    let parent_id = match department.parent {
        None => orig.parent_id,
        Some(p) => {
            query_raw_department(&database, &p)
                .await
                .map_or(None, |v| Some(v.id))
        },
    };
    let type_id = match department.type_id {
        Some(type_id) => match department_type_or_insert(&database, &type_id).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.type_id,
    };

    const UPDATE_QUERY: &str = r#"
        UPDATE departments SET 
            update_at = $1,
            store_name = $2,
            address = $3,
            owner = $4,
            telephone = $5,
            type_id = $6,
            parent_id = $7,
            shorten = $8
        WHERE shorten = $8 RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(UPDATE_QUERY)
        .bind(Utc::now())
        .bind(store_name)
        .bind(address)
        .bind(owner)
        .bind(telephone)
        .bind(type_id)
        .bind(parent_id)
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
            shorten,
            store_name,
            owner,
            telephone,
            t.name AS type_id,
            address
        FROM departments d
            LEFT JOIN department_types t ON t.id = d.type_id
        LIMIT $1 OFFSET $2;
    "#;

    if let Ok(departments) = sqlx::query_as::<_, DepartmentSummary>(QUERY)
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

    let type_id = match department.type_id {
        Some(type_id) => match department_type_or_insert(&database, &type_id).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => None,
    };

    let parent_id = match department.parent {
        Some(p_shorten) => {
            query_parent_id(&database, &p_shorten).await
        },
        None => None,
    };

    const INSERT_QUERY: &str = r#"
        INSERT INTO departments (
            shorten,
            store_name,
            owner,
            telephone,
            type_id,
            parent_id,
            address
        ) VALUES (
            $1, $2, $3
        ) RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
        .bind(department.shorten)
        .bind(department.store_name)
        .bind(department.owner)
        .bind(department.telephone)
        .bind(type_id)
        .bind(parent_id)
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
    /*const QUERY: &str = r#"
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
        }*/
    match query_raw_department(database, shorten)
        .await {
            Some(raw) => {
                let department_type = match raw.type_id {
                    Some(id) => {
                        query_department_type(database, id).await
                            .map(|t| t.name)
                    },
                    None => None,
                };

                let parent = match raw.parent_id {
                    Some(id) => {
                        query_parent_shorten(database, id).await
                    },
                    None => None,
                };
                let childs = query_childs(database, raw.id).await;
                Some(DepartmentInfo {
                    create_at: raw.create_at,
                    update_at: raw.update_at,
                    shorten: raw.shorten,
                    store_name: raw.store_name,
                    owner: raw.owner,
                    telephone: raw.telephone,
                    address: raw.address,
                    type_id: department_type,
                    parent,
                    childs,
                })
            },
            None => None,
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

#[allow(dead_code)]
pub(crate) async fn department_shorten_or_insert(
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

pub(crate) async fn department_name_or_insert(
    database: &Database,
    name: &str
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE name = $1;";
    let department: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(name)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = department {
        Ok(id)
    } else {
        let shorten = Uuid::new_v4();
        const INSERT_QUERY: &str = r#"
            INSERT INTO departments (
                shorten,
                name
            ) VALUES (
                $1,
                $2
            ) RETURNING id;
        "#;
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(shorten.to_string())
            .bind(name)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((department_id,)) => Ok(department_id),
            Err(err) => Err(anyhow!("insert department fail - {err}")),
        }
    }
}

#[allow(dead_code)]
async fn query_department_type(
    database: &Database,
    id: i32,
) -> Option<DepartmentTypeRaw> {
    const QUERY: &str = "SELECT * FROM department_types WHERE id = $1;";

    match sqlx::query_as::<_, DepartmentTypeRaw>(QUERY)
        .bind(id)
        .fetch_optional(database)
        .await {
            Ok(res) => res,
            _ => None,
        }
}

#[allow(dead_code)]
async fn query_parent_shorten(
    database: &Database,
    id: i32,
) -> Option<String> {
    const QUERY: &str = "SELECT shorten FROM departments WHERE id = $1;";

    let row: Result<(String,), _> = sqlx::query_as(QUERY)
        .bind(id)
        .fetch_one(database)
        .await;
    match row {
        Ok(r) => Some(r.0),
        Err(_) => None,
    }
}

#[allow(dead_code)]
async fn query_parent_id(
    database: &Database,
    shorten: &str,
) -> Option<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE shorten = $1;";

    let row: Result<(String,), _> = sqlx::query_as(QUERY)
        .bind(shorten)
        .fetch_one(database)
        .await;
    match row {
        Ok(r) => {
            info!("TODO, check String-{:?} to integer?", r.0);
            r.0.parse::<i32>()
                .map_or(None, |i| Some(i))
        },
        Err(_) => None,
    }
}

#[allow(dead_code)]
async fn query_childs(
    database: &Database,
    pid: i32,
) -> Option<Vec<String>> {
    const QUERY: &str = "SELECT shorten FROM departments WHERE parent_id = $1;";

    /*match sqlx::query(QUERY)
        .bind(pid)
        .map(|row: PgRow| {
            row.get("shorten") as String
        })
        .fetch_all(database)
        .await {
            Ok(res) => Some(res),
            _ => None,
        }
        */
    match sqlx::query(QUERY).bind(pid).fetch_all(database).await {
        Ok(rows) => {
            let res = rows
                .iter()
                .map(|r| format!("{}", r.get::<String, _>("name")))
                .collect::<Vec<String>>();
            Some(res)
        },
        Err(_) => None,
    }
}

#[allow(dead_code)]
pub(crate) async fn department_type_or_insert(
    database: &Database,
    name: &str
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM department_types WHERE name = $1;";
    let d_type: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(name)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = d_type {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = r#"
            INSERT INTO department_types (
                name
            ) VALUES (
                $1
            ) RETURNING id;
        "#;
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(name)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((d_type,)) => Ok(d_type),
            Err(err) => Err(anyhow!("insert department-type fail - {err}")),
        }
    }
}

