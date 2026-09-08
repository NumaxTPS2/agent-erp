use crate::tps2_types::{V1LoginRequest, V1LoginResponse, V1TenantInfo};
use keyring::Entry;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::sync::RwLock;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

pub trait TokenStore: Send + Sync {
    fn save(&self, access: &str, refresh: &str) -> Result<(), ApiError>;
    fn load(&self) -> Result<Option<TokenPair>, ApiError>;
    fn clear(&self) -> Result<(), ApiError>;
}

#[derive(Debug, Default, Clone)]
pub struct KeyringTokenStore;

impl KeyringTokenStore {
    pub fn new() -> Self {
        Self
    }
}

impl TokenStore for KeyringTokenStore {
    fn save(&self, access: &str, refresh: &str) -> Result<(), ApiError> {
        let entry_access = Entry::new("agent-erp-auth", "access_token")
            .map_err(|e| ApiError::KeychainError(format!("Keyring init failed: {}", e)))?;
        entry_access
            .set_password(access)
            .map_err(|e| ApiError::KeychainError(format!("Keyring store failed: {}", e)))?;

        let entry_refresh = Entry::new("agent-erp-auth", "refresh_token")
            .map_err(|e| ApiError::KeychainError(format!("Keyring init failed: {}", e)))?;
        entry_refresh
            .set_password(refresh)
            .map_err(|e| ApiError::KeychainError(format!("Keyring store failed: {}", e)))?;

        Ok(())
    }

    fn load(&self) -> Result<Option<TokenPair>, ApiError> {
        let entry_access = Entry::new("agent-erp-auth", "access_token")
            .map_err(|e| ApiError::KeychainError(format!("Keyring init failed: {}", e)))?;
        let access_token = match entry_access.get_password() {
            Ok(pwd) => pwd,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(ApiError::KeychainError(format!("Keyring retrieve failed: {}", e))),
        };

        let entry_refresh = Entry::new("agent-erp-auth", "refresh_token")
            .map_err(|e| ApiError::KeychainError(format!("Keyring init failed: {}", e)))?;
        let refresh_token = match entry_refresh.get_password() {
            Ok(pwd) => Some(pwd),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => return Err(ApiError::KeychainError(format!("Keyring retrieve failed: {}", e))),
        };

        Ok(Some(TokenPair {
            access_token,
            refresh_token,
        }))
    }

    fn clear(&self) -> Result<(), ApiError> {
        let entry_access = Entry::new("agent-erp-auth", "access_token")
            .map_err(|e| ApiError::KeychainError(format!("Keyring init failed: {}", e)))?;
        let _ = entry_access.delete_password();

        let entry_refresh = Entry::new("agent-erp-auth", "refresh_token")
            .map_err(|e| ApiError::KeychainError(format!("Keyring init failed: {}", e)))?;
        let _ = entry_refresh.delete_password();

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct InMemoryTokenStore {
    tokens: RwLock<Option<TokenPair>>,
}

impl InMemoryTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(None),
        }
    }
}

impl TokenStore for InMemoryTokenStore {
    fn save(&self, access: &str, refresh: &str) -> Result<(), ApiError> {
        let mut lock = self
            .tokens
            .write()
            .map_err(|e| ApiError::KeychainError(format!("InMemory lock poisoned: {}", e)))?;
        *lock = Some(TokenPair {
            access_token: access.to_string(),
            refresh_token: if refresh.is_empty() {
                None
            } else {
                Some(refresh.to_string())
            },
        });
        Ok(())
    }

    fn load(&self) -> Result<Option<TokenPair>, ApiError> {
        let lock = self
            .tokens
            .read()
            .map_err(|e| ApiError::KeychainError(format!("InMemory lock poisoned: {}", e)))?;
        Ok(lock.clone())
    }

    fn clear(&self) -> Result<(), ApiError> {
        let mut lock = self
            .tokens
            .write()
            .map_err(|e| ApiError::KeychainError(format!("InMemory lock poisoned: {}", e)))?;
        *lock = None;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ApiError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("email already taken")]
    EmailTaken,
    #[error("weak password")]
    WeakPassword,
    #[error("tenant not assigned")]
    TenantNotAssigned,
    #[error("tenant code taken")]
    TenantCodeTaken,
    #[error("user locked")]
    UserLocked,
    #[error("provision failed")]
    ProvisionFailed,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("keychain error: {0}")]
    KeychainError(String),
    #[error("database error: {0}")]
    DatabaseError(String),
    #[error("network error: {0}")]
    NetworkError(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::InvalidCredentials => "IAM_ERR_INVALID_CREDENTIALS",
            ApiError::EmailTaken => "IAM_ERR_EMAIL_TAKEN",
            ApiError::WeakPassword => "IAM_ERR_WEAK_PASSWORD",
            ApiError::TenantNotAssigned => "IAM_ERR_TENANT_NOT_ASSIGNED",
            ApiError::TenantCodeTaken => "IAM_ERR_TENANT_CODE_TAKEN",
            ApiError::UserLocked => "IAM_ERR_USER_LOCKED",
            ApiError::ProvisionFailed => "IAM_ERR_PROVISION_FAILED",
            ApiError::InvalidArgument(_) => "IAM_ERR_INVALID_ARGUMENT",
            ApiError::KeychainError(_) => "LOCAL_ERR_KEYCHAIN",
            ApiError::DatabaseError(_) => "LOCAL_ERR_DATABASE",
            ApiError::NetworkError(_) => "LOCAL_ERR_NETWORK",
            ApiError::Unknown(_) => "UNKNOWN_ERROR",
        }
    }

    pub fn from_reason(reason: &str) -> Self {
        match reason {
            "IAM_ERR_INVALID_CREDENTIALS" => ApiError::InvalidCredentials,
            "IAM_ERR_EMAIL_TAKEN" => ApiError::EmailTaken,
            "IAM_ERR_WEAK_PASSWORD" => ApiError::WeakPassword,
            "IAM_ERR_TENANT_NOT_ASSIGNED" => ApiError::TenantNotAssigned,
            "IAM_ERR_TENANT_CODE_TAKEN" => ApiError::TenantCodeTaken,
            "IAM_ERR_USER_LOCKED" => ApiError::UserLocked,
            "IAM_ERR_PROVISION_FAILED" => ApiError::ProvisionFailed,
            other => ApiError::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type, Clone, PartialEq, Eq)]
pub struct ApiErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type, Clone, PartialEq, Eq)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type, Clone, PartialEq, Eq)]
pub struct AuthTenant {
    pub id: String,
    pub code: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type, Clone, PartialEq, Eq)]
pub struct AuthStatusResponse {
    pub status: String,
    pub user: Option<AuthUser>,
    pub tenants: Vec<AuthTenant>,
    #[serde(rename = "activeTenant")]
    pub active_tenant: Option<AuthTenant>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type, Clone, PartialEq, Eq)]
pub struct LogoutResponse {
    pub success: bool,
}

