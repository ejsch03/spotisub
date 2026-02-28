pub use std::collections::HashMap;
pub use std::net::{IpAddr, Ipv4Addr, SocketAddr};
pub use std::time::{Duration, Instant};

// error-handling
pub use anyhow::{Result, anyhow};

// actix
pub use actix_web::web::{Data, Query};
pub use actix_web::{HttpRequest, HttpResponse, Responder};
pub use bytes::Bytes;

// librespot
pub use librespot::core::{Session, SessionConfig, SpotifyId, SpotifyUri, cache::Cache};
pub use librespot::discovery::Credentials as LSpotCreds;
pub use librespot::playback::{
    audio_backend::{Sink, SinkResult},
    config::AudioFormat,
    convert::Converter,
    decoder::AudioPacket,
    mixer::NoOpVolume,
    player::Player,
};

// rspotify
pub use rspotify::ClientCredsSpotify as RSpotify;
pub use rspotify::model::{AlbumId, FullTrack, Id, SearchResult, SearchType, TrackId};
pub use rspotify::prelude::BaseClient;

// misc
pub use md5::{Digest, Md5};
pub use reqwest::Client as HttpClient;
pub use serde::Serialize;
pub use tokio::sync::{
    Mutex,
    mpsc::{UnboundedSender, unbounded_channel},
};
pub use tokio::time::sleep;
pub use zerocopy::IntoBytes;

// local
pub use crate::auth::*;
pub use crate::cfg::*;
pub use crate::consts::*;
pub use crate::json::*;
pub use crate::opus::*;
pub use crate::rate_limit::*;
pub use crate::routes::*;
pub use crate::sink::*;
pub use crate::state::*;
