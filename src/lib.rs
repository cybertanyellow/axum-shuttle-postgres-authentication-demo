mod authentication;
mod dcare_order;
mod dcare_user;
mod department;
mod errors;
mod gsheets;
mod utils;

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Query},
    //extract::Multipart,
    middleware,
    response::{Html, IntoResponse, Redirect},
    routing::{any, get, post},
    //Json,
    Router,
};
use http::Response;

use errors::NotLoggedIn;
use pbkdf2::password_hash::rand_core::OsRng;
use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use shuttle_service::{error::CustomError, ShuttleAxum};
use shuttle_secrets::SecretStore;
use sqlx::Executor;
use tera::{Context, Tera};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    IntoParams, Modify, OpenApi, ToSchema,
};
use utoipa_swagger_ui::SwaggerUi;
//use tracing::{ info, };

use utils::*;

use authentication::{
    //delete_user, login, signup,
    auth,
    AuthState,
};
use dcare_order::{
    order_create, order_delete, order_history_list_request, order_history_request,
    order_list_request, order_request, order_update,
};
use dcare_user::{
    logout_response_api, me_api, post_delete_api, post_login_api, post_signup_api,
    update_myself_api, update_user_api, user_api, users_api,
};
use department::{
    department_create, department_delete, department_list_request, department_request,
    department_update,
    /*department_org_delete, department_org_list_request, department_org_request,*/
};
use gsheets::SharedDcareGoogleSheet;

type Templates = Arc<Tera>;
type Database = sqlx::PgPool;
type Random = Arc<Mutex<ChaCha8Rng>>;

const USER_COOKIE_NAME: &str = "user_token";
const COOKIE_MAX_AGE: &str = "9999999";

#[derive(Deserialize, IntoParams)]
pub struct Pagination {
    pub offset: i32,
    pub entries: i32,
}
impl Pagination {
    pub fn parse(mine: Option<Query<Self>>) -> (i32, i32) {
        mine.map_or((0, 100), |p| (p.offset, p.entries))
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams, Default)]
pub struct ApiResponse {
    code: u16,
    message: Option<String>,
}

impl ApiResponse {
    pub fn new(code: u16, message: Option<String>) -> Self {
        Self { code, message }
    }

    pub fn update(&mut self, code: u16, message: Option<String>) -> &mut Self {
        self.code = code;
        self.message = message;
        self
    }
}

#[shuttle_service::main]
async fn server(
#[shuttle_secrets::Secrets] secret_store: SecretStore,
#[shuttle_shared_db::Postgres] pool: Database,
) -> ShuttleAxum {
    pool.execute(include_str!("../schema.sql"))
        .await
        .map_err(CustomError::new)?;

    let key = secret_store.get("SERVICE_ACCOUNT_JSON");

    let gsheet = SharedDcareGoogleSheet::new(
        key,
        "19cQ_zAgqkM_iqOiqECP1yVTobuRkFbwk-VfegOys8ZE",
        "工單表",
        ).await
        .ok();

    Ok(sync_wrapper::SyncWrapper::new(get_router(pool, gsheet)))
}

