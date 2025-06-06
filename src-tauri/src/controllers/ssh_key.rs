use tauri::AppHandle;
use std::fs;
use rusqlite::params;
use crate::database::connection::get;
use crate::models::SshKey;

#[tauri::command]
pub fn add_ssh_key(app_handle: AppHandle, name: String, path: String, is_default: bool) -> Result<i64, String> {
    let conn = get(&app_handle)?;

    // Use current time for timestamps
    let now = chrono::Local::now().to_rfc3339();

    // If this key is default, unset any existing default
    if is_default {
        conn.execute(
            "UPDATE ssh_keys SET is_default = 0 WHERE is_default = 1",
            [],
        ).map_err(|e| e.to_string())?;
    }

    conn.execute(
        "INSERT INTO ssh_keys (name, path, is_default, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, path, is_default, now, now],
    ).map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_ssh_key(app_handle: AppHandle, id: i64) -> Result<SshKey, String> {
    let conn = get(&app_handle)?;

    conn.query_row(
        "SELECT id, name, path, is_default, created_at, updated_at
         FROM ssh_keys WHERE id = ?1",
        params![id],
        |row| Ok(SshKey {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            path: row.get(2)?,
            is_default: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ssh_keys(app_handle: AppHandle) -> Result<Vec<SshKey>, String> {
    let conn = get(&app_handle)?;

    let mut stmt = conn.prepare("
        SELECT id, name, path, is_default, created_at, updated_at
        FROM ssh_keys
    ").map_err(|e| e.to_string())?;

    let key_iter = stmt.query_map([], |row| {
        Ok(SshKey {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            path: row.get(2)?,
            is_default: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut keys = Vec::new();
    for key in key_iter {
        keys.push(key.map_err(|e| e.to_string())?);
    }

    Ok(keys)
}

#[tauri::command]
pub fn set_default_ssh_key(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let conn = get(&app_handle)?;

    // First, unset any existing default
    conn.execute(
        "UPDATE ssh_keys SET is_default = 0 WHERE is_default = 1",
        [],
    ).map_err(|e| e.to_string())?;

    // Then set the new default
    conn.execute(
        "UPDATE ssh_keys SET is_default = 1 WHERE id = ?1",
        params![id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_ssh_key(app_handle: AppHandle, id: i64, delete_file: bool) -> Result<(), String> {
    let conn = get(&app_handle)?;

    // Get the file path before deleting the record
    let path = if delete_file {
        Some(conn.query_row(
            "SELECT path FROM ssh_keys WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0)
        ).map_err(|e| e.to_string())?)
    } else {
        None
    };

    // Delete the record
    conn.execute(
        "DELETE FROM ssh_keys WHERE id = ?1",
        params![id],
    ).map_err(|e| e.to_string())?;

    // Delete the file if requested
    if let Some(file_path) = path {
        if delete_file {
            if let Err(e) = fs::remove_file(&file_path) {
                return Err(format!("Failed to delete key file: {}", e));
            }
            // Try to remove the .pub file as well
            let pub_path = format!("{}.pub", file_path);
            let _ = fs::remove_file(pub_path); // It's okay if this fails
        }
    }

    Ok(())
}

#[tauri::command]
pub fn generate_ssh_key(app_handle: AppHandle, name: String) -> Result<String, String> {
    // Get the user's home directory
    let home_dir = dirs::home_dir().ok_or("Could not get home directory")?;
    let ssh_dir = home_dir.join(".ssh");

    // Create .ssh directory if it doesn't exist
    if !ssh_dir.exists() {
        fs::create_dir_all(&ssh_dir).map_err(|e| e.to_string())?;
        // Set appropriate permissions (unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&ssh_dir, permissions).map_err(|e| e.to_string())?;
        }
    }

    let key_path = ssh_dir.join(format!("{}", name));
    let key_path_str = key_path.to_str().ok_or("Invalid path")?.to_string();

    // Generate key using ssh-keygen via Command
    use std::process::Command;

    let output = Command::new("ssh-keygen")
        .arg("-t")
        .arg("ed25519")
        .arg("-f")
        .arg(&key_path)
        .arg("-N")  // Empty passphrase
        .arg("")
        .output()
        .map_err(|e| format!("Failed to execute ssh-keygen: {}", e))?;

    if !output.status.success() {
        return Err(format!("ssh-keygen failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Add to database
    add_ssh_key(app_handle, name.clone(), key_path_str.clone(), false)?;

    Ok(key_path_str)
}