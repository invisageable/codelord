// could be possible that we need to kill the server when we modified this
// crate: lsof -i :1337 -t | xargs kill

use codelord_server::routes;
use codelord_server::state::ServerState;

use axum::Router;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
  dotenv::dotenv().ok();
  tracing_subscriber::fmt::init();

  let state = Arc::new(ServerState::new());

  // Spawn background voice worker.
  tokio::spawn(codelord_server::workers::voice::run(state.clone()));

  let app = Router::new()
    .route("/health", axum::routing::get(routes::health::check))
    .nest("/rpc", routes::rpc::router(state.clone()))
    .nest("/voice", routes::voice::router(state.clone()))
    .nest("/playground", routes::playground::router(state.clone()))
    .nest("/preview", routes::preview::router(state.clone()));

  let listener = tokio::net::TcpListener::bind("127.0.0.1:1337").await?;

  tracing::info!("[codelord-server] Listening on http://127.0.0.1:1337");
  axum::serve(listener, app).await?;

  Ok(())
}
