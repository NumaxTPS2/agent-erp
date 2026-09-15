use crate::get_db_path;
use rusqlite::params;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1000);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: String, // "pending" | "in_progress" | "done" | "cancelled"
    pub parent_task_id: Option<String>,
    pub module_id: String,
    pub assignee: String,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, PartialEq, Eq)]
pub struct TaskMessage {
    pub id: String,
    pub task_id: String,
    pub role: String, // "user" | "assistant"
    pub content: String,
    pub timestamp: i64,
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn create_tasks_tables(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            parent_task_id TEXT,
            module_id TEXT NOT NULL,
            assignee TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            completed_at INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_messages (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;

    Ok(())
}

// Usecase: create_task
pub fn create_task_impl(
    conn: &rusqlite::Connection,
    title: String,
    module_id: String,
    assignee: String,
    parent_task_id: Option<String>,
) -> Result<Task, String> {
    let trimmed_title = title.trim();
    if trimmed_title.is_empty() {
        return Err("Task title cannot be empty".to_string());
    }
    let trimmed_module_id = module_id.trim();
    if trimmed_module_id.is_empty() {
        return Err("Module ID cannot be empty".to_string());
    }
    let trimmed_assignee = assignee.trim();
    if trimmed_assignee.is_empty() {
        return Err("Assignee cannot be empty".to_string());
    }

    let now = current_timestamp();
    let id = format!(
        "task_{}_{}",
        now,
        ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let status = "pending".to_string();

    conn.execute(
        "INSERT INTO tasks (id, title, status, parent_task_id, module_id, assignee, created_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            trimmed_title,
            status,
            parent_task_id,
            trimmed_module_id,
            trimmed_assignee,
            now,
            Option::<i64>::None,
        ],
    )
    .map_err(|e| format!("Failed to insert task: {}", e))?;

    Ok(Task {
        id,
        title: trimmed_title.to_string(),
        status,
        parent_task_id,
        module_id: trimmed_module_id.to_string(),
        assignee: trimmed_assignee.to_string(),
        created_at: now,
        completed_at: None,
    })
}

// Usecase: list_tasks
pub fn list_tasks_impl(conn: &rusqlite::Connection, module_id: &str) -> Result<Vec<Task>, String> {
    let trimmed_module = module_id.trim();
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            status: row.get(2)?,
            parent_task_id: row.get(3)?,
            module_id: row.get(4)?,
            assignee: row.get(5)?,
            created_at: row.get(6)?,
            completed_at: row.get(7)?,
        })
    };

    let mut tasks = Vec::new();
    if trimmed_module.is_empty() {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, status, parent_task_id, module_id, assignee, created_at, completed_at
                 FROM tasks ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Failed to prepare list_tasks query: {}", e))?;
        let rows = stmt
            .query_map([], map_row)
            .map_err(|e| format!("Failed to query tasks: {}", e))?;
        for task_res in rows {
            tasks.push(task_res.map_err(|e| format!("Error reading task row: {}", e))?);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, status, parent_task_id, module_id, assignee, created_at, completed_at
                 FROM tasks WHERE module_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Failed to prepare list_tasks query: {}", e))?;
        let rows = stmt
            .query_map(params![trimmed_module], map_row)
            .map_err(|e| format!("Failed to query tasks: {}", e))?;
        for task_res in rows {
            tasks.push(task_res.map_err(|e| format!("Error reading task row: {}", e))?);
        }
    }

    Ok(tasks)
}

// Usecase: update_task_status
pub fn update_task_status_impl(
    conn: &rusqlite::Connection,
    task_id: &str,
    status: &str,
) -> Result<(), String> {
    let valid_statuses = ["pending", "in_progress", "done", "cancelled"];
    if !valid_statuses.contains(&status) {
        return Err(format!(
            "Invalid task status: '{}'. Must be one of: pending, in_progress, done, cancelled",
            status
        ));
    }

    let now = current_timestamp();
    let completed_at = if status == "done" { Some(now) } else { None };

    let rows_affected = conn
        .execute(
            "UPDATE tasks SET status = ?1, completed_at = ?2 WHERE id = ?3",
            params![status, completed_at, task_id],
        )
        .map_err(|e| format!("Failed to update task status: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Task not found: {}", task_id));
    }

    Ok(())
}

