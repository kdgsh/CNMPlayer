use anyhow::{Context, Result, anyhow};
use cyper::Client;
use ncm_api::{ApiClient, ApiResponse};

#[derive(Debug, Clone)]
pub struct ResolvedUrl {
    pub url: String,
}

pub trait MusicSource {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn resolve_song_url(&mut self, song_id: &str, quality: &str) -> Result<ResolvedUrl>;
    async fn fetch_lyric(&mut self, song_id: &str) -> Result<Option<String>>;
    fn http_client(&self) -> &Client;
    fn session_cookie(&self) -> Option<&str>;
}

// ---------------------------------------------------------------------------
// NeteaseSource
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct NeteaseSource {
    client: ApiClient,
    cookie: Option<String>,
    http: Client,
}

impl NeteaseSource {
    pub fn new(cookie: Option<String>, http: Client) -> Result<Self> {
        let client = ApiClient::new(cookie.clone(), http.clone());
        Ok(Self {
            client,
            cookie,
            http,
        })
    }

    pub fn set_cookie(&mut self, cookie: String) {
        self.cookie = Some(cookie.clone());
        self.client.set_cookie(cookie);
    }

    pub fn clear_cookie(&mut self) {
        self.cookie = None;
        self.client.set_cookie(String::new());
    }

    pub fn api_client_mut(&mut self) -> &mut ApiClient {
        &mut self.client
    }

    fn query_with_cookie(&self) -> ncm_api::Query {
        if let Some(ref cookie) = self.cookie {
            if !cookie.is_empty() {
                return ncm_api::Query::new().cookie(cookie.as_str());
            }
        }
        ncm_api::Query::new()
    }
}

impl MusicSource for NeteaseSource {
    fn id(&self) -> &str {
        "netease"
    }

    fn name(&self) -> &str {
        "Netease Cloud Music"
    }

    async fn resolve_song_url(&mut self, song_id: &str, quality: &str) -> Result<ResolvedUrl> {
        let response = self
            .client
            .song_url_v1(
                &self
                    .query_with_cookie()
                    .param("id", song_id)
                    .param("level", quality),
            )
            .await?;

        if let Some(url) = extract_url(&response, "/data/0/url") {
            return Ok(ResolvedUrl { url });
        }

        let fallback = self
            .client
            .song_url(
                &self
                    .query_with_cookie()
                    .param("id", song_id)
                    .param("br", "320000"),
            )
            .await?;

        if let Some(url) = extract_url(&fallback, "/data/0/url") {
            return Ok(ResolvedUrl { url });
        }

        Err(anyhow!("netease: url not found for {} at {}", song_id, quality))
    }

    async fn fetch_lyric(&mut self, song_id: &str) -> Result<Option<String>> {
        let response = self
            .client
            .lyric(&self.query_with_cookie().param("id", song_id))
            .await?;
        Ok(response
            .body
            .pointer("/lrc/lyric")
            .and_then(|v| v.as_str())
            .map(str::to_string))
    }

    fn http_client(&self) -> &Client {
        &self.http
    }

    fn session_cookie(&self) -> Option<&str> {
        self.cookie.as_deref()
    }
}

// ---------------------------------------------------------------------------
// CustomApiSource: LX Music compatible HTTP API
//   GET {api_url}/url?source=wy&songId={id}&quality={q}
//   GET {api_url}/lyric?source=wy&songId={id}
//   Headers: X-API-Key / X-Request-Key: {key}
//   Response: { "code": 200, "url": "https://..." }
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CustomApiSource {
    api_url: String,
    api_key: String,
    http: Client,
}

impl CustomApiSource {
    pub fn new(api_url: String, api_key: String, http: Client) -> Self {
        let api_url = api_url.trim_end_matches('/').to_string();
        Self {
            api_url,
            api_key,
            http,
        }
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let mut req = self.http.get(url)?;
        if !self.api_key.is_empty() {
            req = req
                .header("X-API-Key", &self.api_key)?
                .header("X-Request-Key", &self.api_key)?;
        }
        let resp = req.send().await.context("custom source request")?;
        let resp = crate::app::api::error_for_status(resp).context("custom source HTTP error")?;
        let bytes = resp.bytes().await.context("custom source response body")?;
        serde_json::from_slice(&bytes).context("parse custom source JSON")
    }
}

