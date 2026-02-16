#![allow(dead_code)] // temp

use librespot::core::Session;
use librespot::oauth::OAuthToken;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::player::Player;
use reqwest::Client as HttpClient;
use rspotify::ClientCredsSpotify as RSpotify;
use tokio::sync::Notify;

use crate::*;

pub struct LibreSpotify {
    cred: Credentials,
    token: OAuthToken,
    sess: Session,
    sink: StreamingSink,
    player: Arc<Player>,
    cancel: Arc<Notify>,
}

pub struct AppState {
    http: HttpClient,    // reqwests client
    rspot: RSpotify,     // spotify dev api
    lspot: LibreSpotify, // librespot config
}

impl AppState {
    pub async fn new(cred: Credentials) -> Result<Self> {
        let rspot_cred =
            rspotify::Credentials::new(cred.dev().client_id(), cred.dev().client_secret());
        let rspot = RSpotify::new(rspot_cred);
        rspot.request_token().await?;

        let token = get_token().await?;
        let sess = create_session(&token).await?;

        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        // stream sink setup
        let sink = StreamingSink::new(audio_format);

        let player = Player::new(player_config, sess.clone(), Box::new(NoOpVolume), {
            let sink = sink.clone();
            move || Box::new(sink)
        });

        let lspot = LibreSpotify {
            cred,
            token,
            sess,
            sink,
            player,
            cancel: Default::default(),
        };

        let app_state = Self {
            http: Default::default(),
            rspot,
            lspot,
        };
        Ok(app_state)
    }

    pub const fn http(&self) -> &HttpClient {
        &self.http
    }

    pub const fn rspotify(&self) -> &RSpotify {
        &self.rspot
    }

    pub const fn cred(&self) -> &Credentials {
        &self.lspot.cred
    }

    pub const fn token(&self) -> &OAuthToken {
        &self.lspot.token
    }

    pub const fn session(&self) -> &Session {
        &self.lspot.sess
    }

    pub const fn sink(&self) -> &StreamingSink {
        &self.lspot.sink
    }

    pub fn player(&self) -> Arc<Player> {
        self.lspot.player.clone()
    }

    pub fn cancel(&self) -> Arc<Notify> {
        self.lspot.cancel.clone()
    }
}
