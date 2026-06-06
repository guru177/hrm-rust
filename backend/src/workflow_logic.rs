//! Minimal workflow execution engine — fires on HR events (leave approved, etc.).

pub fn trigger(conn: &rusqlite::Connection, trigger_type: &str, context: &serde_json::Value) {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let context_str = context.to_string();

    let mut stmt = match conn.prepare(
        "SELECT id, name, actions FROM workflows WHERE is_active = 1 AND trigger_type = ?1",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let workflows: Vec<(i64, String, Option<String>)> = stmt
        .query_map([trigger_type], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    for (workflow_id, name, actions_json) in workflows {
        let _ = conn.execute(
            "INSERT INTO workflow_executions (workflow_id, status, trigger_type, created_at, updated_at)
             VALUES (?1, 'completed', ?2, ?3, ?3)",
            rusqlite::params![workflow_id, trigger_type, &now],
        );
        let _ = conn.execute(
            "UPDATE workflows SET execution_count = COALESCE(execution_count, 0) + 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![&now, workflow_id],
        );

        if let Some(ref actions_str) = actions_json {
            if let Ok(actions) = serde_json::from_str::<serde_json::Value>(actions_str) {
                execute_actions(conn, &name, &actions, context, &now);
            }
        }

        log::info!("Workflow '{}' (id={}) executed for trigger '{}'", name, workflow_id, trigger_type);
    }
}

fn execute_actions(
    conn: &rusqlite::Connection,
    workflow_name: &str,
    actions: &serde_json::Value,
    context: &serde_json::Value,
    now: &str,
) {
    let items = match actions {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(_) => vec![actions.clone()],
        _ => return,
    };

    for action in items {
        let action_type = action
            .get("type")
            .or_else(|| action.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match action_type {
            "create_task" | "task" => {
                let title = action
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Workflow task");
                let assignee = context
                    .get("user_id")
                    .and_then(|v| v.as_i64())
                    .or_else(|| action.get("assigned_to").and_then(|v| v.as_i64()));
                let _ = conn.execute(
                    "INSERT INTO tasks (title, description, status, priority, assigned_to, created_by, created_at, updated_at)
                     VALUES (?1, ?2, 'todo', 'medium', ?3, 1, ?4, ?4)",
                    rusqlite::params![
                        format!("{}: {}", workflow_name, title),
                        format!("Auto-created by workflow. Context: {}", context),
                        assignee,
                        now,
                    ],
                );
            }
            "notify" | "log" => {
                log::info!("Workflow notify: {} — {:?}", workflow_name, action);
            }
            _ => {
                log::info!("Workflow action '{}' in '{}' — context: {}", action_type, workflow_name, context);
            }
        }
    }
}
