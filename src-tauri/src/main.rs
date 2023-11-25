#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod spotlight;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            spotlight::init_spotlight_window,
            spotlight::show_spotlight,
            spotlight::hide_spotlight,
            spotlight::will_open_file_picker,
            spotlight::did_close_file_picker
        ])
        .manage(spotlight::State::default())
        .setup(move |app| {
            // Set activation poicy to Accessory to prevent the app icon from showing on the dock
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
