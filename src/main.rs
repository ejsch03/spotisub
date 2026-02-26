#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let cfg = spotisub::cfg::Config::new()?;
    spotisub::app::run(cfg).await
}
