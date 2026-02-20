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
- [x] `get_user`
- [x] `ping`
- [x] `search3`
- [x] `stream`

The other implemented endpoints are stubs.


## Todo
- [ ] Add/improve documentation.
- [ ] Implement automatic session refresh on expiry.
- [ ] Implement the [seek](https://opensubsonic.netlify.app/docs/extensions/transcodeoffset/) extension.
  - [ ] Add this extension to the `getOpenSubsonicExtensions` response.
- [ ] Implement `getArtists` endpoint.
- [ ] Implement `getPlaylists` endpoint.
