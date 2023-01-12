use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    //response::{Html, Redirect},
    Json,
};

use anyhow::{anyhow, Result};
use bit_vec::BitVec;
use chrono::{DateTime, Utc, NaiveDate};
use serde::{/*serde_if_integer128, */ Deserialize, Serialize};
//use serde_json::json;
use tracing::{
    //debug,
    info, error,
};
use utoipa::{IntoParams, ToSchema};

use crate::authentication::{
    //CurrentUser,
    AuthState,
};
use crate::{Database, Random};
use crate::dcare_user::{
    ApiResponse, query_user_id,
    department_id_or_insert,
};

type Price = i32;
//struct Price(i32);

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct OrderInfo {
    id: i32,
    issue_at: DateTime<Utc>,

    department: Option<String>,
    contact: Option<String>,
    customer_name: Option<String>,
    customer_phone: String,
    customer_address: Option<String>,

    brand: String,
    model: Option<String>,

    purchase_at: Option<NaiveDate>,
    accessory_id1: Option<String>,
    accessory_id2: Option<String>,
    accessory_other: Option<String>,
    appearance: BitVec,
    appearance_other: Option<String>,
    service: Option<String>,
    fault_id1: Option<String>,
    fault_id2: Option<String>,
    fault_other: Option<String>,
    photo_url: Option<String>,
    remark: Option<String>,
    cost: Option<i32>,
    prepaid_free: Option<i32>,

    status: String,
    servicer: Option<String>,
    maintainer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderUpdate {
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct OrderNew {
    //number: String,
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    code: u16,
    order: Option<OrderInfo>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrdersResponse {
    code: u16,
    orders: Option<Vec<OrderInfo>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/order/{id}",
    params(
        ("id" = i32, Path, description = "order ID")
    ),
    responses(
        (status = 200, description = "get detail order information", body = OrderResponse)
    )
)]
pub(crate) async fn order_request(
    Extension(_auth_state): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let mut resp = OrderResponse {
        code: 400,
        order: None,
    };

    const QUERY: &str = "SELECT o.id, o.issue_at, d.name department, o.customer_name, o.customer_phone, o.customer_address, m.brand, m.model, o.purchase_at, o.accessory_other, o.appearance, o.appearance_other, o.service, o.fault_other, o.photo_url, o.remark, o.cost, o.prepaid_free, s.flow status FROM orders o INNER JOIN models m ON m.id = o.model_id INNER JOIN departments d ON d.id = o.department_id INNER JOIN models m ON m.id = o.model_id INNER JOIN status s ON s.id = o.status_id WHERE o.id = $1";

    /* TODO map users/accessories/faults table */

    if let Ok(user) = sqlx::query_as::<_, OrderInfo>(QUERY)
        .bind(id)
        .fetch_optional(&database)
        .await
    {
        resp.order = user;
        resp.code = 200;
    }
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    put,
    path = "/api/v1/order/{id}",
    params(
        ("id" = i32, Path, description = "order ID")
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
    Extension(/*mut */_current_user): Extension<AuthState>,
    Path(_id): Path<i32>,
    Extension(_database): Extension<Database>,
    Json(_order): Json<OrderUpdate>,
) -> impl IntoResponse {
    let resp = ApiResponse::new(400, Some(String::from("TODO")));
    (StatusCode::OK, Json(resp)).into_response()
}

#[utoipa::path(
    delete,
    path = "/api/v1/order/{id}",
    params(
        ("id" = i32, Path, description = "order ID to delete")
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
    Path(_id): Path<i32>,
) -> impl IntoResponse {
    let resp = ApiResponse::new(400, Some(String::from("TODO")));
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
    Extension(database): Extension<Database>
) -> impl IntoResponse {
    let mut resp = OrdersResponse {
        code: 400,
        orders: None,
    };

    /*{
        const TEST1: &str = "SELECT id,issue_at,department_id,contact_id,customer_name,customer_phone,model_id,purchase_at FROM orders;";
        let order: Option<(i32,DateTime<Utc>,i32,i32,Option<String>,String,i32,Option<NaiveDate>)> = sqlx::query_as(TEST1)
            .fetch_optional(&database)
            .await
            .unwrap();
        info!("[debug] order get {:?}", order);
    }*/

    const QUERY: &str = "SELECT o.id, o.issue_at, d.name department, u1.username contact, o.customer_name, o.customer_phone, o.customer_address, m.brand, m.model, o.purchase_at, s1.item accessory_id1, s2.item accessory_id2, o.accessory_other, o.appearance, o.appearance_other, o.service, f1.item fault_id1, f2.item fault_id2, o.fault_other, o.photo_url, o.remark, o.cost, o.prepaid_free, s.flow status, NULL servicer, NULL maintainer FROM orders o INNER JOIN models m ON m.id = o.model_id INNER JOIN departments d ON d.id = o.department_id INNER JOIN status s ON s.id = o.status_id INNER JOIN users u1 ON u1.id = o.contact_id INNER JOIN accessories s1 ON s1.id = o.accessory_id1 INNER JOIN accessories s2 ON s2.id = o.accessory_id2 INNER JOIN faults f1 ON f1.id = o.fault_id1 INNER JOIN faults f2 ON f2.id = o.fault_id1;";

    /* TODO map users/accessories/faults table */

    if let Ok(orders) = sqlx::query_as::<_, OrderInfo>(QUERY)
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

    /*if query_order(&database, &order.number).await.is_some() {
        resp.update(400, Some("order exist".to_string()));
        return (StatusCode::OK, Json(resp)).into_response();
    }*/

    let contact_id = if let Some(ref contact) = order.contact {
        match query_user_id(&database, contact).await {
            Some(id) => id,
            None => {
                resp.update(400, Some("contact staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        1 /* super user? */
    };

    let department_id = match order.department {
        Some(department) => match department_id_or_insert(&database, &department).await {
            Ok(id) => id,
            Err(e) => {
                resp.update(500, Some(format!("{e}")));
                error!("{:?}", &resp);
                return (StatusCode::OK, Json(resp)).into_response();
            }
        },
        None => 0, /*TODO, if row-0 non-exist? */
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

    let item = match order.accessory1 {
        Some(ref a) => a,
        None => "none",
    };
    let accessory_id1 = match accessory_id_or_insert(&database, item, 0).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let item = match order.accessory2 {
        Some(ref a) => a,
        None => "none",
    };
    let accessory_id2 = match accessory_id_or_insert(&database, item, 0).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let item = match order.fault1 {
        Some(ref f) => f,
        None => "none",
    };
    let fault_id1 = match fault_id_or_insert(&database, item, 0).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let item = match order.fault2 {
        Some(ref f) => f,
        None => "none",
    };
    let fault_id2 = match fault_id_or_insert(&database, item, 0).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let status_id = match status_id_or_insert(&database, &order.status).await {
        Ok(id) => id,
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let servicer_id = if let Some(ref servicer) = order.servicer{
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
                resp.update(400, Some("servicer staff not found".to_string()));
                return (StatusCode::OK, Json(resp)).into_response();
            }
        }
    } else {
        None
    };

    const INSERT_QUERY: &str =
        "INSERT INTO orders (department_id, contact_id, customer_name, customer_phone, customer_address, model_id, purchase_at, accessory_id1, accessory_id2, accessory_other, appearance, appearance_other, service, fault_id1, fault_id2, fault_other, photo_url, remark, cost, prepaid_free, status_id, servicer_id, maintainer_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23) RETURNING id;";
    let fetch_one: Result<(i32,), _> = sqlx::query_as(INSERT_QUERY)
        .bind(department_id)
        .bind(contact_id)
        .bind(&order.customer_name)
        .bind(&order.customer_phone)
        .bind(&order.customer_address)
        .bind(model_id)
        .bind(&order.purchase_at)
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
        .bind(&order.cost)
        .bind(&order.prepaid_free)
        .bind(status_id)
        .bind(servicer_id)
        .bind(maintainer_id)
        .fetch_one(&database)
        .await;

    match fetch_one {
        Ok((id,)) => {
            resp.update(200, Some(format!("order{id} create success")));
        },
        Err(e) => {
            resp.update(500, Some(format!("{e}")));
            error!("{:?}", &resp);
        }
    }
    return (StatusCode::OK, Json(resp)).into_response();
}

async fn model_id_or_insert(
    database: &Database,
    brand: &str,
    model: &str,
    _price: Option<u32>,
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM models WHERE brand = $1 AND model = $2;";
    let m_id: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(&brand)
        .bind(&model)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = m_id {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = "INSERT INTO models (brand, model) VALUES ($1, $2) RETURNING id;";
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

async fn accessory_id_or_insert(
    database: &Database,
    item: &str,
    price: Price,
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM accessories WHERE item = $1;";
    let accessory: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(&item)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = accessory {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = "INSERT INTO accessories (item, price) VALUES ($1, $2) RETURNING id;";
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

async fn fault_id_or_insert(
    database: &Database,
    item: &str,
    cost: Price,
) -> Result<i32> {
    const QUERY: &str = "SELECT id FROM faults WHERE item = $1;";
    let f_id: Option<(i32,)> = sqlx::query_as(QUERY)
        .bind(item)
        .fetch_optional(database)
        .await
        .unwrap();

    if let Some((id,)) = f_id {
        Ok(id)
    } else {
        const INSERT_QUERY: &str = "INSERT INTO faults (item, cost) VALUES ($1, $2) RETURNING id;";
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

async fn status_id_or_insert(
    database: &Database,
    flow: &str,
) -> Result<i32> {
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
async fn query_order(
    database: &Database,
    id: i32,
) -> Option<OrderInfo> {
    const QUERY: &str = "SELECT o.id, o.issue_at, d.name department, o.customer_name, o.customer_phone, o.customer_address, m.brand, m.model, o.purchase_at, o.accessory_other, o.appearance, o.appearance_other, o.service, o.fault_other, o.photo_url, o.remark, o.cost, o.prepaid_free, s.flow status FROM orders o INNER JOIN models m ON m.id = o.model_id INNER JOIN departments d ON d.id = o.department_id INNER JOIN models m ON m.id = o.model_id INNER JOIN status s ON s.id = o.status_id WHERE o.id = $1";

    /* TODO map users/accessories/faults table */

    if let Ok(user) = sqlx::query_as::<_, OrderInfo>(QUERY)
        .bind(id)
        .fetch_optional(database)
        .await
    {
        user
    } else {
        None
    }
}

