use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};

use anyhow::{anyhow, Result};
use bit_vec::BitVec;
use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
//use serde_json::json;
use tracing::{
    error,
    //debug,
    info,
};
use utoipa::{IntoParams, ToSchema};
//use uuid::Uuid;
//use sqlx::{
//Row,
//postgres::PgRow,
//};

use crate::authentication::AuthState;
use crate::dcare_order::query_order_by_department_id;
use crate::dcare_user::query_user_by_department_id;

use crate::errors::NotLoggedIn;
use crate::{ApiResponse, Database, Pagination};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct DepartmentOrg(String);

#[derive(Deserialize, IntoParams)]
pub struct DepartmentListQuery {
    offset: Option<i32>,
    entries: Option<i32>,

    shorten: Option<String>,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_mask: Option<BitVec>,
    create_start: Option<NaiveDate>,
    create_end: Option<NaiveDate>,
    update_start: Option<NaiveDate>,
    update_end: Option<NaiveDate>,
}

impl DepartmentListQuery {
    pub fn parse(mine: Option<Query<Self>>) -> (i32, i32, String) {
        if let Some(ref q) = mine {
            let offset = q.offset
                .map_or(0, |o| o);
            let entries = q.entries
                .map_or(100, |e| e);

            let where_is = if let Some(ref p) = q.telephone {
                format!("WHERE o.telephone = '{p}'")
            } else {
                "".to_string()
            };
            let where_is = if let Some(ref s) = q.shorten {
                let sql_d = format!("o.shorten = '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql_d}")
                } else {
                    format!("{where_is} AND {sql_d}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.store_name {
                let sql_d = format!("o.store_name = '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql_d}")
                } else {
                    format!("{where_is} AND {sql_d}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.owner {
                let sql_d = format!("o.owner = '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql_d}")
                } else {
                    format!("{where_is} AND {sql_d}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.address {
                let sql_d = format!("o.address = '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql_d}")
                } else {
                    format!("{where_is} AND {sql_d}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.type_mask {
                let sql_d = format!("o.type_mask & {:?}", s);
                if where_is.is_empty() {
                    format!("WHERE {sql_d}")
                } else {
                    format!("{where_is} AND {sql_d}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.create_start {
                let sql = format!("o.create_at >= '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql}")
                } else {
                    format!("{where_is} AND {sql}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.create_end {
                let sql = format!("o.create_at < '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql}")
                } else {
                    format!("{where_is} AND {sql}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.update_start {
                let sql = format!("o.update_at >= '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql}")
                } else {
                    format!("{where_is} AND {sql}")
                }
            } else {
                where_is
            };
            let where_is = if let Some(ref s) = q.update_end {
                let sql = format!("o.update_at < '{s}'");
                if where_is.is_empty() {
                    format!("WHERE {sql}")
                } else {
                    format!("{where_is} AND {sql}")
                }
            } else {
                where_is
            };


            (offset, entries, where_is)
        } else {
            (0, 100, "".to_string())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct DepartmentOrgRaw {
    id: i32,
    create_at: DateTime<Utc>,
    parent_id: Option<i32>,
    child_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentOrgData {
    current: Option<String>,
    parents: Option<Vec<DepartmentOrg>>,
    childs: Option<Vec<DepartmentOrg>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepartmentOrgResponse {
    code: u16,
    org: Option<DepartmentOrgData>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepartmentOrgsResponse {
    code: u16,
    orgs: Option<Vec<DepartmentOrgData>>,
}

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
    type_mask: BitVec,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DepartmentSummary {
    shorten: String,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_mask: BitVec,
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
    type_mask: BitVec,
    parents: Option<Vec<DepartmentOrg>>,
    childs: Option<Vec<DepartmentOrg>>,
}

impl
    From<(
        DepartmentInfoPartial,
        Option<Vec<DepartmentOrg>>,
        Option<Vec<DepartmentOrg>>,
    )> for DepartmentInfo
{
    fn from(
        p: (
            DepartmentInfoPartial,
            Option<Vec<DepartmentOrg>>,
            Option<Vec<DepartmentOrg>>,
        ),
    ) -> Self {
        DepartmentInfo {
            create_at: p.0.create_at,
            update_at: p.0.update_at,
            shorten: p.0.shorten,
            store_name: p.0.store_name,
            owner: p.0.owner,
            telephone: p.0.telephone,
            address: p.0.address,
            type_mask: p.0.type_mask,
            parents: p.1,
            childs: p.2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DepartmentInfoPartial {
    id: i32,
    create_at: DateTime<Utc>,
    update_at: Option<DateTime<Utc>>,

    shorten: String,
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    type_mask: BitVec,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentUpdate {
    #[schema(example = "ADM, BM, ...")]
    shorten: Option<String>,
    #[schema(example = "full store(department) name")]
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    #[schema(example = "總部(b'10000000),維保中心(b...)")]
    type_mask: Option<BitVec>,
    #[schema(example = r#"["BM"]"#)]
    parents: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentNew {
    #[schema(example = "ADM, BM, ...")]
    shorten: String,
    #[schema(example = "full store(department) name")]
    store_name: Option<String>,
    owner: Option<String>,
    telephone: Option<String>,
    address: Option<String>,
    #[schema(example = "總部(b'10000000),維保中心(b...)")]
    type_mask: BitVec,
    #[schema(example = r#"["BM"]"#)]
    parents: Option<Vec<String>>,
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

    if let Some(o) = query_department(&database, &shorten).await {
        resp.code = 200;
        resp.department = Some(o);
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

    let shorten = department.shorten.or(Some(orig.shorten));
    let store_name = department.store_name.or(orig.store_name);
    let address = department.address.or(orig.address);
    let owner = department.owner.or(orig.owner);
    let telephone = department.telephone.or(orig.telephone);
    let type_mask = department.type_mask.map_or(orig.type_mask, |t| t);

    const UPDATE_QUERY: &str = r#"
        UPDATE departments SET 
            update_at = $1,
            store_name = $2,
            address = $3,
            owner = $4,
            telephone = $5,
            type_mask = $6,
            shorten = $7
        WHERE id = $8 RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(UPDATE_QUERY)
        .bind(Utc::now())
        .bind(store_name)
        .bind(address)
        .bind(owner)
        .bind(telephone)
        .bind(type_mask)
        .bind(&shorten)
        .bind(orig.id)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            let org_done = match department.parents {
                Some(ref parents) => {
                    department_org_update_parents(&database, orig.id, parents).await
                }
                None => Ok(()),
            };
            if org_done.is_ok() {
                resp.update(200, Some(format!("department{id} update success")));
            } else {
                resp.update(400, Some("department organization update fail".to_string()));
            }
        }
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
        }
    }

    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    delete,
    path = "/api/v1/department/{shorten}",
    params(
        ("shorten" = String, Path, description = "department shorten to delete"),
        DepartmentOrgPair,
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
    pair: Query<DepartmentOrgPair>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, None);

    let _issuer = if let Some(user) = current_user.get_user().await {
        user
    } else {
        resp.update(400, Some(format!("{}", &NotLoggedIn)));
        return (StatusCode::OK, Json(resp)).into_response();
    };

    match query_raw_department(&database, &shorten).await {
        Some(orig) => {
            if pair.parent.is_none() && pair.child.is_none() {
                /* check related before deleted it */
                if let Some(user) = query_user_by_department_id(&database, orig.id).await {
                    resp.update(400, Some(format!("reject due to user/{user} related")));
                    error!("{:?}", &resp);
                    return (StatusCode::OK, Json(resp)).into_response();
                }
                if let Some(order) = query_order_by_department_id(&database, orig.id).await {
                    resp.update(400, Some(format!("reject due to order/{order} related")));
                    error!("{:?}", &resp);
                    return (StatusCode::OK, Json(resp)).into_response();
                }
            }
        }
        None => {
            resp.update(404, Some(format!("department{shorten} not found")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    }

    /* manual delete organization....
     * if query_childs(&database, orig.id).await.is_some() {
        resp.update(400, Some("denied by child departments".to_string()));
        error!("{:?}", &resp);
        return (StatusCode::OK, Json(resp)).into_response();
    }*/
    match org_delete(&database, &shorten, pair).await {
        Err(e) => {
            resp.update(400, Some(format!("delete organization pair fail - {e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
        Ok(all) => {
            if !all {
                resp.update(200, Some("delete organization pair success".to_string()));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    }

    const QUERY: &str = r#"
        DELETE from departments WHERE shorten = $1
        RETURNING id;"#;

    if sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
        .bind(&shorten)
        .fetch_all(&database)
        .await
        .is_ok()
    {
        resp.update(200, Some("delete success".to_string()));
    }
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/department",
    params(
        DepartmentListQuery
    ),
    responses(
        (status = 200, description = "get department list", body = DepartmentsResponse)
    )
)]
pub(crate) async fn department_list_request(
    Extension(database): Extension<Database>,
    query: Option<Query<DepartmentListQuery>>,
) -> impl IntoResponse {
    let mut resp = DepartmentsResponse {
        code: 400,
        departments: None,
    };

    let (offset, entries, where_dep) = DepartmentListQuery::parse(query);

    let sselect = format!(r#"
        SELECT
            d.id,
            create_at,
            update_at,
            shorten,
            store_name,
            owner,
            telephone,
            type_mask,
            address
        FROM departments d
        {where_dep} LIMIT {entries} OFFSET {offset};
    "#);

    if let Ok(mut departments) = sqlx::query_as::<_, DepartmentInfoPartial>(&sselect)
        .fetch_all(&database)
        .await
    {
        let mut infos: Vec<DepartmentInfo> = Vec::new();

        while let Some(d) = departments.pop() {
            let parents = query_parent_shorten(&database, d.id).await;
            let childs = query_childs(&database, d.id).await;
            let info = DepartmentInfo::from((d, parents, childs));
            infos.push(info);
        }

        resp.departments = Some(infos);
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
            store_name,
            owner,
            telephone,
            type_mask,
            address
        ) VALUES (
            $1, $2, $3, $4, $5, $6
        ) RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
        .bind(department.shorten)
        .bind(department.store_name)
        .bind(department.owner)
        .bind(department.telephone)
        .bind(department.type_mask)
        .bind(department.address)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            let org_done = match department.parents {
                Some(ref parents) => department_org_update_parents(&database, id, parents).await,
                None => Ok(()),
            };
            if org_done.is_ok() {
                resp.update(200, Some(format!("department{id} create success")));
            } else {
                resp.update(400, Some("department organization update fail".to_string()));
            }
        }
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
        }
    }
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DepartmentOrgPair {
    pub parent: Option<String>,
    pub child: Option<String>,
}

#[utoipa::path(
    delete,
    path = "/api/v1/department/org/{shorten}",
    params(
        ("shorten" = String, Path, description = "department ID to delete"),
        DepartmentOrgPair,
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
#[allow(dead_code)]
pub(crate) async fn department_org_delete(
    Extension(_current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(shorten): Path<String>,
    pair: Query<DepartmentOrgPair>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(200, None);

    if let Some(ref parent) = pair.parent {
        const QUERY: &str = r#"
            DELETE from department_orgs
            WHERE
                child_id = (SELECT id FROM departments WHERE shorten = $1)
                AND
                parent_id = (SELECT id FROM departments WHERE shorten = $2)
            RETURNING id;
        "#;

        if sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
            .bind(&shorten)
            .bind(parent)
            .fetch_all(&database)
            .await
            .is_err()
        {
            resp.update(400, Some("delete department/org parent fail".to_string()));
            return (StatusCode::OK, Json(resp)).into_response();
        } else {
            resp.update(
                200,
                Some("delete department/org parent success".to_string()),
            );
        }
    }
    if let Some(ref child) = pair.child {
        const QUERY: &str = r#"
            DELETE from department_orgs
            WHERE
                child_id = (SELECT id FROM departments WHERE shorten = $1)
                AND
                parent_id = (SELECT id FROM departments WHERE shorten = $2)
            RETURNING id;
        "#;

        if sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
            .bind(child)
            .bind(&shorten)
            .fetch_all(&database)
            .await
            .is_err()
        {
            resp.update(400, Some("delete department/org child fail".to_string()));
            return (StatusCode::OK, Json(resp)).into_response();
        } else {
            resp.update(200, Some("delete department/org child success".to_string()));
        }
    }

    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/department/org",
    params(
        Pagination
    ),
    responses(
        (status = 200, description = "get department orgnization list", body = DepartmentOrgsResponse)
    )
)]
#[allow(dead_code)]
pub(crate) async fn department_org_list_request(
    Extension(_database): Extension<Database>,
    //pagination: Option<Query<Pagination>>,
) -> impl IntoResponse {
    let resp = DepartmentOrgsResponse {
        code: 400,
        orgs: None,
    };

    //let (offset, entries) = Pagination::parse(pagination);
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/department/org/{shorten}",
    params(
        ("shorten" = String, Path, description = "department ID to get"),
    ),
    responses(
        (status = 200, description = "get department orgnization list", body = DepartmentOrgsResponse)
    )
)]
#[allow(dead_code)]
pub(crate) async fn department_org_request(
    Extension(_current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(shorten): Path<String>,
) -> impl IntoResponse {
    let mut resp = DepartmentOrgResponse {
        code: 400,
        org: None,
    };

    resp.org = match query_raw_department(&database, &shorten).await {
        Some(raw) => {
            let parents = query_parent_shorten(&database, raw.id).await;
            let childs = query_childs(&database, raw.id).await;
            Some(DepartmentOrgData {
                current: Some(shorten),
                parents,
                childs,
            })
        }
        None => None,
    };

    (StatusCode::OK, Json(resp)).into_response()
}

#[allow(dead_code)]
async fn query_department(database: &Database, shorten: &str) -> Option<DepartmentInfo> {
    match query_raw_department(database, shorten).await {
        Some(raw) => {
            let parents = query_parent_shorten(database, raw.id).await;
            let childs = query_childs(database, raw.id).await;
            Some(DepartmentInfo {
                create_at: raw.create_at,
                update_at: raw.update_at,
                shorten: raw.shorten,
                store_name: raw.store_name,
                owner: raw.owner,
                telephone: raw.telephone,
                address: raw.address,
                type_mask: raw.type_mask,
                parents,
                childs,
            })
        }
        None => None,
    }
}

#[allow(dead_code)]
async fn query_raw_department(database: &Database, shorten: &str) -> Option<DepartmentRawInfo> {
    const QUERY: &str = "SELECT * FROM departments WHERE shorten = $1;";

    match sqlx::query_as::<_, DepartmentRawInfo>(QUERY)
        .bind(shorten)
        .fetch_optional(database)
        .await
    {
        Ok(res) => res,
        _ => None,
    }
}

#[allow(dead_code)]
pub(crate) async fn department_shorten_or_insert(
    database: &Database,
    shorten: &str,
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

/*pub(crate) async fn department_name_or_insert(database: &Database, name: &str) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE store_name = $1;";
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
                store_name
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
}*/

pub(crate) async fn department_shorten_query(
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
        Err(anyhow!("department/shorten/{shorten} not found"))
    }
}

#[allow(dead_code)]
async fn query_parent_shorten(database: &Database, id: i32) -> Option<Vec<DepartmentOrg>> {
    const QUERY: &str = r#"
        SELECT
            d.shorten
        FROM department_orgs o
        LEFT JOIN departments d ON d.id = o.parent_id
        WHERE o.child_id = $1;
    "#;

    let row = sqlx::query_as::<_, DepartmentOrg>(QUERY)
        .bind(id)
        .fetch_all(database)
        .await;
    match row {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

#[allow(dead_code)]
async fn query_parent_id(database: &Database, shorten: &str) -> Option<i32> {
    const QUERY: &str = "SELECT id FROM departments WHERE shorten = $1;";

    let row: Result<(String,), _> = sqlx::query_as(QUERY)
        .bind(shorten)
        .fetch_one(database)
        .await;
    match row {
        Ok(r) => {
            info!("TODO, check String-{:?} to integer?", r.0);
            r.0.parse::<i32>().map_or(None, |i| Some(i))
        }
        Err(_) => None,
    }
}

#[allow(dead_code)]
async fn query_childs(database: &Database, pid: i32) -> Option<Vec<DepartmentOrg>> {
    /*const QUERY: &str = "SELECT shorten FROM departments WHERE parent_id = $1;";

    match sqlx::query(QUERY).bind(pid).fetch_all(database).await {
        Ok(rows) => {
            let res = rows
                .iter()
                .map(|r| format!("{}", r.get::<String, _>("name")))
                .collect::<Vec<String>>();
            Some(res)
        },
        Err(_) => None,
    }*/
    const QUERY: &str = r#"
        SELECT d.shorten FROM department_orgs o
        LEFT JOIN departments d ON d.id = o.child_id
        WHERE o.parent_id = $1;
    "#;

    let row = sqlx::query_as::<_, DepartmentOrg>(QUERY)
        .bind(pid)
        .fetch_all(database)
        .await;
    match row {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

#[allow(dead_code)]
async fn department_org_update_parents(
    database: &Database,
    id: i32,
    parents: &[String],
) -> Result<()> {
    for p in parents.iter() {
        const QUERY: &str = r#"
            SELECT
                id
            FROM department_orgs
            WHERE parent_id = (
                SELECT
                    id
                FROM departments
                WHERE shorten = $1
            ) AND child_id = $2;
        "#;
        let found: Result<Option<(i32,)>, _> = sqlx::query_as(QUERY)
            .bind(p)
            .bind(id)
            .fetch_optional(database)
            .await;

        if let Ok(Some(_)) = found {
            continue;
        } else {
            const INSERT_QUERY: &str = r#"
                INSERT INTO department_orgs (
                    parent_id, child_id
                ) VALUES (
                    (SELECT id FROM departments WHERE shorten = $1), $2
                ) RETURNING id;
            "#;
            let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
                .bind(p)
                .bind(id)
                .fetch_one(database)
                .await;

            match fetch_one {
                Ok(_) => continue,
                Err(err) => {
                    return Err(anyhow!("insert department-type fail - {err}"));
                }
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
async fn department_org_renew_parents(
    database: &Database,
    id: i32,
    parents: &[String],
) -> Result<()> {
    const QUERY: &str = r#"
        DELETE from department_orgs
        WHERE
        child_id = $1
        RETURNING id;
    "#;

    if sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
        .bind(id)
        .fetch_all(database)
        .await
        .is_err()
    {}

    for p in parents.iter() {
        const QUERY: &str = r#"
            SELECT
                id
            FROM department_orgs
            WHERE parent_id = (
                SELECT
                    id
                FROM departments
                WHERE shorten = $1
            ) AND child_id = $2;
        "#;
        let found: Result<Option<(i32,)>, _> = sqlx::query_as(QUERY)
            .bind(p)
            .bind(id)
            .fetch_optional(database)
            .await;

        if let Ok(Some(_)) = found {
            continue;
        } else {
            const INSERT_QUERY: &str = r#"
                INSERT INTO department_orgs (
                    parent_id, child_id
                ) VALUES (
                    (SELECT id FROM departments WHERE shorten = $1), $2
                ) RETURNING id;
            "#;
            let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
                .bind(p)
                .bind(id)
                .fetch_one(database)
                .await;

            match fetch_one {
                Ok(_) => continue,
                Err(err) => {
                    return Err(anyhow!("insert department-type fail - {err}"));
                }
            }
        }
    }
    Ok(())
}

async fn org_delete(
    database: &Database,
    shorten: &String,
    pair: Query<DepartmentOrgPair>,
) -> Result<bool> {
    let mut do_all = true;
    if let Some(ref parent) = pair.parent {
        const QUERY: &str = r#"
            DELETE from department_orgs
            WHERE
            child_id = (SELECT id FROM departments WHERE shorten = $1)
            AND
            parent_id = (SELECT id FROM departments WHERE shorten = $2)
            RETURNING id;
        "#;

        if let Err(e) = sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
            .bind(shorten)
            .bind(parent)
            .fetch_all(database)
            .await
        {
            return Err(anyhow!({ e }));
        }
        do_all = false;
    }
    if let Some(ref child) = pair.child {
        const QUERY: &str = r#"
            DELETE from department_orgs
            WHERE
            child_id = (SELECT id FROM departments WHERE shorten = $1)
            AND
            parent_id = (SELECT id FROM departments WHERE shorten = $2)
            RETURNING id;
        "#;

        if let Err(e) = sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
            .bind(child)
            .bind(shorten)
            .fetch_all(database)
            .await
        {
            return Err(anyhow!({ e }));
        }
        do_all = false;
    }

    if do_all {
        const QUERY: &str = r#"
            DELETE from department_orgs
            WHERE
                child_id = (SELECT id FROM departments WHERE shorten = $1)
                OR
                parent_id = (SELECT id FROM departments WHERE shorten = $1)
            RETURNING id;
        "#;

        if let Err(e) = sqlx::query_as::<_, DepartmentDeleteRes>(QUERY)
            .bind(shorten)
            .fetch_all(database)
            .await
        {
            return Err(anyhow!({ e }));
        }
    }

    Ok(do_all)
}
