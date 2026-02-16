use librespot::core::{SpotifyId, SpotifyUri};
use rspotify::model::{AlbumId, FullTrack, TrackId};

use crate::*;

pub fn get_uri(track: &str) -> Result<SpotifyUri> {
    let id = SpotifyId::from_base62(track)?;
    let uri = SpotifyUri::Track { id };
    Ok(uri)
}

pub fn spotify_to_subsonic(tracks: &[FullTrack]) -> Vec<serde_json::Value> {
    let songs: Vec<serde_json::Value> = tracks
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.as_ref().unwrap_or(&TrackId::from_id("unknown").unwrap()),
                "title": t.name,
                "artist": t.artists.first().map(|a| &a.name).unwrap_or(&"Unknown".to_string()),
                "album": t.album.name,
                "coverArt": t.album.id.as_ref().unwrap_or(&AlbumId::from_id("unknown").unwrap()),
                "track": t.track_number,
                "duration": t.duration.to_std().unwrap().as_secs(),
                "isDir": false,
                "created": t.album.release_date.as_ref().unwrap_or(&"1970-01-01".to_string()),
                "contentType": "audio/ogg; codecs=opus"
            })
        })
        .collect();
    songs
}
