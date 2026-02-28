mod auth;
mod consts;
mod json;
mod opus;
mod prelude;
mod rate_limit;
mod routes;
mod sink;
mod state;

pub mod app;
pub mod cfg;

pub async fn create_auth() -> anyhow::Result<()> {
    auth::create_session().await.map(|_| ())
}
