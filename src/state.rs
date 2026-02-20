#![allow(dead_code)] // temp

use crate::prelude::*;

pub struct LibreSpotify {
    cred: Credentials,
    token: OAuthToken,
    sess: Session,
}

pub struct AppState {
    rspot: RSpotify,                            // spotify dev api
    lspot: LibreSpotify,                        // librespot config
    http: HttpClient,                           // reqwests client
    song_cache: Mutex<HashMap<String, Song>>,   // song metadata cache
    cover_cache: Mutex<HashMap<String, Bytes>>, // cover-art metadata cache
}

impl AppState {
    pub async fn new(cred: Credentials) -> Result<Self> {
        let rspot_cred =
            rspotify::Credentials::new(cred.dev().client_id(), cred.dev().client_secret());
        let rspot = RSpotify::new(rspot_cred);
        rspot.request_token().await?;

        let token = get_token().await?;
        let sess = create_session(&token).await?;

        let lspot = LibreSpotify { cred, token, sess };

        let app_state = Self {
            rspot,
            lspot,
            http: Default::default(),
            song_cache: Default::default(),
            cover_cache: Default::default(),
        };
        Ok(app_state)
    }

    pub const fn rspotify(&self) -> &RSpotify {
        &self.rspot
    }

    pub const fn http(&self) -> &HttpClient {
        &self.http
    }

    pub const fn cred(&self) -> &Credentials {
        &self.lspot.cred
    }

    pub const fn token(&self) -> &OAuthToken {
        &self.lspot.token
    }

    pub fn session(&self) -> Session {
        self.lspot.sess.clone()
    }

    pub const fn song_cache(&self) -> &Mutex<HashMap<String, Song>> {
        &self.song_cache
    }

    pub const fn cover_cache(&self) -> &Mutex<HashMap<String, Bytes>> {
        &self.cover_cache
    }
}
