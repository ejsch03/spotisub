use crate::auth::*;
use crate::prelude::*;

pub struct LibreSpotify {
    cred: Credentials,
    sess: Mutex<Session>,
}

impl LibreSpotify {
    pub async fn session(&self) -> Result<Session> {
        let mut sess = self.sess.lock().await;
        if sess.is_invalid() {
            *sess = Self::create_session().await?;
        }
        Ok(sess.clone())
    }

    async fn create_session() -> Result<Session> {
        let creds = Cache::new(Some("."), None, None, None)?
            .credentials()
            .ok_or(anyhow!("No cached credentials"))?;

        let session_config = SessionConfig::default();
        let session = Session::new(session_config, None);
        session.connect(creds, true).await?;
        Ok(session)
    }
}

pub struct State {
    rspot: RSpotify,                            // spotify dev api
    lspot: LibreSpotify,                        // librespot config
    http: HttpClient,                           // reqwests client
    song_cache: Mutex<HashMap<String, Song>>,   // song metadata cache
    cover_cache: Mutex<HashMap<String, Bytes>>, // cover-art metadata cache
    rate_limits: Mutex<HashMap<IpAddr, RateLimit>>,
}

impl State {
    pub async fn new(cred: Credentials) -> Result<Self> {
        let rspot_cred =
            rspotify::Credentials::new(cred.dev().client_id(), cred.dev().client_secret());
        let rspot = RSpotify::new(rspot_cred);
        rspot.request_token().await?;

        let sess = Mutex::new(create_session().await?);

        let lspot = LibreSpotify { cred, sess };

        let app_state = Self {
            rspot,
            lspot,
            http: Default::default(),
            song_cache: Default::default(),
            cover_cache: Default::default(),
            rate_limits: Default::default(),
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

    pub async fn session(&self) -> Result<Session> {
        self.lspot.session().await
    }

    pub const fn song_cache(&self) -> &Mutex<HashMap<String, Song>> {
        &self.song_cache
    }

    pub const fn cover_cache(&self) -> &Mutex<HashMap<String, Bytes>> {
        &self.cover_cache
    }

    pub const fn rate_limits(&self) -> &Mutex<HashMap<IpAddr, RateLimit>> {
        &self.rate_limits
    }
}