impl From<ApiError> for ApiErrorPayload {
    fn from(err: ApiError) -> Self {
        Self {
            code: err.code().to_string(),
            message: err.to_string(),
        }
    }
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

fn uuid_like_id() -> String {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", ts)
}

fn extract_error_reason(json: &Value) -> Option<&str> {
    // 1. Direct "reason" at root level
    if let Some(r) = json.get("reason").and_then(|v| v.as_str()) {
        return Some(r);
    }

    // 2. Direct "details" array (gRPC-gateway / google.rpc.Status)
    if let Some(details) = json.get("details").and_then(|v| v.as_array()) {
        for item in details {
            if let Some(r) = item.get("reason").and_then(|v| v.as_str()) {
                return Some(r);
            }
        }
    }

    // 3. Nested under "error" object
    if let Some(error_obj) = json.get("error").and_then(|v| v.as_object()) {
        if let Some(r) = error_obj.get("reason").and_then(|v| v.as_str()) {
            return Some(r);
        }
        if let Some(details) = error_obj.get("details").and_then(|v| v.as_array()) {
            for item in details {
                if let Some(r) = item.get("reason").and_then(|v| v.as_str()) {
                    return Some(r);
                }
            }
        }
    }

    // 4. "error_code" or "code" string representation
    if let Some(c) = json.get("error_code").and_then(|v| v.as_str()) {
        return Some(c);
    }
    if let Some(c) = json.get("code").and_then(|v| v.as_str()) {
        return Some(c);
    }

    None
}

fn parse_http_response(status: reqwest::StatusCode, text: &str) -> Result<Value, ApiError> {
    let json_res: Option<Value> = serde_json::from_str(text).ok();

    if !status.is_success() {
        if let Some(ref j) = json_res {
            if let Some(reason) = extract_error_reason(j) {
                return Err(ApiError::from_reason(reason));
            }
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::InvalidCredentials);
        }

        if let Some(ref j) = json_res {
            if let Some(msg) = j.get("message").and_then(|v| v.as_str()) {
                return Err(ApiError::Unknown(msg.to_string()));
            }
            if let Some(msg) = j
                .get("error")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
            {
                return Err(ApiError::Unknown(msg.to_string()));
            }
        }

        return Err(ApiError::Unknown(format!("HTTP {}", status)));
    }

    let json_val = json_res
        .ok_or_else(|| ApiError::NetworkError("Empty or invalid JSON response".to_string()))?;
    Ok(json_val)
}

fn parse_profile_response(profile_val: &Value) -> AuthStatusResponse {
    let user_id = profile_val
        .get("userId")
        .or_else(|| profile_val.get("user_id"))
        .or_else(|| profile_val.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let email = profile_val
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let display_name = profile_val
        .get("displayName")
        .or_else(|| profile_val.get("display_name"))
        .or_else(|| profile_val.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| email.clone());

    let mut tenants = Vec::new();
    if let Some(arr) = profile_val.get("tenants").and_then(|v| v.as_array()) {
        for item in arr {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let code = item
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let role = item
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            tenants.push(AuthTenant {
                id,
                code,
                name,
                role,
            });
        }
    }

    let mut active_tenant = None;
    if let Some(act) = profile_val
        .get("activeTenant")
        .or_else(|| profile_val.get("active_tenant"))
    {
        if let (Some(id), Some(code), Some(name)) = (
            act.get("id").and_then(|v| v.as_str()),
            act.get("code").and_then(|v| v.as_str()),
            act.get("name").and_then(|v| v.as_str()),
        ) {
            let role = act
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            active_tenant = Some(AuthTenant {
                id: id.to_string(),
                code: code.to_string(),
                name: name.to_string(),
                role,
            });
        }
    } else if tenants.len() == 1 {
        active_tenant = tenants.first().cloned();
    }

    let status = if let Some(st) = profile_val.get("status").and_then(|v| v.as_str()) {
        st.to_string()
    } else if tenants.is_empty() {
        "needs_tenant_creation".to_string()
    } else if active_tenant.is_none() {
        "needs_tenant_selection".to_string()
    } else {
        "authenticated".to_string()
    };

    let user = if user_id.is_empty() && email.is_empty() {
        None
    } else {
        Some(AuthUser {
            id: user_id,
            email,
            display_name,
        })
    };

    AuthStatusResponse {
        status,
        user,
        tenants,
        active_tenant,
    }
}

// REST call to real TPS2
async fn call_real_tps2<S: TokenStore>(
    token_store: &S,
    base_url: &str,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<Value, ApiError> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);

    let mut req = match method {
        "GET" => client.get(&url),
        "POST" => {
            if body.is_null() {
                client.post(&url)
            } else {
                client.post(&url).json(body)
            }
        }
        "PUT" => {
            if body.is_null() {
                client.put(&url)
            } else {
                client.put(&url).json(body)
            }
        }
        "DELETE" => {
            if body.is_null() {
                client.delete(&url)
            } else {
                client.delete(&url).json(body)
            }
        }
        _ => {
            return Err(ApiError::InvalidArgument(format!(
                "Unsupported HTTP method: {}",
                method
            )))
        }
    };

    if let Ok(Some(pair)) = token_store.load() {
        if !pair.access_token.trim().is_empty() {
            req = req.bearer_auth(pair.access_token);
        }
    }

    if let Ok(cf_id) = env::var("CF_ACCESS_CLIENT_ID") {
        if !cf_id.trim().is_empty() {
            req = req.header("CF-Access-Client-Id", cf_id.trim());
        }
    }
    if let Ok(cf_secret) = env::var("CF_ACCESS_CLIENT_SECRET") {
        if !cf_secret.trim().is_empty() {
            req = req.header("CF-Access-Client-Secret", cf_secret.trim());
        }
    }

    let res = req
        .send()
        .await
        .map_err(|e| ApiError::NetworkError(format!("HTTP request failed: {}", e)))?;

    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| ApiError::NetworkError(format!("Failed to read response body: {}", e)))?;

    parse_http_response(status, &text)
}

