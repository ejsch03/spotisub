use crate::prelude::*;

pub async fn run(cfg: Config) -> Result<()> {
    // init application state data
    let app_state = actix_web::web::Data::new(State::new(cfg.cred()).await?);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(app_state.clone())
            .service(
                actix_web::web::resource(["/rest/getCoverArt", "/rest/getCoverArt.view"])
                    .route(actix_web::web::get().to(get_cover_art)),
            )
            .service(
                actix_web::web::resource(["/rest/getLicense", "/rest/getLicense.view"])
                    .route(actix_web::web::get().to(get_license)),
            )
            .service(
                actix_web::web::resource([
                    "/rest/getOpenSubsonicExtensions",
                    "/rest/getOpenSubsonicExtensions.view",
                ])
                .route(actix_web::web::get().to(get_open_subsonic_extensions)),
            )
            .service(
                actix_web::web::resource(["/rest/getSong", "/rest/getSong.view"])
                    .route(actix_web::web::get().to(get_song)),
            )
            .service(
                actix_web::web::resource(["/rest/ping", "/rest/ping.view"])
                    .route(actix_web::web::get().to(ping)),
            )
            .service(
                actix_web::web::resource(["/rest/search3", "/rest/search3.view"])
                    .route(actix_web::web::get().to(search3)),
            )
            .service(
                actix_web::web::resource(["/rest/stream", "/rest/stream.view"])
                    .route(actix_web::web::get().to(stream)),
            )
    })
    .bind(cfg.addr())?
    .run()
    .await
    .map_err(Into::into)
}
