#![allow(clippy::unused_unit)]

use tauri::{LogicalPosition, Runtime, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, PanelHandle, PanelLevel, StyleMask,
    WebviewWindowExt as WebviewPanelExt,
};
use thiserror::Error;

tauri_panel! {
    panel!(SpotlightPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true,
        }
    })

    panel_event!(SpotlightPanelEventHandler {
        window_did_become_key(notification: &NSNotification) -> (),
        window_did_resign_key(notification: &NSNotification) -> (),
    })
}

type TauriError = tauri::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("Unable to convert window to panel")]
    Panel,
    #[error("Monitor with cursor not found")]
    MonitorNotFound,
}

pub trait WebviewWindowExt<R: Runtime> {
    fn to_spotlight_panel(&self) -> tauri::Result<PanelHandle<R>>;

    fn center_at_cursor_monitor(&self) -> tauri::Result<()>;
}

impl<R: Runtime> WebviewWindowExt<R> for WebviewWindow<R> {
    fn to_spotlight_panel(&self) -> tauri::Result<PanelHandle<R>> {
        // Convert window to panel
        let panel = self
            .to_panel::<SpotlightPanel<R>>()
            .map_err(|_| TauriError::Anyhow(Error::Panel.into()))?;

        let weak_panel = std::sync::Arc::downgrade(&panel);

        // Set panel level
        panel.set_level(PanelLevel::Floating.value());

        panel.set_collection_behavior(
            CollectionBehavior::new()
                // Makes panel appear alongside full screen apps
                .full_screen_auxiliary()
                // Panel follows active desktop space
                .move_to_active_space()
                .into(),
        );

        // Ensures the panel cannot activate the App
        panel
            .add_style_mask(StyleMask::empty().nonactivating_panel().into())
            .expect("failed to make spotlight panel non-activating");

        // Setup event handler for panel events
        let handler = SpotlightPanelEventHandler::new();

        handler.window_did_become_key(|_| {
            println!("panel became key window");
        });

        handler.window_did_resign_key(move |_| {
            println!("panel resign key window");

            // Hide panel when it resigns key window status
            if let Some(panel) = weak_panel.upgrade() {
                if panel.is_visible() {
                    panel.hide();
                }
            }
        });

        panel.set_event_handler(Some(handler.as_ref()));

        Ok(panel)
    }

    fn center_at_cursor_monitor(&self) -> tauri::Result<()> {
        // Tauri reports the cursor in physical pixels, while macOS monitor lookup
        // expects logical desktop coordinates. Convert using the primary display,
        // whose origin defines the macOS desktop coordinate space.
        let primary_monitor = self.primary_monitor()?;
        let desktop_scale_factor = primary_monitor
            .as_ref()
            .map(|monitor| monitor.scale_factor())
            .unwrap_or(1.0);
        let cursor_position = self
            .cursor_position()?
            .to_logical::<f64>(desktop_scale_factor);
        let monitor = self
            .monitor_from_point(cursor_position.x, cursor_position.y)?
            .or(self.current_monitor()?)
            .or(primary_monitor)
            .ok_or(TauriError::Anyhow(Error::MonitorNotFound.into()))?;

        let monitor_scale_factor = monitor.scale_factor();

        let monitor_size = monitor.size().to_logical::<f64>(monitor_scale_factor);

        let monitor_position = monitor.position().to_logical::<f64>(monitor_scale_factor);

        let window_size = self.outer_size()?.to_logical::<f64>(monitor_scale_factor);
        let x = monitor_position.x + (monitor_size.width - window_size.width) / 2.0;
        let y = monitor_position.y + (monitor_size.height - window_size.height) / 2.0;

        self.set_position(LogicalPosition::new(x, y))?;

        Ok(())
    }
}