// Local mock dispatch handling
async fn mock_dispatch<R: tauri::Runtime, S: TokenStore>(
    app_handle: &tauri::AppHandle<R>,
    token_store: &S,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<Value, ApiError> {
    let db_path = crate::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| ApiError::DatabaseError(format!("Failed to open SQLite: {}", e)))?;

    match (method, path) {
        ("POST", "/v1/auth/login") => {
            let req: V1LoginRequest = serde_json::from_value(body.clone())
                .map_err(|e| ApiError::InvalidArgument(format!("Invalid login request: {}", e)))?;
            let email = req
                .email
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| ApiError::InvalidArgument("Missing email parameter".to_string()))?;
            let password = req
                .password
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| ApiError::InvalidArgument("Missing password parameter".to_string()))?;

            // Retrieve user credentials
            let mut stmt = conn
                .prepare("SELECT id, email, password, name FROM users WHERE email = ?1")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;

            let user_res = stmt.query_row([email], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3).unwrap_or_default(),
                ))
            });

            match user_res {
                Ok((id, db_email, db_password, db_name)) => {
                    let hashed_password = hash_password(password);
                    if db_password != hashed_password {
                        return Err(ApiError::InvalidCredentials);
                    }

                    // Query user tenants
                    let mut stmt_tenants = conn
                        .prepare(
                            "SELECT t.id, t.code, t.name, ut.role FROM tenants t
                         JOIN user_tenants ut ON t.id = ut.tenant_id
                         WHERE ut.user_id = ?1",
                        )
                        .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;

                    let tenant_rows = stmt_tenants
                        .query_map([&id], |row| {
                            Ok(V1TenantInfo {
                                id: Some(row.get::<_, String>(0)?),
                                code: Some(row.get::<_, String>(1)?),
                                name: Some(row.get::<_, String>(2)?),
                                role: Some(row.get::<_, String>(3)?),
                            })
                        })
                        .map_err(|e| ApiError::DatabaseError(format!("Query execute failed: {}", e)))?;

                    let mut tenants = Vec::new();
                    for t in tenant_rows {
                        if let Ok(val) = t {
                            tenants.push(val);
                        }
                    }

                    let mock_token = format!("mock-token-{}", id);
                    let active_tenant_id = if tenants.len() == 1 {
                        tenants
                            .first()
                            .and_then(|t| t.id.as_deref())
                    } else {
                        None
                    };

                    conn.execute(
                        "INSERT OR REPLACE INTO sessions (token, user_id, active_tenant_id, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        (
                            &mock_token,
                            &id,
                            active_tenant_id,
                            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs() as i64
                        )
                    ).map_err(|e| ApiError::DatabaseError(format!("Failed to create session: {}", e)))?;

                    let display_name = if db_name.trim().is_empty() {
                        db_email.clone()
                    } else {
                        db_name
                    };

                    let login_resp = V1LoginResponse {
                        access_token: Some(mock_token),
                        refresh_token: Some(format!("mock-refresh-{}", id)),
                        user_id: Some(id),
                        email: Some(db_email),
                        display_name: Some(display_name),
                        tenants,
                        ..Default::default()
                    };

                    serde_json::to_value(login_resp)
                        .map_err(|e| ApiError::DatabaseError(format!("Serialization failed: {}", e)))
                }
                Err(_) => Err(ApiError::InvalidCredentials),
            }
        }

        ("POST", "/v1/auth/register-tenant") => {
            let admin_name = body
                .get("admin_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing admin_name parameter".to_string()))?;
            let tenant_name = body
                .get("tenant_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing tenant_name parameter".to_string()))?;
            let company_name = body
                .get("company_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing company_name parameter".to_string()))?;
            let admin_email = body
                .get("admin_email")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing admin_email parameter".to_string()))?;
            let admin_password = body
                .get("admin_password")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing admin_password parameter".to_string()))?;
            let tenant_code = body
                .get("tenant_code")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing tenant_code parameter".to_string()))?;

            if admin_name.trim().is_empty() {
                return Err(ApiError::InvalidArgument("admin_name cannot be empty".to_string()));
            }

            if admin_password.len() < 8 {
                return Err(ApiError::WeakPassword);
            }

            // Check duplicate email
            let mut stmt = conn
                .prepare("SELECT count(*) FROM users WHERE email = ?1")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
            let count: i64 = stmt.query_row([admin_email], |row| row.get(0)).unwrap_or(0);
            if count > 0 {
                return Err(ApiError::EmailTaken);
            }

            let user_id = format!("usr_{}", uuid_like_id());
            let tenant_id = format!("tnt_{}", uuid_like_id());

            // Save user with name
            let hashed_admin_password = hash_password(admin_password);
            conn.execute(
                "INSERT INTO users (id, email, password, name) VALUES (?1, ?2, ?3, ?4)",
                (&user_id, admin_email, &hashed_admin_password, admin_name),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to register user: {}", e)))?;

            // Save tenant
            conn.execute(
                "INSERT INTO tenants (id, code, name, company_name) VALUES (?1, ?2, ?3, ?4)",
                (&tenant_id, tenant_code, tenant_name, company_name),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to create tenant: {}", e)))?;

            // Save relation
            conn.execute(
                "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES (?1, ?2, ?3)",
                (&user_id, &tenant_id, "admin"),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to create user tenant relation: {}", e)))?;

            // Create session
            let mock_token = format!("mock-token-{}", user_id);
            conn.execute(
                "INSERT OR REPLACE INTO sessions (token, user_id, active_tenant_id, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                (
                    &mock_token,
                    &user_id,
                    &tenant_id,
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                ),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to create session: {}", e)))?;

            Ok(json!({
                "access_token": mock_token,
                "refresh_token": format!("mock-refresh-{}", user_id),
                "user": {
                    "id": user_id,
                    "email": admin_email,
                    "display_name": admin_name
                },
                "tenants": [
                    {
                        "id": tenant_id,
                        "code": tenant_code,
                        "name": tenant_name,
                        "role": "admin"
                    }
                ]
            }))
        }

        ("POST", "/v1/auth/select-tenant") => {
            let tenant_id = body
                .get("tenant_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing tenant_id parameter".to_string()))?;

            let pair = token_store
                .load()?
                .ok_or(ApiError::InvalidCredentials)?;
            let token = pair.access_token;

            // Retrieve user from current session
            let mut stmt = conn
                .prepare("SELECT user_id FROM sessions WHERE token = ?1")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
            let user_id: String = stmt
                .query_row([&token], |row| row.get(0))
                .map_err(|_| ApiError::InvalidCredentials)?;

            // Verify if user is member of the tenant
            let mut stmt_member = conn
                .prepare("SELECT count(*) FROM user_tenants WHERE user_id = ?1 AND tenant_id = ?2")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
            let is_member: i64 = stmt_member
                .query_row([&user_id, tenant_id], |row| row.get(0))
                .unwrap_or(0);

            if is_member == 0 {
                return Err(ApiError::TenantNotAssigned);
            }

            // Update session active tenant and return a scoped token
            let new_token = format!("mock-scoped-token-{}", user_id);
            conn.execute(
                "UPDATE sessions SET token = ?1, active_tenant_id = ?2 WHERE token = ?3",
                (&new_token, tenant_id, &token),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to update session token: {}", e)))?;

            Ok(json!({
                "access_token": new_token,
                "refresh_token": format!("mock-refresh-{}", user_id),
                "status": "authenticated"
            }))
        }

        ("POST", "/v1/auth/create-tenant") => {
            let tenant_name = body
                .get("tenant_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing tenant_name parameter".to_string()))?;
            let company_name = body
                .get("company_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing company_name parameter".to_string()))?;
            let tenant_code = body
                .get("tenant_code")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::InvalidArgument("Missing tenant_code parameter".to_string()))?;
            let tax_id = body.get("tax_id").and_then(|v| v.as_str());

            let pair = token_store
                .load()?
                .ok_or(ApiError::InvalidCredentials)?;
            let token = pair.access_token;

            // Retrieve user from current session
            let mut stmt = conn
                .prepare("SELECT user_id FROM sessions WHERE token = ?1")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
            let user_id: String = stmt
                .query_row([&token], |row| row.get(0))
                .map_err(|_| ApiError::InvalidCredentials)?;

            // Retrieve user email
            let mut stmt_user = conn
                .prepare("SELECT email FROM users WHERE id = ?1")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
            let email: String = stmt_user
                .query_row([&user_id], |row| row.get(0))
                .map_err(|_| ApiError::DatabaseError("User record missing".to_string()))?;

            // Check if tenant_code is already taken
            let mut stmt_check = conn
                .prepare("SELECT count(*) FROM tenants WHERE code = ?1")
                .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
            let count: i64 = stmt_check
                .query_row([tenant_code], |row| row.get(0))
                .unwrap_or(0);
            if count > 0 {
                return Err(ApiError::TenantCodeTaken);
            }

            let tenant_id = format!("tnt_{}", uuid_like_id());

            // Save tenant
            conn.execute(
                "INSERT INTO tenants (id, code, name, company_name, tax_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                (&tenant_id, tenant_code, tenant_name, company_name, tax_id),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to create tenant: {}", e)))?;

            // Save user tenant relation (owner/admin)
            conn.execute(
                "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES (?1, ?2, ?3)",
                (&user_id, &tenant_id, "admin"),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to create user tenant relation: {}", e)))?;

            // Update session with new active tenant and a new scoped token
            let new_token = format!("mock-scoped-token-{}", user_id);
            conn.execute(
                "UPDATE sessions SET token = ?1, active_tenant_id = ?2 WHERE token = ?3",
                (&new_token, &tenant_id, &token),
            )
            .map_err(|e| ApiError::DatabaseError(format!("Failed to update session token: {}", e)))?;

            Ok(json!({
                "access_token": new_token,
                "refresh_token": format!("mock-refresh-{}", user_id),
                "user_id": user_id,
                "email": email,
                "tenant_id": tenant_id,
                "company_id": format!("cmp_{}", uuid_like_id())
            }))
        }

        ("POST", "/v1/auth/logout") => {
            if let Ok(Some(pair)) = token_store.load() {
                let _ = conn.execute("DELETE FROM sessions WHERE token = ?1", [&pair.access_token]);
            }
            Ok(json!({}))
        }

        ("POST", "/v1/test/expire") => Err(ApiError::InvalidCredentials),

        _ => Err(ApiError::InvalidArgument(format!(
            "Mock endpoint not implemented: {} {}",
            method, path
        ))),
    }
}

