use std::collections::HashMap;

use actix_web::{
    HttpResponse, Responder, get,
    web::{Data, Query},
};
use rspotify::{
    model::{AlbumId, SearchResult, SearchType},
    prelude::BaseClient,
};
use tokio::{sync::mpsc::unbounded_channel, time::sleep};

use crate::*;

#[get("/rest/ping.view")]
async fn ping(data: Data<AppState>, query: Query<HashMap<String, String>>) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        return HttpResponse::Unauthorized().finish();
    }
    ResponseBody::<()>::ok().into_response()
}

#[get("/rest/getLicense.view")]
async fn get_license(
    data: Data<AppState>,
    query: Query<HashMap<String, String>>,
) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        return HttpResponse::Unauthorized().finish();
    }
    ResponseBody::ok_with(serde_json::json!({
        "license": { "valid": true }
    }))
    .into_response()
}

#[get("/rest/getOpenSubsonicExtensions.view")]
async fn get_open_subsonic_extensions() -> impl Responder {
    ResponseBody::ok_with(serde_json::json!({
        "openSubsonicExtensions": []
    }))
    .into_response()
}

#[get("/rest/getMusicFolders.view")]
async fn get_music_folders() -> impl Responder {
    ResponseBody::ok_with(serde_json::json!({
        "musicFolders": {}
    }))
    .into_response()
}

#[get("/rest/getIndexes.view")]
async fn get_indexes() -> impl Responder {
    ResponseBody::ok_with(serde_json::json!({
        "indexes": {}
    }))
    .into_response()
}

#[get("/rest/getArtists.view")]
async fn get_artists() -> impl Responder {
    ResponseBody::ok_with(serde_json::json!({
        "artists": {}
    }))
    .into_response()
}

#[get("/rest/getPlaylists.view")]
async fn get_playlists() -> impl Responder {
    ResponseBody::ok_with(serde_json::json!({
        "playlists": {}
    }))
    .into_response()
}

#[get("/rest/search3.view")]
async fn search3(data: Data<AppState>, query: Query<HashMap<String, String>>) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        return HttpResponse::Unauthorized().finish();
    }
    let songs = if let Some(search_term) = query.get("query") {
        let results = loop {
            match data
                .rspotify()
                .search(search_term, SearchType::Track, None, None, Some(4), None)
                .await
            {
                Ok(r) => break r,
                Err(e) => {
                    log::error!("Rate limited, retrying in {:?}: {}", DELAY_SEARCH3, e);
                    sleep(DELAY_SEARCH3).await;
                }
            }
        };

        if let SearchResult::Tracks(tracks) = results {
            spotify_to_subsonic(tracks.items.as_slice())
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    ResponseBody::ok_with(serde_json::json!({
        "searchResult3": {
            "song": songs
        }
    }))
    .into_response()
}

#[get("/rest/getCoverArt.view")]
pub async fn get_cover_art(
    data: Data<AppState>,
    query: Query<HashMap<String, String>>,
) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        return HttpResponse::Unauthorized().finish();
    }
    // parse album id
    let id = match query.get("id") {
        Some(id) => id,
        None => return ResponseBody::<()>::failed().into_response(),
    };
    // Parse Spotify Track ID
    let album_id = match AlbumId::from_id(id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid Spotify album ID"),
    };
    // Fetch track metadata
    let album = match data.rspotify().album(album_id, None).await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    // Get largest album image
    let image_url = match album.images.first() {
        Some(img) => &img.url,
        None => return HttpResponse::NotFound().finish(),
    };
    // Download image from Spotify CDN
    let image_bytes = match data.http().get(image_url).send().await {
        Ok(resp) => match resp.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        },
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    HttpResponse::Ok()
        .insert_header(("Content-Type", "image/jpeg"))
        .insert_header(("Cache-Control", "public, max-age=86400"))
        .body(image_bytes)
}

// TODO - improvements.
#[get("/rest/stream.view")]
async fn stream(data: Data<AppState>, query: Query<HashMap<String, String>>) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        return HttpResponse::Unauthorized().finish();
    }
    // allow any ongoining streams to finish
    data.cancel().notify_waiters();

    // obtain id, if provided
    let id = match query.get("id") {
        Some(id) => id,
        None => return HttpResponse::NotFound().finish(),
    };

    // parse spotify track id
    let uri = match get_uri(id) {
        Ok(uri) => uri,
        Err(..) => return HttpResponse::NotFound().finish(),
    };

    // create new sender/receiver
    let (tx, rx) = unbounded_channel();
    data.sink().set_sender(tx); // update sender

    let player = data.player().clone();
    let streamed = data.cancel().clone();
    tokio::spawn(async move {
        let s = uri.to_string();
        player.load(uri, true, 0);
        log::info!("Streaming {}...", s);

        // todo: instead, wait for interupting api event
        tokio::select! {
            _ = player.await_end_of_track() => log::info!("Finished track!"),
            _ = streamed.notified() => log::info!("Stopped early! Another stream in progress.")
        }
    });

    let stream = async_stream::stream! {
        let mut pipeline = AudioPipeline::new();

        yield Ok::<_, actix_web::Error>(bytes::Bytes::from(pipeline.encoder.header_bytes()));

        let mut rx = rx;
        while let Some(pcm_bytes) = rx.recv().await {
            let samples: &[i16] = bytemuck::cast_slice(&pcm_bytes);
            let ogg_bytes = pipeline.process(samples);
            if !ogg_bytes.is_empty() {
                yield Ok(bytes::Bytes::from(ogg_bytes));
            }
        }
        let remaining = pipeline.flush();
        if !remaining.is_empty() {
            yield Ok(bytes::Bytes::from(remaining));
        }
    };
    HttpResponse::Ok()
        .content_type("audio/ogg; codecs=opus")
        .streaming(stream)
}
