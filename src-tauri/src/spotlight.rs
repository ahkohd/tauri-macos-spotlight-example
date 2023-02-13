use std::{ffi::c_void, ops::Deref, sync::Once};

use cocoa::{
    appkit::{
        CGFloat, NSApplicationActivationOptions, NSMainMenuWindowLevel, NSWindow,
        NSWindowCollectionBehavior,
    },
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSPoint, NSRect},
};
use core_foundation::{
    base::{CFRelease, FromVoid},
    dictionary::CFDictionary,
    number::{kCFNumberIntType, CFNumberGetValue, CFNumberRef},
};
use core_graphics::{
    display::{
        kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
        CFArrayGetCount, CFArrayGetValueAtIndex, CFDictionaryGetValueIfPresent, CFDictionaryRef,
        CGRect, CGWindowListCopyWindowInfo, CGWindowListOption,
    },
    window::{kCGWindowBounds, kCGWindowOwnerPID},
};
use objc::{class, msg_send, sel, sel_impl};
use tauri::{
    GlobalShortcutManager, Manager, PhysicalPosition, PhysicalSize, Window, WindowEvent, Wry,
};

#[allow(non_camel_case_types)]
type pid_t = i32;

#[macro_export]
macro_rules! nsstring_to_string {
    ($ns_string:expr) => {{
        use objc::{sel, sel_impl};
        let utf8: id = unsafe { objc::msg_send![$ns_string, UTF8String] };
        let string = if !utf8.is_null() {
            Some(unsafe {
                {
                    std::ffi::CStr::from_ptr(utf8 as *const std::ffi::c_char)
                        .to_string_lossy()
                        .into_owned()
                }
            })
        } else {
            None
        };

        string
    }};
}

fn cgnumber_to<T: Default>(number: *const c_void) -> Result<T, ()> {
    let mut value: T = T::default();
    if unsafe {
        CFNumberGetValue(
            number as CFNumberRef,
            kCFNumberIntType,
            (&mut value) as *mut _ as *mut c_void,
        )
    } {
        return Ok(value);
    }
    Err(())
}

static INIT: Once = Once::new();

#[tauri::command]
pub fn init_spotlight_window(window: Window<Wry>) {
    INIT.call_once(|| {
        register_shortcut(&window);
        register_spotlight_window_backdrop(&window);
        set_spotlight_window_collection_behaviour(&window);
        set_window_level(&window);
        window.set_focus().unwrap();
    });
}

fn register_shortcut(window: &Window<Wry>) {
    let window = window.to_owned();
    let mut shortcut_manager = window.app_handle().global_shortcut_manager();

    shortcut_manager
        .register("Cmd+k", move || {
            position_window_at_the_center_of_the_monitor_with_cursor(&window);

            if window.is_visible().unwrap() {
                refocus_window_behind_spotlight_window();
                window.hide().unwrap();
            } else {
                window.set_focus().unwrap();
            };
        })
        .unwrap();
}

