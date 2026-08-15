#[cfg(not(test))]
use keyring::Entry;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::time::SystemTime;

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
thread_local! {
    static MOCK_KEYRING: std::sync::Mutex<std::collections::HashMap<String, String>> = std::sync::Mutex::new(std::collections::HashMap::new());
}

fn set_secure_token(key: &str, token: &str) -> Result<(), String> {
    #[cfg(test)]
    {
        MOCK_KEYRING.with(|m| {
            m.lock().unwrap().insert(key.to_string(), token.to_string());
        });
        Ok(())
    }
    #[cfg(not(test))]
    {
        let entry = Entry::new("agent-erp-auth", key)
            .map_err(|e| format!("auth: Keyring init failed: {}", e))?;
        entry
            .set_password(token)
            .map_err(|e| format!("auth: Keyring store failed: {}", e))?;
        Ok(())
    }
}

fn get_secure_token(key: &str) -> Result<String, String> {
    #[cfg(test)]
    {
        MOCK_KEYRING.with(|m| {
            m.lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| "auth: Token not found in mock keyring".to_string())
        })
    }
    #[cfg(not(test))]
    {
        let entry = Entry::new("agent-erp-auth", key)
            .map_err(|e| format!("auth: Keyring init failed: {}", e))?;
        entry
            .get_password()
            .map_err(|e| format!("auth: Keyring retrieve failed: {}", e))
    }
}

#[allow(dead_code)]
fn delete_secure_token(key: &str) -> Result<(), String> {
    #[cfg(test)]
    {
        MOCK_KEYRING.with(|m| {
            m.lock().unwrap().remove(key);
        });
        Ok(())
    }
    #[cfg(not(test))]
    {
        let entry = Entry::new("agent-erp-auth", key)
            .map_err(|e| format!("auth: Keyring init failed: {}", e))?;
        let _ = entry.delete_password();
        Ok(())
    }
}

fn uuid_like_id() -> String {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", ts)
}

// REST call to real TPS2
async fn call_real_tps2(
    base_url: &str,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);

    let req = match method {
        "GET" => client.get(&url),
        "POST" => client.post(&url).json(body),
        "PUT" => client.put(&url).json(body),
        "DELETE" => client.delete(&url),
        _ => return Err(format!("auth: Unsupported HTTP method: {}", method)),
    };

    let res = req
        .send()
        .await
        .map_err(|e| format!("auth: HTTP request failed: {}", e))?;

    let status = res.status();
    let json_res: Value = res
        .json()
        .await
        .map_err(|e| format!("auth: Failed to parse JSON response: {}", e))?;

    if !status.is_success() {
        let err_reason = json_res
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN_ERROR");
        return Err(err_reason.to_string());
    }

    Ok(json_res)
}

