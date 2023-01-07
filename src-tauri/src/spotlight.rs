use std::ffi::{c_char, CStr};

use cocoa::{
    appkit::{CGFloat, NSMainMenuWindowLevel, NSWindow, NSWindowCollectionBehavior},
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSPoint, NSRect},
};
use objc::{class, msg_send, sel, sel_impl};
use tauri::{
    GlobalShortcutManager, LogicalPosition, Manager, PhysicalPosition, PhysicalSize, Window,
    WindowEvent, Wry,
};

#[tauri::command]
pub fn init_spotlight_window(window: Window<Wry>) {
    register_shortcut(&window);
    register_spotlight_window_backdrop(&window);
    set_spotlight_window_collection_behaviour(&window);
    set_window_above_menubar(&window);
    window.set_focus().unwrap();
}

fn register_shortcut(window: &Window<Wry>) {
    let window = window.to_owned();
    let mut shortcut_manager = window.app_handle().global_shortcut_manager();
    shortcut_manager.register("Cmd+k", move || {
        position_window_at_the_center_of_the_monitor_with_cursor(&window);

        if window.is_visible().unwrap() {
            window.hide().unwrap();
        } else {
            window.set_focus().unwrap();
        }
    });
}

fn register_spotlight_window_backdrop(window: &Window<Wry>) {
    let w = window.to_owned();

    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            w.hide().unwrap();
        }
    });
}

/// Positions a given window at the center of the monitor with cursor
fn position_window_at_the_center_of_the_monitor_with_cursor(window: &Window<Wry>) {
    if let Some(monitor) = get_monitor_with_cursor() {
        let display_size = monitor.size.to_logical::<f64>(monitor.scale_factor);
        let display_pos = monitor.position.to_logical::<f64>(monitor.scale_factor);

        let handle: id = window.ns_window().unwrap() as _;
        let win_frame: NSRect = unsafe { handle.frame() };

        window
            .set_position(LogicalPosition {
                x: (display_pos.x + (display_size.width / 2.0)) - (win_frame.size.width / 2.0),
                y: (display_pos.y + (display_size.height / 2.0)) - (win_frame.size.height / 2.0),
            })
            .unwrap();
    }
}

/// Set the behaviours that makes the window appear on all worksapces
fn set_spotlight_window_collection_behaviour(window: &Window<Wry>) {
    let handle: id = window.ns_window().unwrap() as _;
    unsafe {
        handle.setCollectionBehavior_(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenPrimary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle,
        );
    };
}

/// Set the window above menubar level
fn set_window_above_menubar(window: &Window<Wry>) {
    let handle: id = window.ns_window().unwrap() as _;
    unsafe { handle.setLevel_((NSMainMenuWindowLevel + 2).into()) };
}

struct Monitor {
    pub name: Option<String>,
    pub size: PhysicalSize<u32>,
    pub position: PhysicalPosition<i32>,
    pub scale_factor: f64,
}

impl Monitor {
    /// Returns the monitor's resolution.
    pub fn size(&self) -> &PhysicalSize<u32> {
        &self.size
    }

    /// Returns the top-left corner position of the monitor relative to the larger full screen area.
    pub fn position(&self) -> &PhysicalPosition<i32> {
        &self.position
    }

    /// Returns the scale factor that can be used to map logical pixels to physical pixels, and vice versa.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }
}

#[link(name = "Foundation", kind = "framework")]
extern "C" {
    pub fn NSMouseInRect(aPoint: NSPoint, aRect: NSRect, flipped: BOOL) -> BOOL;
}

/// Returns the Monitor with cursor
fn get_monitor_with_cursor() -> Option<Monitor> {
    objc::rc::autoreleasepool(|| {
        let mouse_location: NSPoint = unsafe { msg_send![class!(NSEvent), mouseLocation] };
        let screens: id = unsafe { msg_send![class!(NSScreen), screens] };
        let screens_iter: id = unsafe { msg_send![screens, objectEnumerator] };
        let mut next_screen: id;

        let frame_with_cursor: Option<NSRect> = loop {
            next_screen = unsafe { msg_send![screens_iter, nextObject] };
            if next_screen == nil {
                break None;
            }

            let frame: NSRect = unsafe { msg_send![next_screen, frame] };
            let is_mouse_in_screen_frame: BOOL =
                unsafe { NSMouseInRect(mouse_location, frame, NO) };
            if is_mouse_in_screen_frame == YES {
                break Some(frame);
            }
        };

        if let Some(frame) = frame_with_cursor {
            let screen_name = nsstring_to_string(unsafe { msg_send![next_screen, localizedName] });
            let scale_factor: CGFloat = unsafe { msg_send![next_screen, backingScaleFactor] };
            let scale_factor: f64 = scale_factor;

            return Some(Monitor {
                name: screen_name,
                position: PhysicalPosition {
                    x: (frame.origin.x * scale_factor) as i32,
                    y: (frame.origin.y * scale_factor) as i32,
                },
                size: PhysicalSize {
                    width: (frame.size.width * scale_factor) as u32,
                    height: (frame.size.height * scale_factor) as u32,
                },
                scale_factor,
            });
        }

        None
    })
}

/// Converts NSString to Rust String
fn nsstring_to_string(ns_string: id) -> Option<String> {
    let utf8: id = unsafe { msg_send![ns_string, UTF8String] };
    if !utf8.is_null() {
        Some(unsafe {
            {
                CStr::from_ptr(utf8 as *const c_char)
                    .to_string_lossy()
                    .into_owned()
            }
        })
    } else {
        None
    }
}
