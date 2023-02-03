use std::{net::SocketAddr/*, time::Duration*/};
use anyhow::Result;
use dcare_rest_service::get_router;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::postgres::{/*PgPool, */PgPoolOptions};
use lambda_web::{is_running_on_lambda, run_hyper_on_lambda, LambdaError};

#[tokio::main]
pub async fn main() -> Result<(), LambdaError> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "dcare-rest-service=debug".into()),
            )
        .with(tracing_subscriber::fmt::layer())
        .init();


    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost".to_string());

    // setup connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        //.connect_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let app = get_router(pool);

    if is_running_on_lambda() {
        run_hyper_on_lambda(app).await?;
    } else {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        tracing::debug!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
    }

    Ok(())
}
