use anyhow::{Context, Result, anyhow};
use ncm_api::{ApiClient, ApiResponse, Query};
use reqwest::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tokio::runtime::{Builder, Runtime};

const MAX_COVER_IMAGE_BYTES: usize = 5 * 1024 * 1024;

pub struct ApiState {
    runtime: Runtime,
    client: ApiClient,
    cookie: Option<String>,
    http: Client,
}

impl ApiState {
    pub fn new(cookie: Option<String>, http: Client) -> Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        let client = ApiClient::new(cookie.clone(), http.clone());

        Ok(Self {
            runtime,
            client,
            cookie,
            http,
        })
    }

    pub fn session_cookie(&self) -> Option<&str> {
        self.cookie.as_deref()
    }

    pub fn set_cookie(&mut self, cookie: String) {
        self.cookie = Some(cookie.clone());
        self.client.set_cookie(cookie);
    }

    pub fn clear_cookie(&mut self) {
        self.cookie = None;
        self.client.set_cookie(String::new());
    }

    pub fn validate_cookie(&mut self, cookie: &str) -> Result<bool> {
        let query = Query::new().cookie(cookie);
        let response = self.runtime.block_on(self.client.login_status(&query))?;
        let code = response
            .body
            .get("code")
            .and_then(|value| value.as_i64())
            .unwrap_or(response.status);

        if code == 200 {
            self.set_cookie(cookie.to_string());
            self.capture_cookie(&response);
            return Ok(true);
        }

        Ok(false)
    }

    pub fn login_status(&mut self) -> Result<ApiResponse> {
        let query = self.query_with_cookie();
        let response = self.runtime.block_on(self.client.login_status(&query))?;
        self.capture_cookie(&response);
        Ok(response)
    }

    pub fn user_account(&mut self) -> Result<ApiResponse> {
        let query = self.query_with_cookie();
        let response = self.runtime.block_on(self.client.user_account(&query))?;
        self.capture_cookie(&response);
        Ok(response)
    }

    pub fn user_playlist_create(
        &mut self,
        uid: &str,
        limit: usize,
        offset: usize,
    ) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("uid", uid)
            .param("limit", &limit.max(1).to_string())
            .param("offset", &offset.to_string());
        let response = self
            .runtime
            .block_on(self.client.user_playlist_create(&query))?;
        Ok(response)
    }

    pub fn user_playlist_collect(
        &mut self,
        uid: &str,
        limit: usize,
        offset: usize,
    ) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("uid", uid)
            .param("limit", &limit.max(1).to_string())
            .param("offset", &offset.to_string());
        let response = self
            .runtime
            .block_on(self.client.user_playlist_collect(&query))?;
        Ok(response)
    }

    pub fn login_email(&mut self, email: &str, password: &str) -> Result<ApiResponse> {
        let query = Query::new()
            .param("email", email)
            .param("password", password);
        let response = self.runtime.block_on(self.client.login(&query))?;
        self.capture_cookie(&response);
        Ok(response)
    }

    pub fn captcha_sent(&mut self, phone: &str) -> Result<ApiResponse> {
        let query = Query::new().param("phone", phone);
        let response = self.runtime.block_on(self.client.captcha_sent(&query))?;
        Ok(response)
    }

    pub fn login_phone_captcha(&mut self, phone: &str, captcha: &str) -> Result<ApiResponse> {
        let query = Query::new().param("phone", phone).param("captcha", captcha);
        let response = self.runtime.block_on(self.client.login_cellphone(&query))?;
        self.capture_cookie(&response);
        Ok(response)
    }

    pub fn login_qr_key(&mut self) -> Result<ApiResponse> {
        let query = Query::new();
        let response = self.runtime.block_on(self.client.login_qr_key(&query))?;
        Ok(response)
    }

    pub fn login_qr_create(&mut self, key: &str) -> Result<ApiResponse> {
        let query = Query::new().param("key", key);
        let response = self.runtime.block_on(self.client.login_qr_create(&query))?;
        Ok(response)
    }

    pub fn login_qr_check(&mut self, key: &str) -> Result<ApiResponse> {
        let query = Query::new().param("key", key);
        let response = self.runtime.block_on(self.client.login_qr_check(&query))?;
        self.capture_cookie(&response);
        Ok(response)
    }

    pub fn recommend_resource(&mut self) -> Result<ApiResponse> {
        let query = self.query_with_cookie();
        let response = self
            .runtime
            .block_on(self.client.recommend_resource(&query))?;
        Ok(response)
    }

    pub fn recommend_songs(&mut self) -> Result<ApiResponse> {
        let query = self.query_with_cookie();
        let response = self.runtime.block_on(self.client.recommend_songs(&query))?;
        Ok(response)
    }

    pub fn personalized(&mut self, limit: usize) -> Result<ApiResponse> {
        let limit = limit.max(1).to_string();
        let query = self.query_with_cookie().param("limit", &limit);
        let response = self.runtime.block_on(self.client.personalized(&query))?;
        Ok(response)
    }

    pub fn playlist_detail(&mut self, id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("id", id);
        let response = self.runtime.block_on(self.client.playlist_detail(&query))?;
        Ok(response)
    }

    pub fn album(&mut self, id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("id", id);
        let response = self.runtime.block_on(self.client.album(&query))?;
        Ok(response)
    }

    pub fn artist_detail(&mut self, id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("id", id);
        let response = self.runtime.block_on(self.client.artist_detail(&query))?;
        Ok(response)
    }

    pub fn artist_desc(&mut self, id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("id", id);
        let response = self.runtime.block_on(self.client.artist_desc(&query))?;
        Ok(response)
    }

    pub fn artist_top_song(&mut self, id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("id", id);
        let response = self.runtime.block_on(self.client.artist_top_song(&query))?;
        Ok(response)
    }

    pub fn artist_album(&mut self, id: &str, limit: usize, offset: usize) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("id", id)
            .param("limit", &limit.max(1).to_string())
            .param("offset", &offset.to_string());
        let response = self.runtime.block_on(self.client.artist_album(&query))?;
        Ok(response)
    }

    pub fn artist_sublist(&mut self, limit: usize, offset: usize) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("limit", &limit.max(1).to_string())
            .param("offset", &offset.to_string());
        let response = self.runtime.block_on(self.client.artist_sublist(&query))?;
        Ok(response)
    }

    pub fn song_detail(&mut self, song_id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("ids", song_id);
        let response = self.runtime.block_on(self.client.song_detail(&query))?;
        Ok(response)
    }

    pub fn lyric(&mut self, song_id: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("id", song_id);
        let response = self.runtime.block_on(self.client.lyric(&query))?;
        Ok(response)
    }

    pub fn like_song(&mut self, song_id: &str, like: bool) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("id", song_id)
            .param("like", if like { "true" } else { "false" });
        let response = self.runtime.block_on(self.client.like(&query))?;
        Ok(response)
    }

    pub fn likelist(&mut self, uid: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("uid", uid);
        let response = self.runtime.block_on(self.client.likelist(&query))?;
        Ok(response)
    }

    pub fn song_like_check(&mut self, ids_json: &str) -> Result<ApiResponse> {
        let query = self.query_with_cookie().param("ids", ids_json);
        let response = self.runtime.block_on(self.client.song_like_check(&query))?;
        Ok(response)
    }

    pub fn song_url(&mut self, song_id: &str) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("id", song_id)
            .param("br", "320000");
        let response = self.runtime.block_on(self.client.song_url(&query))?;
        Ok(response)
    }

    pub fn song_url_v1(&mut self, song_id: &str, level: &str) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("id", song_id)
            .param("level", level);
        let response = self.runtime.block_on(self.client.song_url_v1(&query))?;
        Ok(response)
    }

    pub fn song_stream_url_with_quality(&mut self, song_id: &str, level: &str) -> Result<String> {
        let response = self.song_url_v1(song_id, level)?;
        if let Some(url) = response
            .body
            .pointer("/data/0/url")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Ok(url.to_string());
        }

        // Keep backward compatibility for songs that only expose legacy stream URLs.
        let fallback = self.song_url(song_id)?;
        if let Some(url) = fallback
            .body
            .pointer("/data/0/url")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Ok(url.to_string());
        }

        Err(anyhow!(
            "song stream url not found for id {} at level {}",
            song_id,
            level
        ))
    }

    pub fn vip_info(&mut self) -> Result<ApiResponse> {
        let query = self.query_with_cookie();
        let response = self.runtime.block_on(self.client.vip_info(&query))?;
        Ok(response)
    }

    pub fn vip_info_v2(&mut self) -> Result<ApiResponse> {
        let query = self.query_with_cookie();
        let response = self.runtime.block_on(self.client.vip_info_v2(&query))?;
        Ok(response)
    }

    pub fn search(
        &mut self,
        keywords: &str,
        search_type: i32,
        limit: usize,
        offset: usize,
    ) -> Result<ApiResponse> {
        let query = self
            .query_with_cookie()
            .param("keywords", keywords)
            .param("type", &search_type.to_string())
            .param("limit", &limit.max(1).to_string())
            .param("offset", &offset.to_string());
        let response = self.runtime.block_on(self.client.search(&query))?;
        Ok(response)
    }

    pub fn fetch_cover_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let url = url.trim();
        if url.is_empty() {
            return Ok(Vec::new());
        }

        let bytes = self
            .runtime
            .block_on(async {
                let response = self.http.get(url).send().await?;
                let mut response = response.error_for_status()?;

                if let Some(content_len) = response.content_length() {
                    if content_len > MAX_COVER_IMAGE_BYTES as u64 {
                        return Err(anyhow!(
                            "cover image exceeds {} byte limit",
                            MAX_COVER_IMAGE_BYTES
                        ));
                    }
                }

                let mut bytes = Vec::with_capacity(64 * 1024);
                while let Some(chunk) = response.chunk().await? {
                    if chunk.is_empty() {
                        continue;
                    }

                    if bytes.len().saturating_add(chunk.len()) > MAX_COVER_IMAGE_BYTES {
                        return Err(anyhow!(
                            "cover image exceeds {} byte limit",
                            MAX_COVER_IMAGE_BYTES
                        ));
                    }

                    bytes.extend_from_slice(&chunk);
                }

                Ok::<Vec<u8>, anyhow::Error>(bytes)
            })
            .with_context(|| format!("download cover image failed: {}", url))?;

        Ok(bytes)
    }

    pub fn fetch_audio_to_path(&self, url: &str, path: &Path) -> Result<()> {
        let url = url.trim();
        if url.is_empty() {
            return Err(anyhow!("audio url is empty"));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create audio cache dir failed: {}", parent.display()))?;
        }

        let tmp_path = path.with_extension("part");
        let result = self
            .runtime
            .block_on(async {
                let response = self.http.get(url).send().await?;
                let mut response = response.error_for_status()?;

                let mut file = File::create(&tmp_path).with_context(|| {
                    format!("create temp audio file failed: {}", tmp_path.display())
                })?;

                let mut written = 0u64;
                while let Some(chunk) = response.chunk().await? {
                    if chunk.is_empty() {
                        continue;
                    }
                    file.write_all(&chunk).with_context(|| {
                        format!("write temp audio file failed: {}", tmp_path.display())
                    })?;
                    written = written.saturating_add(chunk.len() as u64);
                }

                file.flush().with_context(|| {
                    format!("flush temp audio file failed: {}", tmp_path.display())
                })?;

                if written == 0 {
                    return Err(anyhow!("song audio payload is empty"));
                }

                Ok::<(), anyhow::Error>(())
            })
            .with_context(|| format!("stream audio to temp file failed: {}", url));

        if result.is_err() {
            let _ = fs::remove_file(&tmp_path);
            return result;
        }

        fs::rename(&tmp_path, path).with_context(|| {
            format!(
                "move temp audio cache file failed: {} -> {}",
                tmp_path.display(),
                path.display()
            )
        })?;
        Ok(())
    }

    fn query_with_cookie(&self) -> Query {
        if let Some(cookie) = self.cookie.as_deref() {
            return Query::new().cookie(cookie);
        }
        Query::new()
    }

    fn capture_cookie(&mut self, response: &ApiResponse) {
        if response.cookie.is_empty() {
            return;
        }

        let merged = response.cookie.join("; ");
        self.cookie = Some(merged.clone());
        self.client.set_cookie(merged);
    }
}
