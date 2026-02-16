mod auth;
mod cfg;
mod consts;
mod json;
mod opus;
mod prelude;
mod routes;
mod sink;
mod state;
mod util;

use auth::*;
use cfg::*;
use consts::*;
use json::*;
use opus::*;
use prelude::*;
use routes::*;
use sink::*;
use state::*;
use util::*;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // init configurations
    let cfg = Config::new()?;

    // init application state data
    let app_state = actix_web::web::Data::new(AppState::new(cfg.cred()).await?);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(app_state.clone())
            .service(ping)
            .service(get_license)
            .service(get_open_subsonic_extensions)
            .service(get_music_folders)
            .service(get_indexes)
            .service(get_artists)
            .service(get_playlists)
            .service(search3)
            .service(stream)
            .service(get_cover_art)
    })
    .bind(cfg.addr())?
    .run()
    .await?;

    Ok(())
}
