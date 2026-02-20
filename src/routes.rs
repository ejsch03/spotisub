use crate::prelude::*;

pub async fn get_cover_art(
    data: Data<AppState>,
    query: Query<HashMap<String, String>>,
) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        log::error!("get_cover_art: Unauthorized.");
        return HttpResponse::Unauthorized().finish();
    }

    let id = match query.get("id") {
        Some(id) => id,
        None => return ResponseBody::<()>::failed().into_response(),
    };

    // return cached image to avoid redundant spotify cdn fetches
    let image_bytes = if let Some(bytes) = data.cover_cache().lock().await.get(id) {
        bytes.clone()
    } else {
        let album_id = match AlbumId::from_id(id) {
            Ok(id) => id,
            Err(_) => return HttpResponse::BadRequest().body("Invalid Spotify album ID"),
        };

        let album = match data.rspotify().album(album_id, None).await {
            Ok(t) => t,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

        // spotify returns images sorted largest first
        let image_url = match album.images.first() {
            Some(img) => &img.url,
            None => return HttpResponse::NotFound().finish(),
        };

        let bytes = match data.http().get(image_url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => bytes,
                Err(_) => return HttpResponse::InternalServerError().finish(),
            },
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

        data.cover_cache()
            .lock()
            .await
            .insert(id.clone(), bytes.clone());

        bytes
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "image/jpeg"))
        .insert_header(("Cache-Control", "public, max-age=86400"))
        .body(image_bytes)
}

pub async fn get_license(
    data: Data<AppState>,
    query: Query<HashMap<String, String>>,
) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        log::error!("get_license: Unauthorized.");
        return HttpResponse::Unauthorized().finish();
    }
    ResponseBody::ok_with(serde_json::json!({
        "license": { "valid": true }
    }))
    .into_response()
}

// intentionally unauthenticated — clients call this before login to discover server capabilities
pub async fn get_open_subsonic_extensions() -> impl Responder {
    ResponseBody::ok_with(serde_json::json!({
        "openSubsonicExtensions": [
            {
                "name": "transcodeOffset",
                "versions": [1]
            }
        ]
    }))
    .into_response()
}

pub async fn get_song(
    data: Data<AppState>,
    query: Query<HashMap<String, String>>,
) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        return HttpResponse::Unauthorized().finish();
    }

    let id = match query.get("id") {
        Some(id) => id,
        None => return HttpResponse::BadRequest().finish(),
    };

    // search results are pre-cached by search3, so this is usually a cache hit
    let value = if let Some(song) = data.song_cache().lock().await.get(id) {
        serde_json::json!({ "song": song })
    } else {
        let track_id = match TrackId::from_id(id) {
            Ok(id) => id,
            Err(_) => return HttpResponse::BadRequest().finish(),
        };

        let track = match data.rspotify().track(track_id, None).await {
            Ok(t) => t,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

        let song = match Song::from_spotify(&track) {
            Some(song) => song,
            None => return HttpResponse::NotFound().finish(),
        };

        data.song_cache()
            .lock()
            .await
            .insert(song.id.clone(), song.clone());

        serde_json::json!({ "song": song })
    };

    ResponseBody::ok_with(value).into_response()
}

pub async fn ping(data: Data<AppState>, query: Query<HashMap<String, String>>) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        log::error!("ping: Unauthorized.");
        return HttpResponse::Unauthorized().finish();
    }
    ResponseBody::<()>::ok().into_response()
}

pub async fn search3(
    data: Data<AppState>,
    query: Query<HashMap<String, String>>,
) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        log::error!("search3: Unauthorized.");
        return HttpResponse::Unauthorized().finish();
    }

    let songs = if let Some(search_term) = query.get("query") {
        // cap results at 7 to avoid excessive spotify api usage
        let count = query
            .get("songCount")
            .and_then(|n| n.parse().ok())
            .unwrap_or(4)
            .min(7);

        let results = loop {
            match data
                .rspotify()
                .search(
                    search_term,
                    SearchType::Track,
                    None,
                    None,
                    Some(count),
                    None,
                )
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
            // collect first, then acquire lock to avoid holding it across the iteration
            let songs: Vec<Song> = tracks.items.iter().filter_map(Song::from_spotify).collect();
            let mut cache = data.song_cache().lock().await;
            for song in &songs {
                cache.insert(song.id.clone(), song.clone());
            }
            songs
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

pub async fn stream(data: Data<AppState>, query: Query<HashMap<String, String>>) -> impl Responder {
    if !verify_auth(data.cred().account(), &query) {
        log::error!("stream: Unauthorized.");
        return HttpResponse::Unauthorized().finish();
    }

    let id = match query.get("id") {
        Some(id) => id,
        None => {
            log::error!("stream: Missing 'id'.");
            return HttpResponse::BadRequest().finish();
        }
    };

    let uri = match SpotifyId::from_base62(id) {
        Ok(id) => SpotifyUri::Track { id },
        Err(..) => {
            log::error!("stream: Invalid 'id'.");
            return HttpResponse::NotFound().finish();
        }
    };

    // transcodeOffset (opensubsonic) takes precedence over timeOffset (standard subsonic)
    let time_offset_ms = query
        .get("transcodeOffset")
        .or_else(|| query.get("timeOffset"))
        .and_then(|t| t.parse::<u32>().ok())
        .map(|secs| secs * 1000)
        .unwrap_or(0);

    let (tx, rx) = unbounded_channel();

    // create a fresh player and sink per request to avoid shared state races between streams
    let sink = StreamingSink::new(Default::default(), tx);
    let player = Player::new(Default::default(), data.session(), Box::new(NoOpVolume), {
        let sink = sink.clone();
        move || Box::new(sink)
    });

    log::info!("Streaming {} (offset {}ms)...", uri, time_offset_ms);
    player.load(uri, true, time_offset_ms);

    let stream = async_stream::stream! {
        // keep player alive for the duration of the stream — dropping it closes tx and ends rx
        let _player = player;
        let mut rx = rx;
        let mut pipeline = AudioPipeline::new();

        // opus identification and comment headers must precede any audio packets
        yield Ok::<_, actix_web::Error>(Bytes::from(pipeline.encoder.header_bytes()));

        while let Some(pcm_bytes) = rx.recv().await {
            let samples: &[i16] = bytemuck::cast_slice(&pcm_bytes);
            let ogg_bytes = pipeline.process(samples);
            if !ogg_bytes.is_empty() {
                yield Ok(Bytes::from(ogg_bytes));
            }
        }

        // flush any samples that didn't fill a complete opus frame
        let remaining = pipeline.flush();
        if !remaining.is_empty() {
            yield Ok(Bytes::from(remaining));
        }
    };

    HttpResponse::Ok()
        .content_type("audio/ogg; codecs=opus")
        .streaming(stream)
}