// Local mock dispatch handling
async fn mock_dispatch<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<Value, String> {
    let db_path = crate::get_db_path(app_handle);
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("auth: Failed to open SQLite: {}", e))?;

    match (method, path) {
        ("POST", "/v1/auth/login") => {
            let email = body
                .get("email")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing email parameter")?;
            let password = body
                .get("password")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing password parameter")?;

            // Retrieve user credentials
            let mut stmt = conn
                .prepare("SELECT id, email, password FROM users WHERE email = ?1")
                .map_err(|e| format!("auth: Query prep failed: {}", e))?;

            let user_res = stmt.query_row([email], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            });

            match user_res {
                Ok((id, db_email, db_password)) => {
                    let hashed_password = hash_password(password);
                    if db_password != hashed_password {
                        return Err("IAM_ERR_INVALID_CREDENTIALS".to_string());
                    }

                    // Query user tenants
                    let mut stmt_tenants = conn
                        .prepare(
                            "SELECT t.id, t.code, t.name, ut.role FROM tenants t
                         JOIN user_tenants ut ON t.id = ut.tenant_id
                         WHERE ut.user_id = ?1",
                        )
                        .map_err(|e| format!("auth: Query prep failed: {}", e))?;

                    let tenant_rows = stmt_tenants
                        .query_map([&id], |row| {
                            Ok(json!({
                                "id": row.get::<_, String>(0)?,
                                "code": row.get::<_, String>(1)?,
                                "name": row.get::<_, String>(2)?,
                                "role": row.get::<_, String>(3)?
                            }))
                        })
                        .map_err(|e| format!("auth: Query execute failed: {}", e))?;

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
                            .and_then(|t| t.get("id"))
                            .and_then(|v| v.as_str())
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
                    ).map_err(|e| format!("auth: Failed to create session: {}", e))?;

                    Ok(json!({
                        "access_token": mock_token,
                        "refresh_token": format!("mock-refresh-{}", id),
                        "user": {
                            "id": id,
                            "email": db_email
                        },
                        "tenants": tenants
                    }))
                }
                Err(_) => Err("IAM_ERR_INVALID_CREDENTIALS".to_string()),
            }
        }

        ("POST", "/v1/auth/register-tenant") => {
            let tenant_name = body
                .get("tenant_name")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing tenant_name parameter")?;
            let company_name = body
                .get("company_name")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing company_name parameter")?;
            let admin_email = body
                .get("admin_email")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing admin_email parameter")?;
            let admin_password = body
                .get("admin_password")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing admin_password parameter")?;
            let tenant_code = body
                .get("tenant_code")
                .and_then(|v| v.as_str())
                .ok_or("auth: Missing tenant_code parameter")?;

            if admin_password.len() < 8 {
                return Err("IAM_ERR_WEAK_PASSWORD".to_string());
            }

            // Check duplicate email
            let mut stmt = conn
                .prepare("SELECT count(*) FROM users WHERE email = ?1")
                .map_err(|e| format!("auth: Query prep failed: {}", e))?;
            let count: i64 = stmt.query_row([admin_email], |row| row.get(0)).unwrap_or(0);
            if count > 0 {
                return Err("IAM_ERR_EMAIL_TAKEN".to_string());
            }

            let user_id = format!("usr_{}", uuid_like_id());
            let tenant_id = format!("tnt_{}", uuid_like_id());

            // Save user
            let hashed_admin_password = hash_password(admin_password);
            conn.execute(
                "INSERT INTO users (id, email, password) VALUES (?1, ?2, ?3)",
                (&user_id, admin_email, &hashed_admin_password),
            )
            .map_err(|e| format!("auth: Failed to register user: {}", e))?;

            // Save tenant
            conn.execute(
                "INSERT INTO tenants (id, code, name, company_name) VALUES (?1, ?2, ?3, ?4)",
                (&tenant_id, tenant_code, tenant_name, company_name),
            )
            .map_err(|e| format!("auth: Failed to create tenant: {}", e))?;

            // Save relation
            conn.execute(
                "INSERT INTO user_tenants (user_id, tenant_id, role) VALUES (?1, ?2, ?3)",
                (&user_id, &tenant_id, "admin"),
            )
            .map_err(|e| format!("auth: Failed to create user tenant relation: {}", e))?;

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
            .map_err(|e| format!("auth: Failed to create session: {}", e))?;

            Ok(json!({
                "access_token": mock_token,
                "refresh_token": format!("mock-refresh-{}", user_id),
                "user": {
                    "id": user_id,
                    "email": admin_email
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
                .ok_or("auth: Missing tenant_id parameter")?;

            let token = get_secure_token("access_token")?;

            // Retrieve user from current session
            let mut stmt = conn
                .prepare("SELECT user_id FROM sessions WHERE token = ?1")
                .map_err(|e| format!("auth: Query prep failed: {}", e))?;
            let user_id: String = stmt
                .query_row([&token], |row| row.get(0))
                .map_err(|_| "auth: Session not found or invalid token".to_string())?;

            // Verify if user is member of the tenant
            let mut stmt_member = conn
                .prepare("SELECT count(*) FROM user_tenants WHERE user_id = ?1 AND tenant_id = ?2")
                .map_err(|e| format!("auth: Query prep failed: {}", e))?;
            let is_member: i64 = stmt_member
                .query_row([&user_id, tenant_id], |row| row.get(0))
                .unwrap_or(0);

            if is_member == 0 {
                return Err("IAM_ERR_TENANT_NOT_ASSIGNED".to_string());
            }

            // Update session active tenant and return a scoped token
            let new_token = format!("mock-scoped-token-{}", user_id);
            conn.execute(
                "UPDATE sessions SET token = ?1, active_tenant_id = ?2 WHERE token = ?3",
                (&new_token, tenant_id, &token),
            )
            .map_err(|e| format!("auth: Failed to update session token: {}", e))?;

            Ok(json!({
                "access_token": new_token,
                "refresh_token": format!("mock-refresh-{}", user_id),
                "status": "authenticated"
            }))
        }

        _ => Err(format!(
            "auth: Mock endpoint not implemented: {} {}",
            method, path
        )),
    }
}

#[tauri::command]
pub async fn api_call<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    method: String,
    path: String,
    body: Value,
) -> Result<Value, String> {
    let base_url = env::var("TPS2_BASE_URL").ok();

    let mut response_val = match base_url {
        Some(url) => call_real_tps2(&url, &method, &path, &body).await?,
        None => mock_dispatch(&app_handle, &method, &path, &body).await?,
    };

    // Extract access and refresh tokens, securely store them, and delete them from the payload returned to Svelte/JS.
    if let Some(obj) = response_val.as_object_mut() {
        if let Some(access_token_val) = obj.remove("access_token") {
            if let Some(access_token) = access_token_val.as_str() {
                set_secure_token("access_token", access_token)?;
            }
        }
        if let Some(refresh_token_val) = obj.remove("refresh_token") {
            if let Some(refresh_token) = refresh_token_val.as_str() {
                set_secure_token("refresh_token", refresh_token)?;
            }
        }
    }

    Ok(response_val)
}

#[tauri::command]
pub async fn get_auth_status<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<Value, String> {
    let token = match get_secure_token("access_token") {
        Ok(t) => t,
        Err(_) => {
            return Ok(json!({
                "status": "unauthenticated",
                "user": null,
                "tenants": [],
                "activeTenant": null
            }));
        }
    };

    let db_path = crate::get_db_path(&app_handle);
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("auth: Failed to open SQLite: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT user_id, active_tenant_id FROM sessions WHERE token = ?1")
        .map_err(|e| format!("auth: Query prep failed: {}", e))?;

    let session_res = stmt.query_row([&token], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    });

    match session_res {
        Ok((user_id, active_tenant_id)) => {
            let mut stmt_user = conn
                .prepare("SELECT email FROM users WHERE id = ?1")
                .map_err(|e| format!("auth: Query prep failed: {}", e))?;
            let email: String = stmt_user
                .query_row([&user_id], |row| row.get(0))
                .map_err(|_| "auth: User record missing for session".to_string())?;

            let mut stmt_tenants = conn
                .prepare(
                    "SELECT t.id, t.code, t.name, ut.role FROM tenants t
                 JOIN user_tenants ut ON t.id = ut.tenant_id
                 WHERE ut.user_id = ?1",
                )
                .map_err(|e| format!("auth: Query prep failed: {}", e))?;

            let tenant_rows = stmt_tenants
                .query_map([&user_id], |row| {
                    Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "code": row.get::<_, String>(1)?,
                        "name": row.get::<_, String>(2)?,
                        "role": row.get::<_, String>(3)?
                    }))
                })
                .map_err(|e| format!("auth: Query execute failed: {}", e))?;

            let mut tenants = Vec::new();
            for t in tenant_rows {
                if let Ok(val) = t {
                    tenants.push(val);
                }
            }

            let active_tenant = if let Some(ref t_id) = active_tenant_id {
                tenants
                    .iter()
                    .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(t_id))
                    .cloned()
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

            Ok(json!({
                "status": status,
                "user": {
                    "id": user_id,
                    "email": email
                },
                "tenants": tenants,
                "activeTenant": active_tenant
            }))
        }
        Err(_) => Ok(json!({
            "status": "unauthenticated",
            "user": null,
            "tenants": [],
            "activeTenant": null
        })),
    }
}

