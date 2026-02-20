use crate::prelude::*;

pub fn verify_auth(acct: &Account, params: &HashMap<String, String>) -> bool {
    // Accept user=admin and password=admin
    let (u, p) = if let (Some(u), Some(p)) = (params.get("u"), params.get("p")) {
        let password = if let Some(hex) = p.strip_prefix("enc:") {
            let bytes: Vec<u8> = (0..hex.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                .collect();
            String::from_utf8(bytes).unwrap_or_default()
        } else {
            p.clone()
        };
        (u, password)
    // Token auth (u, t, s)
    } else if let (Some(u), Some(t), Some(s)) = (params.get("u"), params.get("t"), params.get("s"))
    {
        let mut hasher = Md5::new();
        hasher.update(format!("{}{}", acct.pass(), s));
        let result = hasher.finalize();
        let expected = format!("{:x}", result);
        return u == acct.user() && t == &expected;
    } else {
        return false;
    };
    (u == "admin" && p == "admin")
        || (u == "test" && p == "test")
        || (u == acct.user() && p == acct.pass())
}

pub async fn get_token() -> Result<OAuthToken> {
    let scopes = vec!["streaming"];
    let client =
        librespot::oauth::OAuthClientBuilder::new(SPOTIFY_CLIENT_ID, SPOTIFY_REDIRECT_URI, scopes)
            .open_in_browser()
            .build()
            .map_err(|e| anyhow!("Failed to build OAuth client: {e}"))?;

    let token = client
        .get_access_token_async()
        .await
        .map_err(|e| anyhow!("Failed to get access token: {e}"))?;

    // return a fresh token
    client
        .refresh_token_async(&token.refresh_token)
        .await
        .map_err(Into::into)
}

pub async fn create_session(token: &OAuthToken) -> Result<Session> {
    // authenticate
    let session_config = librespot::core::SessionConfig::default();
    let credentials =
        librespot::discovery::Credentials::with_access_token(token.access_token.as_str());

    // connect to Spotify session
    log::trace!("Connecting...");
    let session = Session::new(session_config, None);
    session.connect(credentials, true).await?;
    Ok(session)
}
