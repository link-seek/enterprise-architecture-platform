mod ai;
mod app;
mod api_doc;
mod config;
mod graphql;
mod middleware;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = config::Configuration::load()?;
    let state = state::AppState::new(cfg).await?;

    let graphql_schema = graphql::build_graphql_schema(&state.db).await?;

    let app = app::build_router(state.clone(), graphql_schema);
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
