mod authentication;
mod dcare_user;
mod errors;
mod utils;

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
        /*.route("/signup", get(get_signup).post(post_signup))
        .route("/login", get(get_login).post(post_login))
        .route("/logout", post(logout_response))
        .route("/delete", post(post_delete))
        .route("/me", get(me))
        .route("/user/:username", get(user))
        .route("/users", get(users))*/
        .route("/styles.css", any(styles))
        //.route("/api/v1/signup", post(post_signup_api))
        .route("/api/v1/login", post(post_login_api))
        .route("/api/v1/logout", post(logout_response_api))
        //.route("/api/v1/delete/:account", post(post_delete_api))
        .route("/api/v1/me", get(me_api).put(update_myself_api))
        .route(
            "/api/v1/user/:account",
            get(user_api).put(update_user_api).delete(post_delete_api),
        )
        .route("/api/v1/user", get(users_api).post(post_signup_api))
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

#[allow(dead_code)]
async fn user(
    Path(username): Path<String>,
    Extension(mut auth_state): Extension<AuthState>,
    Extension(database): Extension<Database>,
    Extension(templates): Extension<Templates>,
) -> impl IntoResponse {
    const QUERY: &str = "SELECT username FROM users WHERE username = $1;";

    let user: Option<(String,)> = sqlx::query_as(QUERY)
        .bind(&username)
        .fetch_optional(&database)
        .await
        .unwrap();

    if let Some((username,)) = user {
        let user_is_self = auth_state
            .get_user()
            .await
            .map(|logged_in_user| logged_in_user.account == username)
            .unwrap_or_default();
        let mut context = Context::new();
        context.insert("username", &username);
        context.insert("is_self", &user_is_self);
        Ok(Html(templates.render("user", &context).unwrap()))
    } else {
        Err(error_page(&NoUser(username)))
    }
}

#[allow(dead_code)]
async fn get_signup(Extension(templates): Extension<Templates>) -> impl IntoResponse {
    Html(templates.render("signup", &Context::new()).unwrap())
}

#[allow(dead_code)]
async fn get_login(Extension(templates): Extension<Templates>) -> impl IntoResponse {
    Html(templates.render("login", &Context::new()).unwrap())
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct CreateUser {
    username: String,
    password: String,
    confirm_password: String,
}

#[allow(dead_code)]
async fn post_signup(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    //multipart: Multipart,
    Json(user): Json<CreateUser>,
) -> impl IntoResponse {
    /*let data = parse_multipart(multipart)
        .await
        .map_err(|err| error_page(&err))?;

    if let (Some(username), Some(password), Some(confirm_password)) = (
        data.get("username"),
        data.get("password"),
        data.get("confirm_password"),
    ) {
        if password != confirm_password {
            return Err(error_page(&SignupError::PasswordsDoNotMatch));
        }

        match signup(&database, random, username, password).await {
            Ok(session_token) => Ok(login_response(session_token)),
            Err(error) => Err(error_page(&error)),
        }
    } else {
        Err(error_page(&SignupError::MissingDetails))
    }*/
    if user.password != user.confirm_password {
        return Err(error_page(&SignupError::PasswordsDoNotMatch));
    }

    match signup(&database, random, &user.username, &user.password).await {
        Ok(session_token) => Ok(login_response(session_token)),
        Err(error) => Err(error_page(&error)),
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct User {
    username: String,
    password: String,
}

#[allow(dead_code)]
async fn post_login(
    Extension(database): Extension<Database>,
    Extension(random): Extension<Random>,
    Json(user): Json<User>,
) -> impl IntoResponse {
    /*let data = parse_multipart(multipart)
        .await
        .map_err(|err| error_page(&err))?;

    if let (Some(username), Some(password)) = (data.get("username"), data.get("password")) {
        match login(&database, random, username, password).await {
            Ok(session_token) => Ok(login_response(session_token)),
            Err(err) => Err(error_page(&err)),
        }
    } else {
        Err(error_page(&LoginError::MissingDetails))
    }*/
    match login(&database, random, &user.username, &user.password).await {
        Ok(session_token) => Ok(login_response(session_token)),
        Err(err) => Err(error_page(&err)),
    }
}

#[allow(dead_code)]
async fn post_delete(Extension(current_user): Extension<AuthState>) -> impl IntoResponse {
    if !current_user.logged_in() {
        return Err(error_page(&NotLoggedIn));
    }

    delete_user(current_user).await;

    Ok(logout_response().await)
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

#[allow(dead_code)]
async fn users(
    Extension(database): Extension<Database>,
    Extension(templates): Extension<Templates>,
) -> impl IntoResponse {
    const QUERY: &str = "SELECT username FROM users LIMIT 100;";

    let users: Vec<(String,)> = sqlx::query_as(QUERY).fetch_all(&database).await.unwrap();

    // This should be a no op right :)
    let users = users.into_iter().map(|(value,)| value).collect::<Vec<_>>();

    let mut context = Context::new();
    context.insert("users", &users);

    Html(templates.render("users", &context).unwrap())
}
