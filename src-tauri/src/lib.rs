mod database;
mod models;
mod controllers;

use tauri::AppHandle;
use database::connection as conn;

#[tauri::command]
fn init_app(app_handle: AppHandle) -> Result<String, String> {
    let handler = app_handle.clone();

    let db_result = conn::init_database(app_handle)?;

    println!("Tauri SQLite Database Initialization Successful!");

    controllers::setting::init_default_settings(handler)?;

    println!("Tauri Settings Initialization Successful!");

    Ok(db_result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            init_app,

            controllers::server::add_server,
            controllers::server::get_server,
            controllers::server::update_server,
            controllers::server::delete_server,
            controllers::server::get_servers,

            controllers::ssh_key::add_ssh_key,
            controllers::ssh_key::get_ssh_key,
            controllers::ssh_key::set_default_ssh_key,
            controllers::ssh_key::generate_ssh_key,

            controllers::setting::get_setting,
            controllers::setting::get_settings,
            controllers::setting::update_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