fn register_spotlight_window_backdrop(window: &Window<Wry>) {
    let w = window.to_owned();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            refocus_window_behind_spotlight_window();
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
        let rect = NSRect {
            origin: NSPoint {
                x: (display_pos.x + (display_size.width / 2.0)) - (win_frame.size.width / 2.0),
                y: (display_pos.y + (display_size.height / 2.0)) - (win_frame.size.height / 2.0),
            },
            size: win_frame.size,
        };
        let _: () = unsafe { msg_send![handle, setFrame: rect display: YES] };
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

fn set_window_level(window: &Window<Wry>) {
    let handle: id = window.ns_window().unwrap() as _;
    unsafe { handle.setLevel_((NSMainMenuWindowLevel).into()) };
}

struct Monitor {
    #[allow(dead_code)]
    pub name: Option<String>,
    pub size: PhysicalSize<u32>,
    pub position: PhysicalPosition<i32>,
    pub scale_factor: f64,
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
            let name: id = unsafe { msg_send![next_screen, localizedName] };
            let screen_name = nsstring_to_string!(name);
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

/// Try to restore focus to the window behind the spotlight window
fn refocus_window_behind_spotlight_window() {
    if let Ok(owner_id) = get_window_behind_owner_id() {
        let running_app: id = unsafe {
            msg_send![
                class!(NSRunningApplication),
                runningApplicationWithProcessIdentifier: owner_id
            ]
        };

        let _: () = unsafe {
            msg_send![
                running_app,
                activateWithOptions:
                    NSApplicationActivationOptions::NSApplicationActivateIgnoringOtherApps
            ]
        };
    }
}

#[derive(Debug)]
pub enum Error {
    CouldNotGetWindowsList,
    NoWindowBehind,
}

/// Gets the owner id of the window behind the spotlight window
fn get_window_behind_owner_id() -> Result<pid_t, Error> {
    let process_info: id = unsafe { msg_send![class!(NSProcessInfo), processInfo] };
    let pid: pid_t = unsafe { msg_send![process_info, processIdentifier] };

    let window_list_options: CGWindowListOption =
        kCGWindowListExcludeDesktopElements | kCGWindowListOptionOnScreenOnly;
    let windows = unsafe { CGWindowListCopyWindowInfo(window_list_options, kCGNullWindowID) };

    if windows.is_null() {
        return Err(Error::CouldNotGetWindowsList);
    }

    let count = unsafe { CFArrayGetCount(windows) };
    let menubar_height = get_menubar_heights().iter().max().unwrap().to_owned();
    let mut found_spotlight_window = false;

    for i in 0..count {
        let window = unsafe { CFArrayGetValueAtIndex(windows, i) as CFDictionaryRef };
        if window.is_null() {
            continue;
        }

        let mut owner_pid: *const c_void = std::ptr::null();
        if unsafe {
            CFDictionaryGetValueIfPresent(window, kCGWindowOwnerPID as *mut c_void, &mut owner_pid)
        } == 0
        {
            continue;
        }
        if owner_pid.is_null() {
            continue;
        }
        let owner_pid = match cgnumber_to::<i32>(owner_pid) {
            Ok(num) => num,
            Err(_) => continue,
        } as pid_t;

        if !found_spotlight_window {
            if owner_pid == pid {
                found_spotlight_window = true;
            }

            continue;
        }

        let mut window_bounds: *const c_void = std::ptr::null();
        if unsafe {
            CFDictionaryGetValueIfPresent(
                window,
                kCGWindowBounds as *mut c_void,
                &mut window_bounds,
            )
        } == 0
        {
            continue;
        }
        if window_bounds.is_null() {
            continue;
        }
        let window_bounds = unsafe { CFDictionary::from_void(window_bounds) };
        let rect = match CGRect::from_dict_representation(window_bounds.deref()) {
            None => {
                continue;
            }
            Some(rect) => rect,
        };

        let is_menubar_window = menubar_height as f32 >= rect.size.height as f32;
        if !is_menubar_window {
            unsafe {
                CFRelease(windows.cast());
            };

            return Ok(owner_pid);
        }
    }

    unsafe {
        CFRelease(windows.cast());
    };

    Err(Error::NoWindowBehind)
}

/// Returns a list of the height of the menubar on available screens
pub fn get_menubar_heights() -> Vec<i32> {
    let mut result: Vec<i32> = vec![];

    objc::rc::autoreleasepool(|| {
        let screens: id = unsafe { msg_send![class!(NSScreen), screens] };
        let screens_iter: id = unsafe { msg_send![screens, objectEnumerator] };
        let mut next_screen: id;

        loop {
            next_screen = unsafe { msg_send![screens_iter, nextObject] };
            if next_screen == nil {
                break;
            }

            let frame: NSRect = unsafe { msg_send![next_screen, frame] };
            let visible_frame: NSRect = unsafe { msg_send![next_screen, visibleFrame] };
            let menubar_height = frame.size.height
                - visible_frame.size.height
                - (visible_frame.origin.y - frame.origin.y)
                - 1.0;

            result.push(menubar_height as i32)
        }
    });

    result
}
