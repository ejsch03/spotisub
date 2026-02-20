mod app;
mod auth;
mod cfg;
mod consts;
mod json;
mod opus;
mod prelude;
mod routes;
mod sink;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let cfg = cfg::Config::new()?;
    app::run(cfg).await
}
