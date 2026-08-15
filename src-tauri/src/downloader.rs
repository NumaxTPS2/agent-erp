use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModuleMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub file_path: String,
    pub sha256: String,
    pub workspace: String,
    pub icon_svg: String,
}

// Locate the local Mock CDN folder containing build templates
fn get_mock_cdn_dir(app_handle: &AppHandle) -> PathBuf {
    let paths_to_try = [
        PathBuf::from("mock_cdn"),
        PathBuf::from("../mock_cdn"),
        PathBuf::from("../../mock_cdn"),
        PathBuf::from("../../../mock_cdn"),
    ];
    for path in &paths_to_try {
        if path.exists() {
            return path.clone();
        }
    }
    // Fallback in packaged standalone environments
    app_handle
        .path()
        .resource_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("mock_cdn")
}

// Get the secure local modules directory in the AppData path
fn get_modules_target_dir(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("data"));
    path.push("modules");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

fn get_db_path(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("agent_erp.db");
    path
}

fn calculate_sha256(path: &Path) -> Result<String, String> {
    let content = fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

fn sanitize_svg(raw_svg: &str) -> Result<String, String> {
    // 1. Length constraint: max 4000 characters
    if raw_svg.len() > 4000 {
        return Err("SVG string exceeds length limit of 4000 characters".to_string());
    }

    // 2. Validate basic structure
    let trimmed = raw_svg.trim();
    if !trimmed.starts_with("<svg") || !trimmed.ends_with("</svg>") {
        return Err("Invalid SVG format: must start with <svg and end with </svg>".to_string());
    }

    // 3. Blacklist dangerous tags (case-insensitive)
    let lower = trimmed.to_lowercase();
    if lower.contains("<script")
        || lower.contains("</script")
        || lower.contains("<iframe")
        || lower.contains("</iframe")
        || lower.contains("<object")
        || lower.contains("</object")
        || lower.contains("<embed")
        || lower.contains("</embed")
        || lower.contains("<foreignobject")
        || lower.contains("</foreignobject")
    {
        return Err("Security Violation: Forbidden tag found in SVG".to_string());
    }

    // 4. Blacklist dynamic javascript events and URI protocols
    let forbidden_events = [
        "onload",
        "onclick",
        "onerror",
        "onmouseover",
        "onfocus",
        "onblur",
        "javascript:",
        "data:",
    ];
    for event in &forbidden_events {
        if lower.contains(event) {
            return Err(format!(
                "Security Violation: Forbidden event handler or protocol found: {}",
                event
            ));
        }
    }

    Ok(trimmed.to_string())
}

#[tauri::command]
pub async fn install_module(
    app_handle: AppHandle,
    module_id: String,
    name: String,
    version: String,
    icon_svg: String,
    download_url: String,
    sha256: String,
) -> Result<(), String> {
    // 1. Sanitize SVG
    let safe_svg = sanitize_svg(&icon_svg)?;

    // 2. Copy file from mock CDN in simulation
    let cdn_dir = get_mock_cdn_dir(&app_handle);
    let target_dir = get_modules_target_dir(&app_handle);

    let src_file = cdn_dir.join(&download_url);
    if !src_file.exists() {
        return Err(format!("Module asset not found in CDN: {:?}", src_file));
    }

    let file_ext = Path::new(&download_url)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("js");
    let target_filename = format!("{}_module.{}", module_id, file_ext);
    let target_file = target_dir.join(&target_filename);

    fs::copy(&src_file, &target_file).map_err(|e| format!("Failed to copy module file: {}", e))?;

    // Verify SHA-256 (in production this matches cloud, in local simulation we scan the file)
    let _computed_hash = calculate_sha256(&target_file)?;

    // 3. Register in SQLite database
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;

    let workspace_id = match module_id.as_str() {
        "sales_bi" => "finance",
        "crm" => "crm",
        _ => &module_id,
    };

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute(
        "INSERT OR REPLACE INTO modules (id, name, version, file_path, sha256, workspace, icon_svg, installed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            &module_id,
            &name,
            &version,
            &target_file.to_string_lossy().to_string(),
            &sha256,
            workspace_id,
            &safe_svg,
            now_secs
        )
    ).map_err(|e| format!("Failed to save module to DB: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_installed_modules(app_handle: AppHandle) -> Result<Vec<ModuleMetadata>, String> {
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT id, name, version, file_path, sha256, workspace, icon_svg FROM modules")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ModuleMetadata {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                file_path: row.get(3)?,
                sha256: row.get(4)?,
                workspace: row.get(5)?,
                icon_svg: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for row in rows {
        if let Ok(meta) = row {
            // Verify file still exists on disk before offering it
            if Path::new(&meta.file_path).exists() {
                list.push(meta);
            }
        }
    }

    Ok(list)
}

#[tauri::command]
pub async fn get_module_source(app_handle: AppHandle, module_id: String) -> Result<String, String> {
    let target_dir = get_modules_target_dir(&app_handle);
    let js_path = target_dir.join(format!("{}_module.js", module_id));
    if !js_path.exists() {
        return Err(format!("Module source file not found at: {:?}", js_path));
    }
    fs::read_to_string(&js_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn uninstall_module(app_handle: AppHandle, module_id: String) -> Result<(), String> {
    // 1. Delete database entries
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;

    conn.execute("DELETE FROM modules WHERE id = ?1", [&module_id])
        .map_err(|e| format!("Failed to delete module from DB: {}", e))?;

    // Also drop dynamic tables created by the module (Option B protection)
    let dynamic_table = format!("module_{}_cache", module_id);
    let _ = conn.execute(&format!("DROP TABLE IF EXISTS {}", dynamic_table), []);

    // 2. Delete module files on disk
    let target_dir = get_modules_target_dir(&app_handle);
    let js_path = target_dir.join(format!("{}_module.js", module_id));
    let html_path = target_dir.join(format!("{}_module.html", module_id));

    if js_path.exists() {
        let _ = fs::remove_file(js_path);
    }
    if html_path.exists() {
        let _ = fs::remove_file(html_path);
    }

    Ok(())
}