// Usecase: append_task_message
pub fn append_task_message_impl(
    conn: &rusqlite::Connection,
    task_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let trimmed_task_id = task_id.trim();
    if trimmed_task_id.is_empty() {
        return Err("Task ID cannot be empty".to_string());
    }

    if role != "user" && role != "assistant" {
        return Err("Invalid role: must be user or assistant".to_string());
    }

    let trimmed_content = content.trim();
    if trimmed_content.is_empty() {
        return Err("Message content cannot be empty".to_string());
    }

    let now = current_timestamp();
    let id = format!("msg_{}_{}", now, ID_COUNTER.fetch_add(1, Ordering::Relaxed));

    conn.execute(
        "INSERT INTO task_messages (id, task_id, role, content, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, trimmed_task_id, role, trimmed_content, now],
    )
    .map_err(|e| format!("Failed to insert task message: {}", e))?;

    Ok(())
}

// Usecase: get_task_messages
pub fn get_task_messages_impl(
    conn: &rusqlite::Connection,
    task_id: &str,
) -> Result<Vec<TaskMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, role, content, timestamp
             FROM task_messages WHERE task_id = ?1 ORDER BY timestamp ASC, id ASC",
        )
        .map_err(|e| format!("Failed to prepare get_task_messages query: {}", e))?;

    let rows = stmt
        .query_map(params![task_id.trim()], |row| {
            Ok(TaskMessage {
                id: row.get(0)?,
                task_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query task messages: {}", e))?;

    let mut messages = Vec::new();
    for msg_res in rows {
        messages.push(msg_res.map_err(|e| format!("Error reading task message row: {}", e))?);
    }

    Ok(messages)
}

// Delivery layer: Tauri Commands
#[tauri::command]
#[specta::specta]
pub async fn create_task<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    title: String,
    module_id: String,
    assignee: String,
    parent_task_id: Option<String>,
) -> Result<Task, String> {
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    create_task_impl(&conn, title, module_id, assignee, parent_task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn list_tasks<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    module_id: String,
) -> Result<Vec<Task>, String> {
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    list_tasks_impl(&conn, &module_id)
}

#[tauri::command]
#[specta::specta]
pub async fn update_task_status<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    task_id: String,
    status: String,
) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    update_task_status_impl(&conn, &task_id, &status)
}

#[tauri::command]
#[specta::specta]
pub async fn append_task_message<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    task_id: String,
    role: String,
    content: String,
) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    append_task_message_impl(&conn, &task_id, &role, &content)
}

