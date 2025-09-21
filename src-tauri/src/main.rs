#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use tauri_nspanel::ManagerExt;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

use crate::window::WebviewWindowExt;

mod command;
mod window;

pub const SPOTLIGHT_LABEL: &str = "main";

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![command::show, command::hide])
        .plugin(tauri_nspanel::init())
        .setup(move |app| {
            // Set activation policy to Prohibited to prevent
            // app icon in dock and focus stealing on first launch
            //
            // Alternative: use Accessory to allow app activation
            // but hide from dock, it will steal focus on first launch
            app.set_activation_policy(tauri::ActivationPolicy::Prohibited);

            Ok(())
        })
        // Register a global shortcut (⌘+K) to toggle the visibility of the spotlight panel
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut(Shortcut::new(Some(Modifiers::SUPER), Code::KeyK))
                .unwrap()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed
                        && shortcut.matches(Modifiers::SUPER, Code::KeyK)
                    {
                        let window = app.get_webview_window(SPOTLIGHT_LABEL).unwrap();

                        match app
                            .get_webview_panel(SPOTLIGHT_LABEL)
                            .or_else(|_| window.to_spotlight_panel())
                        {
                            Ok(panel) => {
                                if panel.is_visible() {
                                    panel.hide();
                                } else {
                                    window.center_at_cursor_monitor().unwrap();
                                    panel.show_and_make_key();
                                }
                            }
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                })
                .build(),
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
