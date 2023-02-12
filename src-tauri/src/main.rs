#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod spotlight;

use spotlight::State;

fn main() {
    tauri::Builder::default()
        .manage(State::default())
        .invoke_handler(tauri::generate_handler![spotlight::init_spotlight_window])
        .setup(move |app| {
            // Set the app's activation poicy to Accessory does the following behaviours:
            // - Makes the windows of this app appear above full-screen windows of other apps.
            // - Prevents the app's icon from showing on the dock.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