#[tauri::command]
#[specta::specta]
pub async fn get_task_messages<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    task_id: String,
) -> Result<Vec<TaskMessage>, String> {
    let db_path = get_db_path(&app_handle);
    let conn =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    get_task_messages_impl(&conn, &task_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tasks_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn test_tc_task_01_create_top_level_task_success() {
        // Given: An initialized database and valid top-level task parameters
        let conn = setup_test_db();
        let title = "新租戶起步".to_string();
        let module_id = "sales".to_string();
        let assignee = "主管".to_string();
        let parent_task_id = None;

        // When: create_task_impl is executed
        let result = create_task_impl(
            &conn,
            title.clone(),
            module_id.clone(),
            assignee.clone(),
            parent_task_id,
        );

        // Then: The task is successfully created with status "pending" and no parent
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.title, title);
        assert_eq!(task.module_id, module_id);
        assert_eq!(task.assignee, assignee);
        assert_eq!(task.status, "pending");
        assert_eq!(task.parent_task_id, None);
        assert_eq!(task.completed_at, None);
        assert!(task.created_at > 0);
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_tc_task_02_create_subtask_success() {
        // Given: An initialized database and an existing parent task
        let conn = setup_test_db();
        let parent = create_task_impl(
            &conn,
            "新租戶起步".to_string(),
            "sales".to_string(),
            "主管".to_string(),
            None,
        )
        .unwrap();

        // When: create_task_impl is executed with parent_task_id
        let subtask = create_task_impl(
            &conn,
            "設定部門".to_string(),
            "sales".to_string(),
            "主管".to_string(),
            Some(parent.id.clone()),
        )
        .unwrap();

        // Then: The subtask is created and links to parent_task_id
        assert_eq!(subtask.title, "設定部門");
        assert_eq!(subtask.parent_task_id, Some(parent.id));
        assert_eq!(subtask.status, "pending");
    }

    #[test]
    fn test_tc_task_03_create_task_empty_title_error() {
        // Given: An initialized database and an empty title
        let conn = setup_test_db();

        // When: create_task_impl is called with whitespace title
        let result = create_task_impl(
            &conn,
            "   ".to_string(),
            "sales".to_string(),
            "主管".to_string(),
            None,
        );

        // Then: It returns validation error
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Task title cannot be empty");
    }

    #[test]
    fn test_tc_task_04_create_task_empty_module_or_assignee_error() {
        // Given: An initialized database and empty module or assignee
        let conn = setup_test_db();

        // When: create_task_impl is called with empty module
        let err_module = create_task_impl(
            &conn,
            "任務".to_string(),
            "".to_string(),
            "主管".to_string(),
            None,
        );
        // And: create_task_impl is called with empty assignee
        let err_assignee = create_task_impl(
            &conn,
            "任務".to_string(),
            "sales".to_string(),
            "  ".to_string(),
            None,
        );

        // Then: Both return corresponding validation errors
        assert_eq!(err_module.unwrap_err(), "Module ID cannot be empty");
        assert_eq!(err_assignee.unwrap_err(), "Assignee cannot be empty");
    }

    #[test]
    fn test_tc_task_05_and_06_list_tasks_by_module() {
        // Given: Tasks in "sales" and "crm" modules
        let conn = setup_test_db();
        create_task_impl(&conn, "Sales 1".into(), "sales".into(), "User".into(), None).unwrap();
        create_task_impl(&conn, "Sales 2".into(), "sales".into(), "User".into(), None).unwrap();
        create_task_impl(&conn, "CRM 1".into(), "crm".into(), "User".into(), None).unwrap();

        // When: Querying "sales" module
        let sales_tasks = list_tasks_impl(&conn, "sales").unwrap();
        // And: Querying "finance" module (no tasks)
        let finance_tasks = list_tasks_impl(&conn, "finance").unwrap();

        // Then: Exactly 2 sales tasks are returned and 0 finance tasks are returned
        assert_eq!(sales_tasks.len(), 2);
        assert_eq!(sales_tasks[0].title, "Sales 1");
        assert_eq!(sales_tasks[1].title, "Sales 2");
        assert_eq!(finance_tasks.len(), 0);
    }

    #[test]
    fn test_tc_task_07_list_tasks_all_modules() {
        // Given: Tasks across different modules
        let conn = setup_test_db();
        create_task_impl(&conn, "Task A".into(), "sales".into(), "User".into(), None).unwrap();
        create_task_impl(&conn, "Task B".into(), "crm".into(), "User".into(), None).unwrap();

        // When: Querying with empty module_id
        let all_tasks = list_tasks_impl(&conn, "").unwrap();

        // Then: All tasks across all modules are returned
        assert_eq!(all_tasks.len(), 2);
    }

    #[test]
    fn test_tc_task_08_update_task_status_transitions() {
        // Given: A pending task
        let conn = setup_test_db();
        let task = create_task_impl(
            &conn,
            "設定部門".into(),
            "sales".into(),
            "User".into(),
            None,
        )
        .unwrap();

        // When: Updating to in_progress
        update_task_status_impl(&conn, &task.id, "in_progress").unwrap();
        let list = list_tasks_impl(&conn, "sales").unwrap();
        assert_eq!(list[0].status, "in_progress");
        assert_eq!(list[0].completed_at, None);

        // When: Updating to done
        update_task_status_impl(&conn, &task.id, "done").unwrap();
        let list_done = list_tasks_impl(&conn, "sales").unwrap();

        // Then: Status is done and completed_at is recorded
        assert_eq!(list_done[0].status, "done");
        assert!(list_done[0].completed_at.is_some());

        // When: Updating to cancelled
        update_task_status_impl(&conn, &task.id, "cancelled").unwrap();
        let list_cancelled = list_tasks_impl(&conn, "sales").unwrap();
        assert_eq!(list_cancelled[0].status, "cancelled");
        assert_eq!(list_cancelled[0].completed_at, None);
    }

    #[test]
    fn test_tc_task_09_update_task_invalid_status_error() {
        // Given: A pending task
        let conn = setup_test_db();
        let task = create_task_impl(
            &conn,
            "設定部門".into(),
            "sales".into(),
            "User".into(),
            None,
        )
        .unwrap();

        // When: Updating to invalid status "failed"
        let result = update_task_status_impl(&conn, &task.id, "failed");

        // Then: Error is returned and status remains pending
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid task status"));
        let list = list_tasks_impl(&conn, "sales").unwrap();
        assert_eq!(list[0].status, "pending");
    }

    #[test]
    fn test_tc_task_10_update_nonexistent_task_not_found_error() {
        // Given: An initialized database without task "task_9999"
        let conn = setup_test_db();

        // When: Updating nonexistent task
        let result = update_task_status_impl(&conn, "task_9999", "done");

        // Then: Error indicating Task not found is returned
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Task not found: task_9999");
    }

    #[test]
    fn test_tc_task_11_and_12_and_15_append_and_get_task_messages() {
        // Given: A task and main agent context
        let conn = setup_test_db();
        let task = create_task_impl(
            &conn,
            "設定部門".into(),
            "sales".into(),
            "User".into(),
            None,
        )
        .unwrap();

        // When: Appending user and assistant messages for task and main
        append_task_message_impl(&conn, &task.id, "user", "幫我新增一個銷售部").unwrap();
        append_task_message_impl(&conn, &task.id, "assistant", "已為您成功建立銷售部！").unwrap();
        append_task_message_impl(
            &conn,
            "main",
            "assistant",
            "✅「設定部門」已完成，新增了『銷售部』",
        )
        .unwrap();

        // Then: get_task_messages for task returns 2 messages in chronological order
        let task_msgs = get_task_messages_impl(&conn, &task.id).unwrap();
        assert_eq!(task_msgs.len(), 2);
        assert_eq!(task_msgs[0].role, "user");
        assert_eq!(task_msgs[0].content, "幫我新增一個銷售部");
        assert_eq!(task_msgs[1].role, "assistant");
        assert_eq!(task_msgs[1].content, "已為您成功建立銷售部！");

        // And: get_task_messages for main returns the summary message
        let main_msgs = get_task_messages_impl(&conn, "main").unwrap();
        assert_eq!(main_msgs.len(), 1);
        assert_eq!(
            main_msgs[0].content,
            "✅「設定部門」已完成，新增了『銷售部』"
        );
    }

    #[test]
    fn test_tc_task_13_append_message_empty_content_or_task_id_error() {
        // Given: An initialized database
        let conn = setup_test_db();

        // When: Appending with empty task_id
        let err_id = append_task_message_impl(&conn, "   ", "user", "Hello");
        // And: Appending with empty content
        let err_content = append_task_message_impl(&conn, "task_1", "user", "   ");

        // Then: Validation errors are returned
        assert_eq!(err_id.unwrap_err(), "Task ID cannot be empty");
        assert_eq!(err_content.unwrap_err(), "Message content cannot be empty");
    }

    #[test]
    fn test_tc_task_14_append_message_invalid_role_error() {
        // Given: An initialized database
        let conn = setup_test_db();

        // When: Appending with invalid role "system"
        let result = append_task_message_impl(&conn, "task_1", "system", "Hello");

        // Then: Error is returned
        assert_eq!(
            result.unwrap_err(),
            "Invalid role: must be user or assistant"
        );
    }

    #[test]
    fn test_tc_task_16_get_messages_nonexistent_task_empty_vec() {
        // Given: An initialized database with no messages for "task_none"
        let conn = setup_test_db();

        // When: get_task_messages is queried for "task_none"
        let msgs = get_task_messages_impl(&conn, "task_none").unwrap();

        // Then: An empty list is returned without error
        assert_eq!(msgs.len(), 0);
    }
}
