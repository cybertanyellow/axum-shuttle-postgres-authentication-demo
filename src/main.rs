use std::{net::SocketAddr/*, time::Duration*/};
use clap::Parser;
use anyhow::Result;
use dcare_rest_service::get_router;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sqlx::postgres::{/*PgPool, */PgPoolOptions};

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "example_tokio_postgres=debug".into()),
            )
        .with(tracing_subscriber::fmt::layer())
        .init();


    let cli = Cli::parse();
    //let _port = cli.port.unwrap_or(8000);

    let db_connection_str = cli.url
        .unwrap_or("postgres://postgres:password@localhost".to_string());

    // setup connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        //.connect_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    let app = get_router(pool);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[derive(Parser, Debug)]
#[clap(name = "dcare-rest-service", version, author, about = "A Dcare Rest Service without DB")]
struct Cli {
    #[clap(long)]
    url: Option<String>,
    #[clap(long)]
    port: Option<u16>,
}