pub fn get_router(database: Database, gsheet: Option<SharedDcareGoogleSheet>) -> Router {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("base.html", include_str!("../templates/base.html")),
        ("index", include_str!("../templates/index.html")),
        ("signup", include_str!("../templates/signup.html")),
        ("login", include_str!("../templates/login.html")),
        ("users", include_str!("../templates/users.html")),
        ("user", include_str!("../templates/user.html")),
    ])
    .unwrap();

    let middleware_database = database.clone();
    let random = ChaCha8Rng::seed_from_u64(OsRng.next_u64());

    #[derive(OpenApi)]
    #[openapi(
        paths(
            dcare_user::post_signup_api,
            dcare_user::post_login_api,
            dcare_user::post_delete_api,
            dcare_user::me_api,
            dcare_user::update_myself_api,
            dcare_user::user_api,
            dcare_user::update_user_api,
            dcare_user::users_api,

            dcare_order::order_request,
            dcare_order::order_list_request,
            dcare_order::order_delete,
            dcare_order::order_update,
            dcare_order::order_create,
            dcare_order::order_history_request,
            dcare_order::order_history_list_request,

            department::department_request,
            department::department_list_request,
            department::department_delete,
            department::department_update,
            department::department_create,

            /*department::department_org_request,
            department::department_org_list_request,
            department::department_org_delete,*/
        ),
        components(
            schemas(
                dcare_user::UserLogin, dcare_user::ResponseUserLogin, dcare_user::UserNew,
                dcare_user::UserInfo, dcare_user::ResponseUser, dcare_user::ResponseUsers,
                dcare_user::UpdateMe, dcare_user::UpdateUser,
                ApiResponse,

                dcare_order::OrdersResponse, dcare_order::OrderResponse,
                dcare_order::OrderInfo, dcare_order::OrderSummary,
                dcare_order::OrderNew, dcare_order::OrderUpdate,
                dcare_order::OrderApiResponse,

                department::DepartmentsResponse, department::DepartmentResponse,
                department::DepartmentInfo, department::DepartmentSummary,
                department::DepartmentNew, department::DepartmentUpdate,

                /*department::DepartmentOrgsResponse, department::DepartmentOrgResponse,
                department::DepartmentOrgData,*/
            )
        ),
        modifiers(&SecurityAddon),
        tags(
            (name = "dcare", description = "dcare service/management API")
            )
     )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "logined cookie/session-id",
                    SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(USER_COOKIE_NAME))),
                )
            }
        }
    }

    let router = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/", get(index))
        .route("/styles.css", any(styles))
        .route("/api/v1/login", post(post_login_api))
        .route("/api/v1/logout", get(logout_response_api))
        .route("/api/v1/me", get(me_api).put(update_myself_api))
        .route(
            "/api/v1/user/:account",
            get(user_api).put(update_user_api).delete(post_delete_api),
        )
        .route("/api/v1/user", get(users_api).post(post_signup_api))
        .route("/api/v1/order/history/:sn", get(order_history_request))
        .route("/api/v1/order/history", get(order_history_list_request))
        .route(
            "/api/v1/order/:sn",
            get(order_request).put(order_update).delete(order_delete),
        )
        .route("/api/v1/order", get(order_list_request).post(order_create))
        .route(
            "/api/v1/department/:shorten",
            get(department_request)
                .put(department_update)
                .delete(department_delete),
        )
        .route(
            "/api/v1/department",
            get(department_list_request).post(department_create),
        )
        /*.route(
            "/api/v1/department/org/:shorten",
            get(department_org_request).delete(department_org_delete),
        )
        .route("/api/v1/department/org", get(department_org_list_request))*/
        .layer(middleware::from_fn(move |req, next| {
            auth(req, next, middleware_database.clone())
        }))
        .layer(Extension(Arc::new(tera)))
        .layer(Extension(database))
        .layer(Extension(Arc::new(Mutex::new(random))));


    if let Some(gsheets) = gsheet {
        router.layer(Extension(gsheets))
    } else {
        router
    }
}

async fn index(
    Extension(current_user): Extension<AuthState>,
    Extension(templates): Extension<Templates>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("logged_in", &current_user.logged_in());
    context.insert("home_screen", &true);
    Html(templates.render("index", &context).unwrap())
}

async fn styles() -> impl IntoResponse {
    Response::builder()
        .status(http::StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_str!("../public/styles.css").to_owned())
        .unwrap()
}

#[allow(dead_code)]
async fn me(
    Extension(mut current_user): Extension<AuthState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if let Some(user) = current_user.get_user().await {
        Ok(Redirect::to(&format!("/user/{}", user.account)))
    } else {
        Err(error_page(&NotLoggedIn))
    }
}
