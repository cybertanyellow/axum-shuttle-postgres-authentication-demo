mod authentication;
mod dcare_user;
mod errors;
mod utils;
mod dcare_order;

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Extension, Path},
    //extract::Multipart,
    middleware,
    response::{Html, IntoResponse, Redirect},
    routing::{any, get, post},
    Json,
    Router,
};
use http::Response;

use authentication::{auth, delete_user, login, signup, AuthState};
use dcare_user::{
    logout_response_api, me_api, post_delete_api, post_login_api, post_signup_api,
    update_myself_api, update_user_api, user_api, users_api,
};
use errors::{/*LoginError, */ NoUser, NotLoggedIn, SignupError};
use pbkdf2::password_hash::rand_core::OsRng;
use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use shuttle_service::{error::CustomError, ShuttleAxum};
use sqlx::Executor;
use tera::{Context, Tera};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

use utils::*;

use dcare_order::{
    order_create, order_list_request,
    order_request, order_update, order_delete,
};

type Templates = Arc<Tera>;
type Database = sqlx::PgPool;
type Random = Arc<Mutex<ChaCha8Rng>>;

const USER_COOKIE_NAME: &str = "user_token";
const COOKIE_MAX_AGE: &str = "9999999";

#[shuttle_service::main]
async fn server(#[shuttle_shared_db::Postgres] pool: Database) -> ShuttleAxum {
    pool.execute(include_str!("../schema.sql"))
        .await
        .map_err(CustomError::new)?;

    Ok(sync_wrapper::SyncWrapper::new(get_router(pool)))
}

pub fn get_router(database: Database) -> Router {
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
        ),
        components(
            schemas(
                dcare_user::UserLogin, dcare_user::ResponseUserLogin, dcare_user::UserNew,
                dcare_user::UserInfo, dcare_user::ResponseUser, dcare_user::ResponseUsers,
                dcare_user::UpdateMe, dcare_user::UpdateUser,
                dcare_user::ApiResponse,
            )
        ),
        modifiers(&SecurityAddon),
        tags(
            (name = "user", description = "User management API")
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

    Router::new()
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
        .route(
            "/api/v1/order/:id",
            get(order_request).put(order_update).delete(order_delete),
        )
        .route("/api/v1/order", get(order_list_request).post(order_create))
        .layer(middleware::from_fn(move |req, next| {
            auth(req, next, middleware_database.clone())
        }))
        .layer(Extension(Arc::new(tera)))
        .layer(Extension(database))
        .layer(Extension(Arc::new(Mutex::new(random))))
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
