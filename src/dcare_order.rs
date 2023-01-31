use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};

use anyhow::{anyhow, Result};
use bit_vec::BitVec;
use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use serde::{/*serde_if_integer128, */ Deserialize, Serialize};
//use serde_json::json;
use tracing::error;
use utoipa::{IntoParams, ToSchema};

use crate::authentication::AuthState;

use crate::dcare_user::query_user_id;
use crate::department::department_name_or_insert;
use crate::errors::NotLoggedIn;
use crate::{ApiResponse, Database, Pagination, Random};

type Price = i32;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct OrderDeleteRes {
    id: i32,
}

//struct Price(i32);
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderRawInfo {
    id: i32,
    issue_at: DateTime<Utc>,

    issuer_id: Option<i32>,
    sn: Option<String>,

    department_id: Option<i32>,
    contact_id: Option<i32>,
    customer_name: Option<String>,
    customer_phone: String,
    customer_address: Option<String>,

    model_id: Option<i32>,

    purchase_at: Option<NaiveDate>,
    accessory_id1: Option<i32>,
    accessory_id2: Option<i32>,
    accessory_other: Option<String>,
    appearance: BitVec,
    appearance_other: Option<String>,
    service: Option<String>,
    fault_id1: Option<i32>,
    fault_id2: Option<i32>,
    fault_other: Option<String>,
    photo_url: Option<String>,
    remark: Option<String>,
    cost: Option<i32>,
    prepaid_free: Option<i32>,

    status_id: Option<i32>,
    servicer_id: Option<i32>,
    maintainer_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct OrderInfo {
    sn: Option<String>,
    issue_at: DateTime<Utc>,

    department: Option<String>,
    contact: Option<String>,
    customer_name: Option<String>,
    customer_phone: String,
    customer_address: Option<String>,

    brand: String,
    model: Option<String>,

    purchase_at: Option<NaiveDate>,
    accessory1: Option<String>,
    accessory2: Option<String>,
    accessory_other: Option<String>,
    #[schema(example = json!({"storage": [1], "nbits": 8}))]
    appearance: BitVec,
    appearance_other: Option<String>,
    service: Option<String>,
    fault1: Option<String>,
    fault2: Option<String>,
    fault_other: Option<String>,
    photo_url: Option<String>,
    remark: Option<String>,
    cost: Option<i32>,
    prepaid_free: Option<i32>,

    status: String,
    servicer: Option<String>,
    maintainer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct OrderUpdate {
    #[schema(example = "department's store_name")]
    department: Option<String>,
    customer_address: Option<String>,

    accessory1: Option<String>,
    accessory2: Option<String>,
    accessory_other: Option<String>,
    #[schema(example = json!({"storage": [2], "nbits": 8}))]
    appearance: Option<BitVec>,
    appearance_other: Option<String>,
    service: Option<String>,
    fault1: Option<String>,
    fault2: Option<String>,
    fault_other: Option<String>,
    photo_url: Option<String>,
    remark: Option<String>,
    cost: Option<i32>,
    prepaid_free: Option<i32>,

    status: Option<String>,
    #[schema(example = "user's account")]
    servicer: Option<String>,
    #[schema(example = "user's account")]
    maintainer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct OrderNew {
    #[schema(example = "department's store_name")]
    department: String,
    #[schema(example = "user's account")]
    contact: Option<String>,
    customer_name: Option<String>,
    customer_phone: String,
    customer_address: Option<String>,

    brand: String,
    model: Option<String>,

    #[schema(example = "2023-01-18")]
    purchase_at: Option<NaiveDate>,
    accessory1: Option<String>,
    accessory2: Option<String>,
    accessory_other: Option<String>,
    #[schema(example = json!({"storage": [1], "nbits": 8}))]
    appearance: BitVec,
    appearance_other: Option<String>,
    service: Option<String>,
    fault1: Option<String>,
    fault2: Option<String>,
    fault_other: Option<String>,
    photo_url: Option<String>,
    remark: Option<String>,
    cost: Option<i32>,
    prepaid_free: Option<i32>,

    status: String,
    #[schema(example = "user's account")]
    servicer: Option<String>,
    #[schema(example = "user's account")]
    maintainer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    code: u16,
    order: Option<OrderInfo>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct OrderSummary {
    sn: Option<String>,
    issue_at: DateTime<Utc>,

    department: Option<String>,
    contact: Option<String>,
    customer_name: Option<String>,
    customer_phone: Option<String>,

    service: Option<String>,
    cost: Option<i32>,

    status: String,
    servicer: Option<String>,
    maintainer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrdersResponse {
    code: u16,
    orders: Option<Vec<OrderSummary>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/order/{sn}",
    params(
        ("sn" = String, Path, description = "order serial-number")
    ),
    responses(
        (status = 200, description = "get detail order information", body = OrderResponse)
    )
)]
pub(crate) async fn order_request(
    Extension(_auth_state): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(sn): Path<String>,
) -> impl IntoResponse {
    let mut resp = OrderResponse {
        code: 400,
        order: None,
    };

    if let Some(o) = query_order(&database, &sn).await {
        resp.code = 200;
        resp.order = Some(o);
    }

    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    put,
    path = "/api/v1/order/{sn}",
    params(
        ("sn" = String, Path, description = "order serial-number")
    ),
    request_body = OrderUpdate,
    responses(
        (status = 200, description = "update success", body = ApiResponse, example = json!(ApiResponse::new(200, Some(String::from("success"))))),
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
    Path(sn): Path<String>,
    Extension(database): Extension<Database>,
    Json(order): Json<OrderUpdate>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, None);

    let issuer = if let Some(user) = current_user.get_user().await {
        user
    } else {
        resp.update(400, Some(format!("{}", &NotLoggedIn)));
        return (StatusCode::OK, Json(resp)).into_response();
    };

    let orig = match query_raw_order(&database, &sn).await {
        Some(orig) => orig,
        None => {
            resp.update(404, Some(format!("order/{sn} not found")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let department_id = match order.department {
        Some(department) => match department_name_or_insert(&database, &department).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.department_id,
    };

    let accessory_id1 = match order.accessory1 {
        Some(ref item) => match accessory_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.accessory_id1,
    };

    let accessory_id2 = match order.accessory2 {
        Some(ref item) => match accessory_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.accessory_id2,
    };

    let appearance = order.appearance.or(Some(orig.appearance));

    let appearance_other = if order.appearance_other.is_some() {
        order.appearance_other
    } else {
        orig.appearance_other
    };

    let fault_id1 = match order.fault1 {
        Some(ref item) => match fault_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.fault_id1,
    };

    let fault_id2 = match order.fault2 {
        Some(ref item) => match fault_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.fault_id2,
    };

    let status_id = match order.status {
        Some(ref status) => match status_id_or_insert(&database, status).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => orig.status_id,
    };

    let servicer_id = if let Some(ref servicer) = order.servicer {
        match query_user_id(&database, servicer).await {
            Some(id) => Some(id),
            None => {
                resp.update(400, Some("servicer staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        orig.servicer_id
    };

    let maintainer_id = if let Some(ref maintainer) = order.maintainer {
        match query_user_id(&database, maintainer).await {
            Some(id) => Some(id),
            None => {
                resp.update(400, Some("maintainer staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        orig.maintainer_id
    };

    let customer_address = if order.customer_address.is_some() {
        order.customer_address
    } else {
        orig.customer_address
    };

    let accessory_other = order.accessory_other.or(orig.accessory_other);
    let service = order.service.or(orig.service);
    let fault_other = order.fault_other.or(orig.fault_other);
    let photo_url = order.photo_url.or(orig.photo_url);
    let remark = order.remark.or(orig.remark);
    let cost = order.cost.or(orig.cost);
    let prepaid_free = order.prepaid_free.or(orig.prepaid_free);

    const UPDATE_QUERY: &str = r#"
        WITH order_updated AS (
            UPDATE orders SET 
                department_id = $1,
                customer_address = $2,
                accessory_id1 = $3,
                accessory_id2 = $4,
                accessory_other = $5,
                appearance = $6,
                appearance_other = $7,
                service = $8,
                fault_id1 = $9,
                fault_id2 = $10,
                fault_other = $11,
                photo_url = $12,
                remark = $13,
                cost = $14,
                prepaid_free = $15,
                status_id = $16,
                servicer_id = $17,
                maintainer_id = $18
            WHERE sn = $19 RETURNING id
        )
        INSERT INTO order_histories (
            order_id,
            issuer_id,
            status_id,
            remark,
            cost
        ) VALUES (
            (SELECT id FROM order_updated),
            $20,
            $16,
            $13,
            $14
        ) RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(UPDATE_QUERY)
        .bind(department_id)
        .bind(customer_address)
        .bind(accessory_id1)
        .bind(accessory_id2)
        .bind(accessory_other)
        .bind(appearance)
        .bind(appearance_other)
        .bind(service)
        .bind(fault_id1)
        .bind(fault_id2)
        .bind(fault_other)
        .bind(photo_url)
        .bind(remark)
        .bind(cost)
        .bind(prepaid_free)
        .bind(status_id)
        .bind(servicer_id)
        .bind(maintainer_id)
        .bind(sn)
        .bind(issuer.id)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            resp.update(200, Some(format!("order update success - history{id}")));
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
    path = "/api/v1/order/{sn}",
    params(
        ("sn" = String, Path, description = "order(serial-number) to delete")
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
    Extension(mut current_user): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(sn): Path<String>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(400, None);

    let _issuer = if let Some(user) = current_user.get_user().await {
        user
    } else {
        resp.update(400, Some(format!("{}", &NotLoggedIn)));
        return (StatusCode::OK, Json(resp)).into_response();
    };

    let _orig = match query_raw_order(&database, &sn).await {
        Some(orig) => orig,
        None => {
            resp.update(404, Some(format!("order/{sn} not found")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    const QUERY: &str = r#"
        WITH order_hist_deleted AS (
            DELETE FROM order_histories
                WHERE order_id = ( SELECT id FROM orders WHERE sn = $1)
            RETURNING id
        )
        DELETE from orders WHERE sn = $1
        RETURNING id;"#;

    if sqlx::query_as::<_, OrderDeleteRes>(QUERY)
        .bind(sn)
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
    path = "/api/v1/order",
    params(
        Pagination
    ),
    responses(
        (status = 200, description = "get order list", body = OrdersResponse)
    )
)]
pub(crate) async fn order_list_request(
    Extension(database): Extension<Database>,
    pagination: Option<Query<Pagination>>,
) -> impl IntoResponse {
    let mut resp = OrdersResponse {
        code: 400,
        orders: None,
    };

    let (offset, entries) = Pagination::parse(pagination);

    const QUERY: &str = r#"
        SELECT
            o.sn,
            o.issue_at,
            d.store_name AS department,
            u1.username AS contact,
            o.customer_name,
            o.customer_phone,
            o.service,
            o.cost,
            s.flow AS status,
            u2.username AS servicer,
            u3.username AS maintainer
        FROM orders o
            LEFT JOIN departments d ON d.id = o.department_id
            LEFT JOIN status s ON s.id = o.status_id
            LEFT JOIN users u1 ON u1.id = o.contact_id
            LEFT JOIN users u2 ON u2.id = o.servicer_id
            LEFT JOIN users u3 ON u3.id = o.maintainer_id
        LIMIT $1 OFFSET $2;
    "#;

    if let Ok(orders) = sqlx::query_as::<_, OrderSummary>(QUERY)
        .bind(entries)
        .bind(offset)
        .fetch_all(&database)
        .await
    {
        resp.orders = Some(orders);
        resp.code = 200;
    }
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
    Extension(database): Extension<Database>,
    Extension(_random): Extension<Random>,
    Extension(_auth_state): Extension<AuthState>,
    Json(order): Json<OrderNew>,
) -> impl IntoResponse {
    let mut resp = ApiResponse::new(200, Some(String::from("success")));

    let contact_id = if let Some(ref contact) = order.contact {
        match query_user_id(&database, contact).await {
            Some(id) => Some(id),
            None => {
                resp.update(400, Some("contact staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        None /* super user? */
    };

    let department_id = match department_name_or_insert(&database, &order.department).await {
        Ok(id) => Some(id),
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let brand = &order.brand;
    let model = match order.model {
        Some(ref m) => m,
        None => "unknown",
    };
    let model_id = match model_id_or_insert(&database, brand, model, None).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let accessory_id1 = match order.accessory1 {
        Some(ref item) => match accessory_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => None,
    };

    let accessory_id2 = match order.accessory2 {
        Some(ref item) => match accessory_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => None,
    };

    let fault_id1 = match order.fault1 {
        Some(ref item) => match fault_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => None,
    };

    let fault_id2 = match order.fault2 {
        Some(ref item) => match fault_id_or_insert(&database, item, 0).await {
            Ok(id) => Some(id),
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => None,
    };

    let status_id = match status_id_or_insert(&database, &order.status).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let servicer_id = if let Some(ref servicer) = order.servicer {
        match query_user_id(&database, servicer).await {
            Some(id) => Some(id),
            None => {
                resp.update(400, Some("servicer staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        None
    };

    let maintainer_id = if let Some(ref maintainer) = order.maintainer {
        match query_user_id(&database, maintainer).await {
            Some(id) => Some(id),
            None => {
                resp.update(400, Some("maintainer staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        None
    };

    let sn = OrderSN::generate(&database, department_id).await;

    const INSERT_QUERY: &str = r#"
        INSERT INTO orders (
            department_id,
            contact_id,
            customer_name,
            customer_phone,
            customer_address,
            model_id,
            purchase_at,
            accessory_id1,
            accessory_id2,
            accessory_other,
            appearance,
            appearance_other,
            service,
            fault_id1,
            fault_id2,
            fault_other,
            photo_url,
            remark,
            cost,
            prepaid_free,
            status_id,
            servicer_id,
            maintainer_id,
            sn
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8,
            $9, $10, $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21, $22, $23, $24
        ) RETURNING id;"#;
    let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
        .bind(department_id)
        .bind(contact_id)
        .bind(&order.customer_name)
        .bind(&order.customer_phone)
        .bind(&order.customer_address)
        .bind(model_id)
        .bind(order.purchase_at)
        .bind(accessory_id1)
        .bind(accessory_id2)
        .bind(&order.accessory_other)
        .bind(&order.appearance)
        .bind(&order.appearance_other)
        .bind(&order.service)
        .bind(fault_id1)
        .bind(fault_id2)
        .bind(&order.fault_other)
        .bind(&order.photo_url)
        .bind(&order.remark)
        .bind(order.cost)
        .bind(order.prepaid_free)
        .bind(status_id)
        .bind(servicer_id)
        .bind(maintainer_id)
        .bind(&sn.0)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            resp.update(200, Some(format!("order{id} create success")));
        }
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
        }
    }

    (StatusCode::OK, Json(resp)).into_response()
}

async fn model_id_or_insert(
    database: &Database,
    brand: &str,
    model: &str,
    _price: Option<u32>,
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM models WHERE brand = $1 AND model = $2;";
    let m_id: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(brand)
        .bind(model)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = m_id {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = r#"
            INSERT INTO models (
                brand, model
            ) VALUES (
                $1, $2
            ) RETURNING id;"#;

        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(brand)
            .bind(model)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((m_id,)) => Ok(m_id),
            Err(err) => Err(anyhow!("insert model fail - {err}")),
        }
    }
}

async fn accessory_id_or_insert(database: &Database, item: &str, price: Price) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM accessories WHERE item = $1;";
    let accessory: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(item)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = accessory {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = r#"
            INSERT INTO accessories (
                item, price
            ) VALUES (
                $1, $2
            ) RETURNING id;"#;
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(item)
            .bind(price)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((a_id,)) => Ok(a_id),
            Err(err) => Err(anyhow!("insert accessory fail - {err}")),
        }
    }
}

async fn fault_id_or_insert(database: &Database, item: &str, cost: Price) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM faults WHERE item = $1;";
    let f_id: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(item)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = f_id {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = r#"
            INSERT INTO faults (
                item, cost
            ) VALUES (
                $1, $2
            ) RETURNING id;"#;
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(item)
            .bind(cost)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((id,)) => Ok(id),
            Err(err) => Err(anyhow!("insert fault fail - {err}")),
        }
    }
}

async fn status_id_or_insert(database: &Database, flow: &str) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM status WHERE flow = $1;";
    let f_id: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(flow)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = f_id {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = "INSERT INTO status (flow) VALUES ($1) RETURNING id;";
        let fetch_one = sqlx::query_as(INSERT_QUERY)
            .bind(flow)
            .fetch_one(database)
            .await;

        match fetch_one {
            Ok((id,)) => Ok(id),
            Err(err) => Err(anyhow!("insert status fail - {err}")),
        }
    }
}

#[allow(dead_code)]
async fn query_order(database: &Database, sn: &str) -> Option<OrderInfo> {
    const QUERY: &str = r#"
        SELECT
            o.sn,
            o.issue_at,
            d.store_name AS department,
            u1.username AS contact,
            o.customer_name,
            o.customer_phone,
            o.customer_address,
            m.brand,
            m.model,
            o.purchase_at,
            s1.item AS accessory1,
            s2.item AS accessory2,
            o.accessory_other,
            o.appearance,
            o.appearance_other,
            o.service,
            f1.item AS fault1,
            f2.item AS fault2,
            o.fault_other,
            o.photo_url,
            o.remark,
            o.cost,
            o.prepaid_free,
            s.flow status,
            u2.username AS servicer,
            u3.username AS maintainer
        FROM orders o
            LEFT JOIN models m ON m.id = o.model_id
            LEFT JOIN departments d ON d.id = o.department_id
            LEFT JOIN status s ON s.id = o.status_id
            LEFT JOIN users u1 ON u1.id = o.contact_id
            LEFT JOIN accessories s1 ON s1.id = o.accessory_id1
            LEFT JOIN accessories s2 ON s2.id = o.accessory_id2
            LEFT JOIN faults f1 ON f1.id = o.fault_id1
            LEFT JOIN faults f2 ON f2.id = o.fault_id2
            LEFT JOIN users u2 ON u2.id = o.servicer_id
            LEFT JOIN users u3 ON u3.id = o.maintainer_id
        WHERE o.sn = $1;
    "#;

    match sqlx::query_as::<_, OrderInfo>(QUERY)
        .bind(sn)
        .fetch_optional(database)
        .await
    {
        Ok(res) => res,
        _ => None,
    }
}

#[allow(dead_code)]
async fn query_raw_order(database: &Database, sn: &str) -> Option<OrderRawInfo> {
    const QUERY: &str = "SELECT * FROM orders WHERE sn = $1;";

    match sqlx::query_as::<_, OrderRawInfo>(QUERY)
        .bind(sn)
        .fetch_optional(database)
        .await
    {
        Ok(res) => res,
        _ => None,
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderSN(String);

impl OrderSN {
    async fn generate(database: &Database, department_id: Option<i32>) -> Self {
        /* (old) DB 22 12 21 20 17 07 0 => DD YY MM DD hh mm ss 0
         * (new) DD DY MM DD hh XX XX 0
         */
        const QUERY: &str = r#"
            SELECT
                id, (SELECT shorten FROM departments WHERE id = $1)
            FROM orders
            ORDER BY id DESC LIMIT 1;
        "#;
        let res: Result<Option<(i32, String)>, _> = sqlx::query_as(QUERY)
            .bind(department_id)
            .fetch_optional(database)
            .await;
        let (next, shorten) = match res {
            Ok(res) => res.map_or((1, "NN".to_string()), |(i, s)| (i + 1, s)),
            Err(_) => (1, "NN".to_string()),
        };

        let now = Utc::now();

        let res = format!(
            "{shorten:0<3}{y}{mm:02}{dd:02}{hh:02}{next:04}0",
            y = (now.year() % 10),
            mm = now.month(),
            dd = now.day(),
            hh = now.hour()
        );
        Self(res)
    }
}

pub(crate) async fn query_order_by_department_id(database: &Database, did: i32) -> Option<String> {
    const QUERY: &str = "SELECT * FROM orders WHERE department_id = $1;";

    if let Ok(order) = sqlx::query_as::<_, OrderRawInfo>(QUERY)
        .bind(did)
        .fetch_optional(database)
        .await
    {
        order.and_then(|o| o.sn)
    } else {
        None
    }
}

pub(crate) async fn query_order_by_user_id(database: &Database, uid: i32) -> Option<String> {
    const QUERY: &str = r#"
            SELECT * FROM orders
            WHERE
                issuer_id = $1 OR
                contact_id = $1 OR
                servicer_id = $1 OR
                maintainer_id = $1;
        "#;

    if let Ok(order) = sqlx::query_as::<_, OrderRawInfo>(QUERY)
        .bind(uid)
        .fetch_optional(database)
        .await
    {
        order.and_then(|o| o.sn)
    } else {
        None
    }
}

/*#[test]
fn test_format() {
    let shorten = "NN".to_string();
    let next = 99;
    let now = Utc::now();
    let res = format!("{shorten:0<3}{y}{mm:02}{dd:02}{hh:02}{next:04}0",
                      y=(now.year() % 10),
                      mm=now.month(),
                      dd=now.day(),
                      hh=now.hour());

    assert_eq!(res, "NN0309080700990".to_string());
}*/
