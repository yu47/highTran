use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

const LARK_BASE: &str = "https://open.larksuite.com/open-apis";
const TOKEN_REFRESH_MARGIN_SECS: i64 = 300;

#[derive(Debug, Serialize, Deserialize)]
pub struct DriveItem {
    pub name: String,
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LarkCredential {
    #[serde(rename = "app", rename_all = "camelCase")]
    App {
        app_id: String,
        app_secret: String,
    },
    #[serde(rename = "token", rename_all = "camelCase")]
    Token { token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LarkAuthConfig {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LarkResponse<T> {
    code: i32,
    msg: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct TenantTokenResponse {
    code: i32,
    msg: Option<String>,
    tenant_access_token: Option<String>,
    expire: Option<i64>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    value: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct RootFolderData {
    token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LarkFile {
    #[serde(default)]
    token: String,
    #[serde(default)]
    file_token: Option<String>,
    #[serde(default)]
    folder_token: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(rename = "type")]
    file_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateFolderData {
    #[serde(default)]
    token: String,
    #[serde(default)]
    folder_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadFileData {
    file_token: Option<String>,
    token: Option<String>,
}

#[derive(Clone)]
pub struct LarkDriveClient {
    client: Client,
    auth: LarkAuthConfig,
    credential: LarkCredential,
    tenant_token: Arc<RwLock<Option<CachedToken>>>,
    direct_token: Arc<RwLock<Option<CachedToken>>>,
    root_token: Arc<RwLock<Option<String>>>,
}

impl LarkDriveClient {
    pub fn new(credential: LarkCredential) -> Self {
        let credential = match credential {
            LarkCredential::App { app_id, app_secret } => LarkCredential::App {
                app_id: app_id.trim().to_string(),
                app_secret: app_secret.trim().to_string(),
            },
            LarkCredential::Token { token } => LarkCredential::Token {
                token: normalize_token(&token),
            },
        };
        let auth = match &credential {
            LarkCredential::App { app_id, app_secret } => LarkAuthConfig {
                app_id: app_id.clone(),
                app_secret: app_secret.clone(),
            },
            LarkCredential::Token { .. } => LarkAuthConfig {
                app_id: String::new(),
                app_secret: String::new(),
            },
        };
        let client = Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            auth,
            credential,
            tenant_token: Arc::new(RwLock::new(None)),
            direct_token: Arc::new(RwLock::new(None)),
            root_token: Arc::new(RwLock::new(None)),
        }
    }

    async fn token(&self) -> Result<String, String> {
        match &self.credential {
            LarkCredential::App { .. } => self.ensure_tenant_token(false).await,
            LarkCredential::Token { token } => {
                let now = chrono::Utc::now().timestamp();
                if let Some(cached) = self.direct_token.read().await.clone() {
                    if cached.expires_at - TOKEN_REFRESH_MARGIN_SECS > now {
                        return Ok(cached.value);
                    }
                }

                let token = normalize_token(token);
                if token.is_empty() {
                    return Err("Lark token is missing. Please update Settings.".into());
                }

                let cached = CachedToken {
                    value: token.clone(),
                    expires_at: now + 86400,
                };
                *self.direct_token.write().await = Some(cached);
                Ok(token)
            }
        }
    }

    async fn refresh_token(&self) -> Result<String, String> {
        self.ensure_tenant_token(true).await
    }

    async fn ensure_tenant_token(&self, force_refresh: bool) -> Result<String, String> {
        if self.auth.app_id.is_empty() || self.auth.app_secret.is_empty() {
            return Err("Lark App ID/App Secret is missing. Please update Settings.".into());
        }

        let now = chrono::Utc::now().timestamp();
        if !force_refresh {
            if let Some(cached) = self.tenant_token.read().await.clone() {
                if cached.expires_at - TOKEN_REFRESH_MARGIN_SECS > now {
                    return Ok(cached.value);
                }
            }
        }

        let body = serde_json::json!({
            "app_id": self.auth.app_id,
            "app_secret": self.auth.app_secret
        });
        let resp = self
            .client
            .post(format!("{}/auth/v3/tenant_access_token/internal", LARK_BASE))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Get Lark tenant_access_token failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Get Lark tenant_access_token failed ({}): {}",
                status, body
            ));
        }

        let parsed: TenantTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse Lark tenant_access_token failed: {}", e))?;
        if parsed.code != 0 {
            return Err(format!(
                "Get Lark tenant_access_token failed: Lark code {}{}",
                parsed.code,
                parsed
                    .msg
                    .filter(|msg| !msg.is_empty())
                    .map(|msg| format!(" - {}", msg))
                    .unwrap_or_default()
            ));
        }

        let token = parsed
            .tenant_access_token
            .map(|token| normalize_token(&token))
            .filter(|token| !token.is_empty())
            .ok_or_else(|| "Get Lark tenant_access_token failed: missing token".to_string())?;
        let expire = parsed.expire.unwrap_or(7200).max(60);
        let cached = CachedToken {
            value: token.clone(),
            expires_at: now + expire,
        };
        *self.tenant_token.write().await = Some(cached);
        Ok(token)
    }

    fn lark_error<T>(action: &str, resp: LarkResponse<T>) -> String {
        format!(
            "{} failed: Lark code {}{}",
            action,
            resp.code,
            resp.msg
                .filter(|msg| !msg.is_empty())
                .map(|msg| format!(" - {}", msg))
                .unwrap_or_default()
        )
    }

    fn is_auth_code(code: i32) -> bool {
        matches!(code, 99991663 | 99991664 | 99991667 | 99991668 | 99991677)
    }

    async fn parse_lark_response<T>(&self, action: &str, resp: reqwest::Response) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("{} failed ({}): {}", action, status, body));
        }

        let parsed: LarkResponse<T> = resp
            .json()
            .await
            .map_err(|e| format!("Parse {} failed: {}", action, e))?;
        if parsed.code != 0 {
            return Err(Self::lark_error(action, parsed));
        }

        parsed
            .data
            .ok_or_else(|| format!("{} failed: missing data", action))
    }

    fn item_name(item: &LarkFile) -> &str {
        item.file_name.as_deref().unwrap_or(&item.name)
    }

    fn item_token(item: &LarkFile) -> String {
        if !item.token.is_empty() {
            return item.token.clone();
        }
        item.folder_token
            .clone()
            .or_else(|| item.file_token.clone())
            .unwrap_or_default()
    }

    fn is_folder(item: &LarkFile) -> bool {
        item.file_type.as_deref() == Some("folder") || item.folder_token.is_some()
    }

    async fn root_token(&self) -> Result<String, String> {
        if let Some(token) = self.root_token.read().await.clone() {
            return Ok(token);
        }

        let token = self.token().await?;
        let url = format!("{}/drive/explorer/v2/root_folder/meta", LARK_BASE);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Get Lark root folder failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Get Lark root folder failed ({}): {}",
                status, body
            ));
        }

        let parsed: LarkResponse<RootFolderData> = resp
            .json()
            .await
            .map_err(|e| format!("Parse Lark root folder failed: {}", e))?;
        if parsed.code != 0 {
            return Err(Self::lark_error("Get Lark root folder", parsed));
        }

        let data = parsed
            .data
            .ok_or_else(|| "Get Lark root folder failed: missing data".to_string())?;
        *self.root_token.write().await = Some(data.token.clone());
        Ok(data.token)
    }

    async fn list_folder_by_token(&self, folder_token: &str) -> Result<Vec<LarkFile>, String> {
        let token = self.token().await?;
        let mut page_token = String::new();
        let mut files = Vec::new();

        loop {
            let mut req = self
                .client
                .get(format!(
                    "{}/drive/explorer/v2/folder/{}/children",
                    LARK_BASE,
                    urlencoding(folder_token)
                ))
                .bearer_auth(&token)
                .query(&[("page_size", "200")]);

            if !page_token.is_empty() {
                req = req.query(&[("page_token", page_token.as_str())]);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| format!("List Lark folder failed: {}", e))?;

            if resp.status().as_u16() == 404 {
                return Ok(vec![]);
            }

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("List Lark folder failed ({}): {}", status, body));
            }

            let parsed: LarkResponse<Value> = resp
                .json()
                .await
                .map_err(|e| format!("Parse Lark folder list failed: {}", e))?;
            if parsed.code != 0 {
                return Err(Self::lark_error("List Lark folder", parsed));
            }

            let data = parsed
                .data
                .ok_or_else(|| "List Lark folder failed: missing data".to_string())?;
            files.extend(parse_lark_items(data.get("files")));
            files.extend(parse_lark_items(data.get("children")));

            let has_more = data
                .get("has_more")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !has_more {
                break;
            }

            page_token = data
                .get("next_page_token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if page_token.is_empty() {
                break;
            }
        }

        Ok(files)
    }

    async fn find_child(&self, parent_token: &str, name: &str) -> Result<Option<LarkFile>, String> {
        let items = self.list_folder_by_token(parent_token).await?;
        Ok(items.into_iter().find(|item| Self::item_name(item) == name))
    }

    async fn resolve_folder_token(&self, path: &str) -> Result<Option<String>, String> {
        let mut current = self.root_token().await?;
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();

        for part in parts {
            let child = self.find_child(&current, part).await?;
            match child {
                Some(item) if Self::is_folder(&item) => {
                    let token = Self::item_token(&item);
                    if token.is_empty() {
                        return Ok(None);
                    }
                    current = token;
                }
                _ => return Ok(None),
            }
        }

        Ok(Some(current))
    }

    async fn resolve_item(&self, path: &str) -> Result<Option<LarkFile>, String> {
        let normalized = path.trim_matches('/');
        if normalized.is_empty() {
            return Ok(None);
        }

        let mut parts: Vec<&str> = normalized
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        let name = parts.pop().ok_or_else(|| "Invalid Lark path".to_string())?;
        let parent_path = parts.join("/");
        let parent_token = match self.resolve_folder_token(&parent_path).await? {
            Some(token) => token,
            None => return Ok(None),
        };

        self.find_child(&parent_token, name).await
    }

    pub async fn create_folder(&self, parent_path: &str, folder_name: &str) -> Result<(), String> {
        let parent_token = self
            .resolve_folder_token(parent_path)
            .await?
            .ok_or_else(|| format!("Parent folder not found: {}", parent_path))?;

        if let Some(item) = self.find_child(&parent_token, folder_name).await? {
            if Self::is_folder(&item) {
                return Ok(());
            }
        }

        let token = self.token().await?;
        let body = serde_json::json!({
            "name": folder_name,
            "folder_token": parent_token
        });

        let resp = self
            .client
            .post(format!("{}/drive/v1/files/create_folder", LARK_BASE))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Create Lark folder failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create Lark folder failed ({}): {}", status, body));
        }

        let parsed: LarkResponse<CreateFolderData> = resp
            .json()
            .await
            .map_err(|e| format!("Parse Lark folder create failed: {}", e))?;
        if parsed.code != 0 {
            return Err(Self::lark_error("Create Lark folder", parsed));
        }

        let data = parsed
            .data
            .ok_or_else(|| "Create Lark folder failed: missing data".to_string())?;
        let _ = data
            .folder_token
            .filter(|token| !token.is_empty())
            .unwrap_or(data.token);
        Ok(())
    }

    pub async fn upload_file(&self, path: &str, data: Vec<u8>) -> Result<(), String> {
        let file_size = data.len().to_string();
        let normalized = path.trim_matches('/');
        let mut parts: Vec<&str> = normalized
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        let file_name = parts
            .pop()
            .ok_or_else(|| "Invalid upload path".to_string())?;
        let parent_path = parts.join("/");
        let parent_token = self
            .resolve_folder_token(&parent_path)
            .await?
            .ok_or_else(|| format!("Parent folder not found: {}", parent_path))?;

        if let Some(existing) = self.find_child(&parent_token, file_name).await? {
            let existing_token = Self::item_token(&existing);
            let _ = self
                .delete_item_by_token(&existing_token, existing.file_type.as_deref())
                .await;
        }

        let token = self.token().await?;
        let boundary = format!(
            "----hightran-lark-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let body = build_multipart_body(
            &boundary,
            &[
                ("file_name", file_name),
                ("parent_type", "explorer"),
                ("parent_node", parent_token.as_str()),
                ("size", file_size.as_str()),
            ],
            "file",
            file_name,
            data,
        );

        let resp = self
            .client
            .post(format!("{}/drive/v1/files/upload_all", LARK_BASE))
            .bearer_auth(&token)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Upload to Lark failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Upload to Lark failed ({}): {}", status, body));
        }

        let parsed: LarkResponse<UploadFileData> = resp
            .json()
            .await
            .map_err(|e| format!("Parse Lark upload failed: {}", e))?;
        if parsed.code != 0 {
            return Err(Self::lark_error("Upload to Lark", parsed));
        }

        if let Some(data) = parsed.data {
            let _ = data.file_token.or(data.token);
        }
        Ok(())
    }

    pub async fn download_file(&self, path: &str) -> Result<Vec<u8>, String> {
        let item = match self.resolve_item(path).await? {
            Some(item) => item,
            None => return Err("NOT_FOUND".into()),
        };

        let token = self.token().await?;
        let resp = self
            .client
            .get(format!(
                "{}/drive/v1/files/{}/download",
                LARK_BASE,
                urlencoding(&Self::item_token(&item))
            ))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Download from Lark failed: {}", e))?;

        if resp.status().as_u16() == 404 {
            return Err("NOT_FOUND".into());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Download from Lark failed ({}): {}", status, body));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Read Lark download failed: {}", e))
    }

    pub async fn list_children(&self, path: &str) -> Result<Vec<DriveItem>, String> {
        let folder_token = match self.resolve_folder_token(path).await? {
            Some(token) => token,
            None => return Ok(vec![]),
        };
        let items = self.list_folder_by_token(&folder_token).await?;
        Ok(items
            .into_iter()
            .map(|item| DriveItem {
                name: Self::item_name(&item).to_string(),
                id: Self::item_token(&item),
            })
            .collect())
    }

    pub async fn delete_item(&self, path: &str) -> Result<(), String> {
        if let Some(item) = self.resolve_item(path).await? {
            self.delete_item_by_token(&Self::item_token(&item), item.file_type.as_deref())
                .await?;
        }
        Ok(())
    }

    async fn delete_item_by_token(
        &self,
        item_token: &str,
        item_type: Option<&str>,
    ) -> Result<(), String> {
        let token = self.token().await?;
        let file_type = item_type.unwrap_or("file");
        let resp = self
            .client
            .delete(format!(
                "{}/drive/v1/files/{}",
                LARK_BASE,
                urlencoding(item_token)
            ))
            .bearer_auth(&token)
            .query(&[("type", file_type)])
            .send()
            .await
            .map_err(|e| format!("Delete Lark item failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Delete Lark item failed ({}): {}", status, body));
        }

        let parsed: LarkResponse<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Parse Lark delete failed: {}", e))?;
        if parsed.code != 0 {
            return Err(Self::lark_error("Delete Lark item", parsed));
        }

        Ok(())
    }

    pub async fn ensure_path(&self, path: &str) -> Result<(), String> {
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        let mut current = String::new();
        for part in &parts {
            let parent = if current.is_empty() {
                "/".to_string()
            } else {
                current.clone()
            };
            self.create_folder(&parent, part).await?;
            if current.is_empty() {
                current = part.to_string();
            } else {
                current = format!("{}/{}", current, part);
            }
        }
        Ok(())
    }
}

fn urlencoding(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('?', "%3F")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('#', "%23")
        .replace('+', "%2B")
        .replace('@', "%40")
}

fn build_multipart_body(
    boundary: &str,
    fields: &[(&str, &str)],
    file_field: &str,
    file_name: &str,
    file_data: Vec<u8>,
) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                escape_header_value(name)
            )
            .as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            escape_header_value(file_field),
            escape_header_value(file_name)
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(&file_data);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    body
}

fn escape_header_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_lark_items(value: Option<&Value>) -> Vec<LarkFile> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        Some(Value::Object(items)) => items
            .values()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn normalize_token(token: &str) -> String {
    token
        .trim()
        .strip_prefix("Bearer ")
        .or_else(|| token.trim().strip_prefix("bearer "))
        .unwrap_or_else(|| token.trim())
        .trim()
        .to_string()
}