pub(crate) async fn execute_api_call<R: tauri::Runtime, S: TokenStore>(
    app_handle: &tauri::AppHandle<R>,
    token_store: &S,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<Value, ApiError> {
    let base_url = env::var("TPS2_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());

    let response_result = match base_url {
        Some(url) => call_real_tps2(token_store, &url, method, path, body).await,
        None => mock_dispatch(app_handle, token_store, method, path, body).await,
    };

    match response_result {
        Ok(mut response_val) => {
            if path == "/v1/auth/logout" {
                let _ = token_store.clear();
            } else if let Some(obj) = response_val.as_object_mut() {
                let access_opt = obj
                    .remove("access_token")
                    .or_else(|| obj.remove("accessToken"))
                    .and_then(|v| v.as_str().map(String::from));
                let refresh_opt = obj
                    .remove("refresh_token")
                    .or_else(|| obj.remove("refreshToken"))
                    .and_then(|v| v.as_str().map(String::from));

                if let Some(access) = access_opt {
                    let refresh = refresh_opt.as_deref().unwrap_or("");
                    token_store.save(&access, refresh)?;
                }
            }
            Ok(response_val)
        }
        Err(err) => {
            if path != "/v1/auth/login" && err == ApiError::InvalidCredentials {
                let _ = token_store.clear();
            }
            Err(err)
        }
    }
}

pub(crate) async fn execute_get_auth_status<R: tauri::Runtime, S: TokenStore>(
    app_handle: &tauri::AppHandle<R>,
    token_store: &S,
) -> Result<AuthStatusResponse, ApiError> {
    let token = match token_store.load() {
        Ok(Some(pair)) if !pair.access_token.trim().is_empty() => pair.access_token,
        _ => {
            return Ok(AuthStatusResponse {
                status: "unauthenticated".to_string(),
                user: None,
                tenants: Vec::new(),
                active_tenant: None,
            });
        }
    };

    let base_url = env::var("TPS2_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());

    if let Some(url) = base_url {
        match call_real_tps2(token_store, &url, "GET", "/v1/auth/profile", &Value::Null).await {
            Ok(profile_val) => Ok(parse_profile_response(&profile_val)),
            Err(ApiError::InvalidCredentials) => {
                let _ = token_store.clear();
                Ok(AuthStatusResponse {
                    status: "unauthenticated".to_string(),
                    user: None,
                    tenants: Vec::new(),
                    active_tenant: None,
                })
            }
            Err(err) => Err(err),
        }
    } else {
        let db_path = crate::get_db_path(app_handle);
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| ApiError::DatabaseError(format!("Failed to open SQLite: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT user_id, active_tenant_id FROM sessions WHERE token = ?1")
            .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;

        let session_res = stmt.query_row([&token], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        });

        match session_res {
            Ok((user_id, active_tenant_id)) => {
                let mut stmt_user = conn
                    .prepare("SELECT email, name FROM users WHERE id = ?1")
                    .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;
                let (email, name): (String, String) = stmt_user
                    .query_row([&user_id], |row| {
                        Ok((row.get(0)?, row.get::<_, String>(1).unwrap_or_default()))
                    })
                    .map_err(|_| {
                        ApiError::DatabaseError("User record missing for session".to_string())
                    })?;

                let mut stmt_tenants = conn
                    .prepare(
                        "SELECT t.id, t.code, t.name, ut.role FROM tenants t
                     JOIN user_tenants ut ON t.id = ut.tenant_id
                     WHERE ut.user_id = ?1",
                    )
                    .map_err(|e| ApiError::DatabaseError(format!("Query prep failed: {}", e)))?;

                let tenant_rows = stmt_tenants
                    .query_map([&user_id], |row| {
                        Ok(AuthTenant {
                            id: row.get::<_, String>(0)?,
                            code: row.get::<_, String>(1)?,
                            name: row.get::<_, String>(2)?,
                            role: row.get::<_, String>(3)?,
                        })
                    })
                    .map_err(|e| ApiError::DatabaseError(format!("Query execute failed: {}", e)))?;

                let mut tenants = Vec::new();
                for t in tenant_rows {
                    if let Ok(val) = t {
                        tenants.push(val);
                    }
                }

                let active_tenant = if let Some(ref t_id) = active_tenant_id {
                    tenants.iter().find(|t| &t.id == t_id).cloned()
                } else {
                    None
                };

                let status = if tenants.is_empty() {
                    "needs_tenant_creation"
                } else if active_tenant.is_none() {
                    "needs_tenant_selection"
                } else {
                    "authenticated"
                };

                let display_name = if name.trim().is_empty() {
                    email.clone()
                } else {
                    name
                };

                Ok(AuthStatusResponse {
                    status: status.to_string(),
                    user: Some(AuthUser {
                        id: user_id,
                        email,
                        display_name,
                    }),
                    tenants,
                    active_tenant,
                })
            }
            Err(_) => Ok(AuthStatusResponse {
                status: "unauthenticated".to_string(),
                user: None,
                tenants: Vec::new(),
                active_tenant: None,
            }),
        }
    }
}

pub(crate) async fn execute_logout<R: tauri::Runtime, S: TokenStore>(
    app_handle: &tauri::AppHandle<R>,
    token_store: &S,
) -> Result<(), ApiError> {
    let base_url = env::var("TPS2_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    if let Some(url) = base_url {
        let _ = call_real_tps2(token_store, &url, "POST", "/v1/auth/logout", &json!({})).await;
    } else if let Ok(Some(pair)) = token_store.load() {
        let db_path = crate::get_db_path(app_handle);
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            let _ = conn.execute("DELETE FROM sessions WHERE token = ?1", [&pair.access_token]);
        }
    }

    let _ = token_store.clear();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn api_call<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    method: String,
    path: String,
    body: Value,
) -> Result<Value, ApiErrorPayload> {
    let token_store = KeyringTokenStore::new();
    execute_api_call(&app_handle, &token_store, &method, &path, &body)
        .await
        .map_err(ApiErrorPayload::from)
}

#[tauri::command]
#[specta::specta]
pub async fn get_auth_status<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<AuthStatusResponse, ApiErrorPayload> {
    let token_store = KeyringTokenStore::new();
    execute_get_auth_status(&app_handle, &token_store)
        .await
        .map_err(ApiErrorPayload::from)
}

#[tauri::command]
#[specta::specta]
pub async fn logout<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<LogoutResponse, ApiErrorPayload> {
    let token_store = KeyringTokenStore::new();
    execute_logout(&app_handle, &token_store)
        .await
        .map(|_| LogoutResponse { success: true })
        .map_err(ApiErrorPayload::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn setup_test_db() -> tauri::AppHandle<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        let handle = app.handle().clone();

        let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("agent_erp_test_{}_{}.db", uuid_like_id(), counter);
        let mut db_path = std::path::PathBuf::from("target");
        if !db_path.exists() {
            let _ = std::fs::create_dir_all(&db_path).unwrap();
        }
        db_path.push(db_name);

        crate::TEST_DB_PATH.with(|path| {
            *path.borrow_mut() = Some(db_path.clone());
        });

        if db_path.exists() {
            let _ = std::fs::remove_file(&db_path);
        }

        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "CREATE TABLE users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                name TEXT NOT NULL DEFAULT ''
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE tenants (
                id TEXT PRIMARY KEY,
                code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                company_name TEXT NOT NULL,
                tax_id TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE user_tenants (
                user_id TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                role TEXT NOT NULL,
                PRIMARY KEY (user_id, tenant_id)
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE sessions (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                active_tenant_id TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();

        handle
    }

    #[tokio::test]
    async fn test_login_success() {
        // Given: setup user with name in test database and in-memory token store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password, name) VALUES ('u1', 'test@example.com', ?1, 'Test Admin')",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = serde_json::to_value(V1LoginRequest {
            email: Some("test@example.com".to_string()),
            password: Some("password123".to_string()),
            ..Default::default()
        })
        .unwrap();

        // When: user logs in with valid credentials
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &req_body,
        )
        .await;

        // Then: login succeeds and returns display_name
        assert!(res.is_ok());

        let res_val = res.unwrap();
        let login_resp: V1LoginResponse = serde_json::from_value(res_val.clone()).unwrap();
        assert_eq!(login_resp.email.as_deref(), Some("test@example.com"));
        assert_eq!(login_resp.display_name.as_deref(), Some("Test Admin"));
        assert_eq!(login_resp.user_id.as_deref(), Some("u1"));

        // Ensure access_token/refresh_token was removed from response
        assert!(res_val.get("access_token").is_none());
        assert!(res_val.get("accessToken").is_none());
        assert!(res_val.get("refresh_token").is_none());
        assert!(res_val.get("refreshToken").is_none());

        // Validate token actually stored in token_store
        let pair = token_store.load().unwrap();
        assert!(pair.is_some());
        assert_eq!(pair.unwrap().access_token, "mock-token-u1");
    }

    #[tokio::test]
    async fn test_login_fallback_display_name_to_email() {
        // Given: user with empty name in database
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password, name) VALUES ('u1', 'noname@example.com', ?1, '')",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = serde_json::to_value(V1LoginRequest {
            email: Some("noname@example.com".to_string()),
            password: Some("password123".to_string()),
            ..Default::default()
        })
        .unwrap();

        // When: user logs in
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &req_body,
        )
        .await;

        // Then: display_name falls back to email
        assert!(res.is_ok());
        let res_val = res.unwrap();
        let login_resp: V1LoginResponse = serde_json::from_value(res_val).unwrap();
        assert_eq!(login_resp.display_name.as_deref(), Some("noname@example.com"));
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        // Given: setup user in test database and in-memory token store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password, name) VALUES ('u1', 'test@example.com', ?1, 'Test User')",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = json!({
            "email": "test@example.com",
            "password": "wrong_password"
        });

        // When: calling internal execute_api_call with wrong password
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &req_body,
        )
        .await;

        // Then: returns ApiError::InvalidCredentials enum variant
        assert_eq!(res.unwrap_err(), ApiError::InvalidCredentials);

        // And When: calling tauri command api_call across IPC boundary
        let ipc_res = api_call(
            handle,
            "POST".to_string(),
            "/v1/auth/login".to_string(),
            req_body,
        )
        .await;

        // Then: returns ApiErrorPayload with IAM_ERR_INVALID_CREDENTIALS
        assert_eq!(
            ipc_res.unwrap_err(),
            ApiErrorPayload {
                code: "IAM_ERR_INVALID_CREDENTIALS".to_string(),
                message: "invalid credentials".to_string(),
            }
        );
    }
    #[tokio::test]
    async fn test_login_missing_parameters() {
        // Given: setup user in test database and in-memory token store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        // When: calling login with missing email
        let res_no_email = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &json!({
                "password": "password123"
            }),
        )
        .await;

        // Then: returns InvalidArgument error
        assert!(matches!(res_no_email.unwrap_err(), ApiError::InvalidArgument(_)));

        // When: calling login with empty email
        let res_empty_email = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &json!({
                "email": "   ",
                "password": "password123"
            }),
        )
        .await;

        // Then: returns InvalidArgument error
        assert!(matches!(res_empty_email.unwrap_err(), ApiError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn test_register_tenant_success() {
        // Given: setup fresh test database and in-memory token store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        let req_body = json!({
            "admin_name": "Peter Chen",
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        // When: registering new tenant
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/register-tenant",
            &req_body,
        )
        .await;

        // Then: registration succeeds and returns admin_name as display_name
        assert!(res.is_ok());

        let res_val = res.unwrap();
        assert_eq!(
            res_val
                .get("user")
                .unwrap()
                .get("email")
                .unwrap()
                .as_str()
                .unwrap(),
            "admin@example.com"
        );
        assert_eq!(
            res_val
                .get("user")
                .unwrap()
                .get("display_name")
                .unwrap()
                .as_str()
                .unwrap(),
            "Peter Chen"
        );
        assert_eq!(res_val.get("tenants").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(
            res_val.get("tenants").unwrap().as_array().unwrap()[0]
                .get("code")
                .unwrap()
                .as_str()
                .unwrap(),
            "test_tnt"
        );

        // Verify tokens are stored but not returned
        assert!(res_val.get("access_token").is_none());
        assert!(res_val.get("refresh_token").is_none());
        assert!(token_store.load().unwrap().is_some());
    }

    #[tokio::test]
    async fn test_register_tenant_missing_or_empty_admin_name() {
        // Given: fresh database
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        let req_body_empty = json!({
            "admin_name": "   ",
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        // When: registering with empty admin_name
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/register-tenant",
            &req_body_empty,
        )
        .await;

        // Then: returns InvalidArgument error
        assert!(matches!(res.unwrap_err(), ApiError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn test_register_tenant_email_taken() {
        // Given: database with existing email
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password, name) VALUES ('u1', 'admin@example.com', ?1, 'Admin')",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = json!({
            "admin_name": "Peter Chen",
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        // When: registering with duplicate email
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/register-tenant",
            &req_body,
        )
        .await;

        // Then: returns ApiError::EmailTaken
        assert_eq!(res.unwrap_err(), ApiError::EmailTaken);
    }

    #[tokio::test]
    async fn test_register_tenant_weak_password() {
        // Given: fresh database
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        let req_body = json!({
            "admin_name": "Peter Chen",
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "weak",
            "tenant_code": "test_tnt"
        });

        // When: registering with weak password
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/register-tenant",
            &req_body,
        )
        .await;

        // Then: returns ApiError::WeakPassword
        assert_eq!(res.unwrap_err(), ApiError::WeakPassword);
    }

    #[tokio::test]
    async fn test_token_never_returned_to_js() {
        // Given: fresh database
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        let req_body = json!({
            "admin_name": "Peter Chen",
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        // When: registering tenant
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/register-tenant",
            &req_body,
        )
        .await;

        // Then: response does not contain any access or refresh tokens
        assert!(res.is_ok());
        let res_val = res.unwrap();
        assert!(!res_val.to_string().contains("mock-token-usr_"));
        assert!(!res_val.to_string().contains("access_token"));
        assert!(!res_val.to_string().contains("refresh_token"));
    }

    #[tokio::test]
    async fn test_select_tenant_success() {
        // Given: Seed user, tenant, member relation, active session and token in token_store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tenants (id, code, name, company_name) VALUES ('tnt1', 'tenant1', 'Tenant 1', 'Company 1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES ('u1', 'tnt1', 'admin')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-u1', 'u1', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-u1", "mock-refresh-u1").unwrap();

        // When: Perform select-tenant call
        let req_body = json!({
            "tenant_id": "tnt1"
        });
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/select-tenant",
            &req_body,
        )
        .await;

        // Then: selection succeeds and returns authenticated status
        assert!(res.is_ok());
        let res_val = res.unwrap();
        assert_eq!(
            res_val.get("status").unwrap().as_str().unwrap(),
            "authenticated"
        );

        // Verify tokens are updated and saved in token_store
        let pair = token_store.load().unwrap().unwrap();
        assert_eq!(pair.access_token, "mock-scoped-token-u1");
    }

    #[tokio::test]
    async fn test_select_tenant_not_member() {
        // Given: Seed user and session, but NO user_tenant relationship to tnt2
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-u1', 'u1', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-u1", "mock-refresh-u1").unwrap();

        let req_body = json!({
            "tenant_id": "tnt2"
        });

        // When: selecting tenant that user does not belong to
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/select-tenant",
            &req_body,
        )
        .await;

        // Then: returns ApiError::TenantNotAssigned
        assert_eq!(res.unwrap_err(), ApiError::TenantNotAssigned);
    }

    #[tokio::test]
    async fn test_login_multi_tenant_requires_selection() {
        // Given: user with 2 tenants
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tenants (id, code, name, company_name) VALUES ('tnt1', 'tenant1', 'Tenant 1', 'Company 1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tenants (id, code, name, company_name) VALUES ('tnt2', 'tenant2', 'Tenant 2', 'Company 2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES ('u1', 'tnt1', 'admin')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES ('u1', 'tnt2', 'member')",
            [],
        )
        .unwrap();

        // When: Perform login
        let req_body = json!({
            "email": "test@example.com",
            "password": "password123"
        });
        let login_res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &req_body,
        )
        .await;
        assert!(login_res.is_ok());

        // Then: Get auth status should indicate needs_tenant_selection
        let status_res = execute_get_auth_status(&handle, &token_store).await;
        assert!(status_res.is_ok());

        let status_val = status_res.unwrap();
        assert_eq!(status_val.status, "needs_tenant_selection");
        assert!(status_val.active_tenant.is_none());
        assert_eq!(status_val.tenants.len(), 2);
    }

    #[tokio::test]
    async fn test_create_tenant_success() {
        // Given: Seed user and active session without active_tenant_id
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-u1', 'u1', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-u1", "mock-refresh-u1").unwrap();

        let req_body = json!({
            "tenant_name": "New Tenant",
            "company_name": "New Company",
            "tenant_code": "new_tnt",
            "tax_id": "12345678"
        });

        // When: creating new tenant
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/create-tenant",
            &req_body,
        )
        .await;

        // Then: tenant creation succeeds
        assert!(res.is_ok());
        let res_val = res.unwrap();
        assert_eq!(res_val.get("user_id").unwrap().as_str().unwrap(), "u1");
        assert_eq!(
            res_val.get("email").unwrap().as_str().unwrap(),
            "test@example.com"
        );
        assert!(res_val.get("tenant_id").is_some());
        assert!(res_val.get("company_id").is_some());

        // Verify session was updated to the new scoped token and has active_tenant_id set
        let pair = token_store.load().unwrap().unwrap();
        assert_eq!(pair.access_token, "mock-scoped-token-u1");

        // Verify status is authenticated
        let status_res = execute_get_auth_status(&handle, &token_store).await.unwrap();
        assert_eq!(status_res.status, "authenticated");
        assert_eq!(status_res.active_tenant.unwrap().code, "new_tnt");
    }

    #[tokio::test]
    async fn test_create_tenant_duplicate_code() {
        // Given: Seed user, existing tenant with same code, and active session
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tenants (id, code, name, company_name, tax_id) VALUES ('tnt_existing', 'dup_tnt', 'Existing', 'Existing Corp', NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-u1', 'u1', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-u1", "mock-refresh-u1").unwrap();

        let req_body = json!({
            "tenant_name": "New Tenant",
            "company_name": "New Company",
            "tenant_code": "dup_tnt",
            "tax_id": ""
        });

        // When: creating tenant with duplicate code
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/create-tenant",
            &req_body,
        )
        .await;

        // Then: returns ApiError::TenantCodeTaken
        assert_eq!(res.unwrap_err(), ApiError::TenantCodeTaken);
    }

    #[test]
    fn test_in_memory_token_store_save_load_clear_roundtrip() {
        // Given: fresh InMemoryTokenStore and tokens
        let store = InMemoryTokenStore::new();
        let access = "test_access_token_value";
        let refresh = "test_refresh_token_value";

        // When: initially loading
        let initial = store.load().unwrap();
        // Then: should be None
        assert_eq!(initial, None);

        // When: saving token pair
        store.save(access, refresh).unwrap();

        // Then: we should retrieve the same token pair
        let retrieved = store.load().unwrap().unwrap();
        assert_eq!(retrieved.access_token, access);
        assert_eq!(retrieved.refresh_token, Some(refresh.to_string()));

        // When: clearing the store
        store.clear().unwrap();

        // Then: load should return None
        assert_eq!(store.load().unwrap(), None);
    }

    #[tokio::test]
    async fn test_api_call_logout_success() {
        // Given: setup test database with a user, active session and in-memory tokens
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-u1', 'u1', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-u1", "mock-refresh-u1").unwrap();

        // When: calling execute_api_call with logout path
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/logout",
            &json!({}),
        )
        .await;

        // Then: the call should succeed
        assert!(res.is_ok());

        // And: the session should be deleted from SQLite
        let mut stmt = conn
            .prepare("SELECT count(*) FROM sessions WHERE token = 'mock-token-u1'")
            .unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);

        // And: Tokens should be cleared in token_store
        assert_eq!(token_store.load().unwrap(), None);
    }

    #[tokio::test]
    async fn test_api_call_expired_token_clears_keychain() {
        // Given: mock token stored in token_store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        token_store.save("mock-token-u1", "mock-refresh-u1").unwrap();

        // When: calling execute_api_call with an endpoint that returns invalid credentials
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/test/expire",
            &json!({}),
        )
        .await;

        // Then: returns ApiError::InvalidCredentials
        assert_eq!(res.unwrap_err(), ApiError::InvalidCredentials);

        // And: Tokens should be automatically cleared
        assert_eq!(token_store.load().unwrap(), None);
    }

    #[tokio::test]
    async fn test_api_call_login_failure_does_not_clear_keychain() {
        // Given: setup test database, seed previous active token
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        token_store.save("mock-token-prev", "mock-refresh-prev").unwrap();

        // When: login fails with invalid credentials
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &json!({
                "email": "test@example.com",
                "password": "wrong_password"
            }),
        )
        .await;

        // Then: returns ApiError::InvalidCredentials
        assert_eq!(res.unwrap_err(), ApiError::InvalidCredentials);

        // And: the previous token should STILL exist (not cleared)
        let pair = token_store.load().unwrap().unwrap();
        assert_eq!(pair.access_token, "mock-token-prev");
        assert_eq!(pair.refresh_token, Some("mock-refresh-prev".to_string()));
    }

    // ==========================================
    // Issue #30 Layer 1 Acceptance Criteria Tests
    // ==========================================

    #[tokio::test]
    async fn test_get_auth_status_unauthenticated() {
        // Given: no token in token_store
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        // When: checking auth status
        let res = execute_get_auth_status(&handle, &token_store).await;

        // Then: should return unauthenticated
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val.status, "unauthenticated");
        assert!(val.user.is_none());
        assert!(val.active_tenant.is_none());
        assert!(val.tenants.is_empty());
    }

    #[tokio::test]
    async fn test_get_auth_status_needs_tenant_creation() {
        // Given: user logged in with session but has 0 tenants assigned
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u_new', 'newuser@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-unew', 'u_new', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-unew", "").unwrap();

        // When: checking auth status
        let res = execute_get_auth_status(&handle, &token_store).await;

        // Then: should return needs_tenant_creation
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val.status, "needs_tenant_creation");
        assert_eq!(val.user.unwrap().email, "newuser@example.com");
        assert!(val.tenants.is_empty());
    }

    #[tokio::test]
    async fn test_get_auth_status_needs_tenant_selection() {
        // Given: user with multiple tenants but active_tenant_id is NULL
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u_multi', 'multi@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tenants (id, code, name, company_name) VALUES ('tnt_a', 'code_a', 'Tenant A', 'Comp A'), ('tnt_b', 'code_b', 'Tenant B', 'Comp B')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES ('u_multi', 'tnt_a', 'admin'), ('u_multi', 'tnt_b', 'member')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-umulti', 'u_multi', NULL, 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-umulti", "").unwrap();

        // When: checking auth status
        let res = execute_get_auth_status(&handle, &token_store).await;

        // Then: should return needs_tenant_selection
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val.status, "needs_tenant_selection");
        assert_eq!(val.tenants.len(), 2);
        assert!(val.active_tenant.is_none());
    }

    #[tokio::test]
    async fn test_get_auth_status_authenticated() {
        // Given: user with active_tenant_id selected in session
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u_auth', 'auth@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tenants (id, code, name, company_name) VALUES ('tnt_act', 'code_act', 'Active Tenant', 'Active Comp')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES ('u_auth', 'tnt_act', 'admin')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (token, user_id, active_tenant_id, created_at) VALUES ('mock-token-uauth', 'u_auth', 'tnt_act', 1234567890)",
            [],
        )
        .unwrap();

        token_store.save("mock-token-uauth", "").unwrap();

        // When: checking auth status
        let res = execute_get_auth_status(&handle, &token_store).await;

        // Then: should return authenticated with activeTenant populated
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val.status, "authenticated");
        assert_eq!(val.active_tenant.unwrap().code, "code_act");
    }

    #[tokio::test]
    async fn test_command_get_auth_status_delivery_boundary() {
        let handle = setup_test_db();
        let res = get_auth_status(handle).await;
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val.status, "unauthenticated");
    }

    #[tokio::test]
    async fn test_command_logout_delivery_boundary() {
        let handle = setup_test_db();
        let res = logout(handle).await;
        assert!(res.is_ok());
        let val = res.unwrap();
        assert!(val.success);
    }

    #[test]
    fn test_extract_error_reason_variations() {
        // Given: different JSON structures
        let top_level = json!({ "reason": "IAM_ERR_INVALID_CREDENTIALS" });
        let details_array = json!({
            "code": 3,
            "message": "invalid password",
            "details": [
                {
                    "@type": "type.googleapis.com/google.rpc.ErrorInfo",
                    "reason": "IAM_ERR_WEAK_PASSWORD",
                    "domain": "iam.numax.com"
                }
            ]
        });
        let error_nested = json!({
            "error": {
                "reason": "IAM_ERR_EMAIL_TAKEN",
                "details": []
            }
        });
        let error_nested_details = json!({
            "error": {
                "details": [
                    { "reason": "IAM_ERR_USER_LOCKED" }
                ]
            }
        });
        let error_code_field = json!({ "error_code": "IAM_ERR_TENANT_NOT_ASSIGNED" });
        let empty_obj = json!({ "message": "plain error" });

        // When & Then: reasons are extracted accurately
        assert_eq!(
            extract_error_reason(&top_level),
            Some("IAM_ERR_INVALID_CREDENTIALS")
        );
        assert_eq!(
            extract_error_reason(&details_array),
            Some("IAM_ERR_WEAK_PASSWORD")
        );
        assert_eq!(
            extract_error_reason(&error_nested),
            Some("IAM_ERR_EMAIL_TAKEN")
        );
        assert_eq!(
            extract_error_reason(&error_nested_details),
            Some("IAM_ERR_USER_LOCKED")
        );
        assert_eq!(
            extract_error_reason(&error_code_field),
            Some("IAM_ERR_TENANT_NOT_ASSIGNED")
        );
        assert_eq!(extract_error_reason(&empty_obj), None);
    }

    #[test]
    fn test_parse_http_response_success() {
        // Given: 200 OK status and valid JSON body
        let status = reqwest::StatusCode::OK;
        let text = r#"{"accessToken":"t1","user":{"id":"u1"}}"#;

        // When: parsing HTTP response
        let res = parse_http_response(status, text);

        // Then: succeeds with parsed JSON
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val.get("accessToken").unwrap(), "t1");
    }

    #[test]
    fn test_parse_http_response_401_unauthorized() {
        // Given: 401 Unauthorized status
        let status = reqwest::StatusCode::UNAUTHORIZED;
        let text = r#"{"message":"unauthorized"}"#;

        // When: parsing HTTP response
        let res = parse_http_response(status, text);

        // Then: returns InvalidCredentials
        assert_eq!(res.unwrap_err(), ApiError::InvalidCredentials);
    }

    #[test]
    fn test_parse_http_response_grpc_error_info_mapping() {
        // Given: status codes and response bodies with google.rpc.ErrorInfo
        let cases = vec![
            (
                reqwest::StatusCode::BAD_REQUEST,
                r#"{"details":[{"reason":"IAM_ERR_WEAK_PASSWORD"}]}"#,
                ApiError::WeakPassword,
            ),
            (
                reqwest::StatusCode::CONFLICT,
                r#"{"details":[{"reason":"IAM_ERR_EMAIL_TAKEN"}]}"#,
                ApiError::EmailTaken,
            ),
            (
                reqwest::StatusCode::FORBIDDEN,
                r#"{"details":[{"reason":"IAM_ERR_TENANT_NOT_ASSIGNED"}]}"#,
                ApiError::TenantNotAssigned,
            ),
            (
                reqwest::StatusCode::CONFLICT,
                r#"{"details":[{"reason":"IAM_ERR_TENANT_CODE_TAKEN"}]}"#,
                ApiError::TenantCodeTaken,
            ),
            (
                reqwest::StatusCode::FORBIDDEN,
                r#"{"details":[{"reason":"IAM_ERR_USER_LOCKED"}]}"#,
                ApiError::UserLocked,
            ),
            (
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"details":[{"reason":"IAM_ERR_PROVISION_FAILED"}]}"#,
                ApiError::ProvisionFailed,
            ),
            (
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"message":"server crashed"}"#,
                ApiError::Unknown("server crashed".to_string()),
            ),
        ];

        for (status, body, expected_err) in cases {
            // When: parsing HTTP response
            let res = parse_http_response(status, body);

            // Then: error is correctly mapped
            assert_eq!(res.unwrap_err(), expected_err);
        }
    }

    #[test]
    fn test_parse_profile_response_single_and_multi_tenant() {
        // Given: profile with single tenant
        let single_val = json!({
            "userId": "usr_1",
            "email": "user1@example.com",
            "displayName": "User One",
            "tenants": [
                { "id": "tnt_1", "code": "code1", "name": "Tenant 1", "role": "admin" }
            ]
        });

        // When: parsing single tenant profile
        let single_res = parse_profile_response(&single_val);

        // Then: status is authenticated and active tenant is selected
        assert_eq!(single_res.status, "authenticated");
        assert_eq!(single_res.user.as_ref().unwrap().email, "user1@example.com");
        assert_eq!(single_res.active_tenant.as_ref().unwrap().code, "code1");

        // Given: profile with multiple tenants without active tenant
        let multi_val = json!({
            "userId": "usr_2",
            "email": "user2@example.com",
            "displayName": "User Two",
            "tenants": [
                { "id": "tnt_1", "code": "code1", "name": "Tenant 1", "role": "admin" },
                { "id": "tnt_2", "code": "code2", "name": "Tenant 2", "role": "member" }
            ]
        });

        // When: parsing multi tenant profile
        let multi_res = parse_profile_response(&multi_val);

        // Then: status is needs_tenant_selection
        assert_eq!(multi_res.status, "needs_tenant_selection");
        assert_eq!(multi_res.tenants.len(), 2);
        assert!(multi_res.active_tenant.is_none());

        // Given: profile with no tenants
        let empty_val = json!({
            "userId": "usr_3",
            "email": "user3@example.com",
            "tenants": []
        });

        // When: parsing empty tenants profile
        let empty_res = parse_profile_response(&empty_val);

        // Then: status is needs_tenant_creation
        assert_eq!(empty_res.status, "needs_tenant_creation");
    }

    #[tokio::test]
    async fn test_call_real_tps2_unsupported_method_and_network_error() {
        // Given: token store and dummy url
        let token_store = InMemoryTokenStore::new();

        // When: invoking unsupported HTTP method
        let res_method = call_real_tps2(&token_store, "http://127.0.0.1:9", "PATCH", "/v1/auth", &Value::Null).await;

        // Then: returns InvalidArgument
        assert!(matches!(res_method.unwrap_err(), ApiError::InvalidArgument(_)));

        // When: invoking invalid/unreachable host
        let res_net = call_real_tps2(&token_store, "http://127.0.0.1:1", "GET", "/v1/auth", &Value::Null).await;

        // Then: returns NetworkError
        assert!(matches!(res_net.unwrap_err(), ApiError::NetworkError(_)));
    }

    #[tokio::test]
    async fn test_execute_api_call_empty_base_url_falls_back_to_mock() {
        // Given: TPS2_BASE_URL set to whitespace or empty string
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();
        std::env::set_var("TPS2_BASE_URL", "   ");

        // When: calling api_call for unregistered email
        let res = execute_api_call(
            &handle,
            &token_store,
            "POST",
            "/v1/auth/login",
            &json!({"email": "none@example.com", "password": "pass"}),
        )
        .await;

        // Then: fallback to local SQLite mock returns InvalidCredentials
        assert_eq!(res.unwrap_err(), ApiError::InvalidCredentials);

        std::env::remove_var("TPS2_BASE_URL");
    }

    #[tokio::test]
    async fn test_execute_get_auth_status_no_token_returns_unauthenticated() {
        // Given: token store without token
        let handle = setup_test_db();
        let token_store = InMemoryTokenStore::new();

        // When: get auth status
        let res = execute_get_auth_status(&handle, &token_store).await;

        // Then: returns unauthenticated
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status, "unauthenticated");
    }

    #[test]
    fn test_export_specta_bindings() {
        let builder = tauri_specta::Builder::<tauri::Wry>::new()
            .commands(tauri_specta::collect_commands![
                api_call::<tauri::Wry>,
                get_auth_status::<tauri::Wry>,
                logout::<tauri::Wry>
            ]);

        builder
            .export(
                specta_typescript::Typescript::default(),
                "../src/lib/bindings.d.ts",
            )
            .expect("Failed to export specta typescript bindings");
    }
}