#[tauri::command]
pub async fn logout<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String> {
    if let Ok(token) = get_secure_token("access_token") {
        let db_path = crate::get_db_path(&app_handle);
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            let _ = conn.execute("DELETE FROM sessions WHERE token = ?1", [&token]);
        }
    }

    let _ = delete_secure_token("access_token");
    let _ = delete_secure_token("refresh_token");
    Ok(())
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
                password TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE tenants (
                id TEXT PRIMARY KEY,
                code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                company_name TEXT NOT NULL
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
        let handle = setup_test_db();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = json!({
            "email": "test@example.com",
            "password": "password123"
        });

        let res = api_call(
            handle.clone(),
            "POST".to_string(),
            "/v1/auth/login".to_string(),
            req_body,
        )
        .await;
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
            "test@example.com"
        );

        // Ensure access_token/refresh_token was removed from response
        assert!(res_val.get("access_token").is_none());
        assert!(res_val.get("refresh_token").is_none());

        // Validate token actually stored in keyring
        let token = get_secure_token("access_token");
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), "mock-token-u1");
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let handle = setup_test_db();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'test@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = json!({
            "email": "test@example.com",
            "password": "wrong_password"
        });

        let res = api_call(
            handle,
            "POST".to_string(),
            "/v1/auth/login".to_string(),
            req_body,
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "IAM_ERR_INVALID_CREDENTIALS");
    }

    #[tokio::test]
    async fn test_register_tenant_success() {
        let handle = setup_test_db();

        let req_body = json!({
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        let res = api_call(
            handle.clone(),
            "POST".to_string(),
            "/v1/auth/register-tenant".to_string(),
            req_body,
        )
        .await;
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
        assert!(get_secure_token("access_token").is_ok());
    }

    #[tokio::test]
    async fn test_register_tenant_email_taken() {
        let handle = setup_test_db();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        conn.execute(
            "INSERT INTO users (id, email, password) VALUES ('u1', 'admin@example.com', ?1)",
            [hash_password("password123")],
        )
        .unwrap();

        let req_body = json!({
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        let res = api_call(
            handle,
            "POST".to_string(),
            "/v1/auth/register-tenant".to_string(),
            req_body,
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "IAM_ERR_EMAIL_TAKEN");
    }

    #[tokio::test]
    async fn test_register_tenant_weak_password() {
        let handle = setup_test_db();

        let req_body = json!({
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "weak",
            "tenant_code": "test_tnt"
        });

        let res = api_call(
            handle,
            "POST".to_string(),
            "/v1/auth/register-tenant".to_string(),
            req_body,
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "IAM_ERR_WEAK_PASSWORD");
    }

    #[tokio::test]
    async fn test_token_never_returned_to_js() {
        let handle = setup_test_db();

        let req_body = json!({
            "tenant_name": "Test Tenant",
            "company_name": "Test Company",
            "admin_email": "admin@example.com",
            "admin_password": "secure_password",
            "tenant_code": "test_tnt"
        });

        let res = api_call(
            handle.clone(),
            "POST".to_string(),
            "/v1/auth/register-tenant".to_string(),
            req_body,
        )
        .await;
        assert!(res.is_ok());

        let res_val = res.unwrap();
        assert!(!res_val.to_string().contains("mock-token-usr_"));
        assert!(!res_val.to_string().contains("access_token"));
        assert!(!res_val.to_string().contains("refresh_token"));
    }

    #[tokio::test]
    async fn test_select_tenant_success() {
        let handle = setup_test_db();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // Seed user, tenant, member relation, and active session
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

        // Set mock token in keyring
        set_secure_token("access_token", "mock-token-u1").unwrap();

        // Perform select-tenant call
        let req_body = json!({
            "tenant_id": "tnt1"
        });
        let res = api_call(
            handle.clone(),
            "POST".to_string(),
            "/v1/auth/select-tenant".to_string(),
            req_body,
        )
        .await;

        assert!(res.is_ok());
        let res_val = res.unwrap();
        assert_eq!(
            res_val.get("status").unwrap().as_str().unwrap(),
            "authenticated"
        );

        // Verify tokens are updated and saved in keyring
        let new_token = get_secure_token("access_token").unwrap();
        assert_eq!(new_token, "mock-scoped-token-u1");
    }

    #[tokio::test]
    async fn test_select_tenant_not_member() {
        let handle = setup_test_db();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // Seed user and session, but NO user_tenant relationship to tnt2
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

        set_secure_token("access_token", "mock-token-u1").unwrap();

        let req_body = json!({
            "tenant_id": "tnt2"
        });
        let res = api_call(
            handle,
            "POST".to_string(),
            "/v1/auth/select-tenant".to_string(),
            req_body,
        )
        .await;

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "IAM_ERR_TENANT_NOT_ASSIGNED");
    }

    #[tokio::test]
    async fn test_login_multi_tenant_requires_selection() {
        let handle = setup_test_db();
        let db_path = crate::get_db_path(&handle);
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // Seed user with 2 tenants
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

        // Perform login
        let req_body = json!({
            "email": "test@example.com",
            "password": "password123"
        });
        let login_res = api_call(
            handle.clone(),
            "POST".to_string(),
            "/v1/auth/login".to_string(),
            req_body,
        )
        .await;
        assert!(login_res.is_ok());

        // Get auth status
        let status_res = get_auth_status(handle).await;
        assert!(status_res.is_ok());

        let status_val = status_res.unwrap();
        // The user has multiple tenants, so they should need selection and activeTenant must be null
        assert_eq!(
            status_val.get("status").unwrap().as_str().unwrap(),
            "needs_tenant_selection"
        );
        assert!(status_val.get("activeTenant").unwrap().is_null());
        assert_eq!(
            status_val.get("tenants").unwrap().as_array().unwrap().len(),
            2
        );
    }
}
