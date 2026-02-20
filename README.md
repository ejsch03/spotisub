# spotisub

Self-hosted OpenSubsonic backend powered by librespot with Opus encoding.


## Config
Create the `$HOME/spotisub.json` file. Every entry is required.
```json
{
    "user": "...",
    "pass": "...",

    "client_id": "...",
    "client_secret": "..."
}
```
- `user/pass`: OpenSubsonic credentials.
- `client_id/secret`: Spotify developer app credentials.


## Implemented OpenSubsonic Endpoints
- [x] `getCoverArt`
- [x] `getLicense`
- [x] `getOpenSubsonicExtensions`
- [x] `getSong`
- [x] `ping`
- [x] `search3`
- [x] `stream`

The other implemented endpoints are stubs.


## Todo
- [ ] Add/improve documentation.
- [ ] Implement automatic session refresh on expiry.
- [ ] Implement `getArtists` endpoint.
- [ ] Implement `getPlaylists` endpoint.
