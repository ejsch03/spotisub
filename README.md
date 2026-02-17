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
- [x] `ping`
- [x] `getLicense`
- [x] `search3`
- [x] `getCoverArt`
- [x] `stream`

The other implemented endpoints are stubs.


## Todo
- [ ] Add/improve documentation.
- [ ] Fix `stream` behavior (a bit buggy).
- [ ] Implement automatic session refresh on expiry.
- [ ] Implement the [seek](https://opensubsonic.netlify.app/docs/extensions/transcodeoffset/) extension.
  - [ ] Add this extension to the `getOpenSubsonicExtensions` response.
- [ ] Implement `getArtists` endpoint.
- [ ] Implement `getPlaylists` endpoint.
- [ ] Fine-tune Opus encoding.