impl MusicSource for CustomApiSource {
    fn id(&self) -> &str {
        "custom"
    }

    fn name(&self) -> &str {
        "Custom Source"
    }

    async fn resolve_song_url(&mut self, song_id: &str, quality: &str) -> Result<ResolvedUrl> {
        let level = map_quality(quality);
        let url = format!(
            "{}/url?source=wy&songId={}&quality={}",
            self.api_url, song_id, level
        );
        let parsed = self.get_json(&url).await?;

        if let Some(url) = parsed["url"].as_str().filter(|s| !s.is_empty()) {
            return Ok(ResolvedUrl { url: url.to_string() });
        }
        if let Some(url) = parsed["data"]["url"].as_str().filter(|s| !s.is_empty()) {
            return Ok(ResolvedUrl { url: url.to_string() });
        }

        Err(anyhow!(
            "custom source: no url for {} at {} (code={})",
            song_id,
            quality,
            parsed["code"].as_i64().unwrap_or(-1)
        ))
    }

    async fn fetch_lyric(&mut self, song_id: &str) -> Result<Option<String>> {
        let url = format!("{}/lyric?source=wy&songId={}", self.api_url, song_id);
        let parsed = self.get_json(&url).await?;
        if let Some(lrc) = parsed["lyric"].as_str().filter(|s| !s.is_empty()) {
            return Ok(Some(lrc.to_string()));
        }
        Ok(parsed["data"]["lyric"].as_str().filter(|s| !s.is_empty()).map(str::to_string))
    }

    fn http_client(&self) -> &Client {
        &self.http
    }

    fn session_cookie(&self) -> Option<&str> {
        None
    }
}

// ---------------------------------------------------------------------------
// SourceManager
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum SourceManager {
    Netease(NeteaseSource),
    Custom(NeteaseSource, CustomApiSource),
}

impl SourceManager {
    pub fn netease(cookie: Option<String>, http: Client) -> Result<Self> {
        Ok(SourceManager::Netease(NeteaseSource::new(cookie, http)?))
    }

    pub fn custom(cookie: Option<String>, api_url: String, api_key: String, http: Client) -> Result<Self> {
        let netease = NeteaseSource::new(cookie, http.clone())?;
        let custom = CustomApiSource::new(api_url, api_key, http);
        Ok(SourceManager::Custom(netease, custom))
    }

    pub fn set_cookie(&mut self, cookie: String) {
        match self {
            SourceManager::Netease(n) => n.set_cookie(cookie),
            SourceManager::Custom(n, _) => n.set_cookie(cookie),
        }
    }

    pub fn clear_cookie(&mut self) {
        match self {
            SourceManager::Netease(n) => n.clear_cookie(),
            SourceManager::Custom(n, _) => n.clear_cookie(),
        }
    }

    pub fn is_using_custom(&self) -> bool {
        matches!(self, SourceManager::Custom(_, _))
    }

    pub async fn resolve_song_url(&mut self, song_id: &str, quality: &str) -> Result<ResolvedUrl> {
        match self {
            SourceManager::Netease(n) => n.resolve_song_url(song_id, quality).await,
            SourceManager::Custom(n, c) => {
                match c.resolve_song_url(song_id, quality).await {
                    Ok(url) => return Ok(url),
                    Err(e) => log::warn!("custom source failed, falling back to netease: {}", e),
                }
                n.resolve_song_url(song_id, quality).await
            }
        }
    }

    pub async fn fetch_lyric(&mut self, song_id: &str) -> Result<Option<String>> {
        match self {
            SourceManager::Netease(n) => n.fetch_lyric(song_id).await,
            SourceManager::Custom(n, c) => {
                match c.fetch_lyric(song_id).await {
                    Ok(Some(lrc)) if !lrc.is_empty() => return Ok(Some(lrc)),
                    _ => {}
                }
                n.fetch_lyric(song_id).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_url(response: &ApiResponse, pointer: &str) -> Option<String> {
    response
        .body
        .pointer(pointer)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn map_quality(level: &str) -> &str {
    match level {
        "standard" => "128k",
        "higher" => "192k",
        "exhigh" => "320k",
        "lossless" => "flac",
        "hires" => "flac24bit",
        "jyeffect" | "sky" | "dolby" | "jymaster" => "flac",
        _ => "320k",
    }
}
