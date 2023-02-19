use std::{ffi::c_void, ops::Deref, sync::Once};

use cocoa::{
    appkit::{CGFloat, NSMainMenuWindowLevel, NSWindow, NSWindowCollectionBehavior},
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
        kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenBelowWindow,
        kCGWindowListOptionOnScreenOnly, CFArrayGetCount, CFArrayGetValueAtIndex,
        CFDictionaryGetValueIfPresent, CFDictionaryRef, CGRect, CGWindowID,
        CGWindowListCopyWindowInfo, CGWindowListOption,
    },
    window::{
        kCGWindowBounds, kCGWindowLayer, kCGWindowNumber, kCGWindowOwnerName, kCGWindowOwnerPID,
    },
};
use objc::{class, msg_send, sel, sel_impl};
use tauri::{
    GlobalShortcutManager, Manager, PhysicalPosition, PhysicalSize, Window, WindowEvent, Wry,
};

use crate::accessibility::{bring_window_to_top, focus_window, get_axuielements};

#[allow(non_camel_case_types)]
type pid_t = i32;
type CGWindowLevelKey = i32;
type CGWindowLevel = i32;

#[link(name = "Foundation", kind = "framework")]
extern "C" {
    pub fn NSMouseInRect(aPoint: NSPoint, aRect: NSRect, flipped: BOOL) -> BOOL;
    pub fn CGWindowLevelForKey(key: CGWindowLevelKey) -> CGWindowLevel;
}

#[allow(dead_code)]
enum _CGWindowLevelKey {
    BaseWindowLevelKey = 0,
    MinimumWindowLevelKey = 1,
    DesktopWindowLevelKey = 2,
    BackstopMenuLevelKey = 3,
    NormalWindowLevelKey = 4,
    FloatingWindowLevelKey = 5,
    TornOffMenuWindowLevelKey = 6,
    DockWindowLevelKey = 7,
    MainMenuWindowLevelKey = 8,
    StatusWindowLevelKey = 9,
    ModalPanelWindowLevelKey = 10,
    PopUpMenuWindowLevelKey = 11,
    DraggingWindowLevelKey = 12,
    ScreenSaverWindowLevelKey = 13,
    MaximumWindowLevelKey = 14,
    OverlayWindowLevelKey = 15,
    HelpWindowLevelKey = 16,
    UtilityWindowLevelKey = 17,
    DesktopIconWindowLevelKey = 18,
    NumberOfWindowLevelKeys = 19,
}

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
        set_above_main_window_level(&window);
        window.set_focus().unwrap();
    });
}

#[tauri::command]
pub fn hide_spotlight(window: Window<Wry>) {
    focus_window_behind(&window);
    window.hide().unwrap();
}

fn register_shortcut(window: &Window<Wry>) {
    let window = window.to_owned();
    let mut shortcut_manager = window.app_handle().global_shortcut_manager();

    shortcut_manager
        .register("Cmd+k", move || {
            position_window_at_the_center_of_the_monitor_with_cursor(&window);

            if window.is_visible().unwrap() {
                hide_spotlight(window.clone());
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

fn set_above_main_window_level(window: &Window<Wry>) {
    let handle: id = window.ns_window().unwrap() as _;
    unsafe { handle.setLevel_((NSMainMenuWindowLevel + 2).into()) };
}

struct Monitor {
    #[allow(dead_code)]
    pub name: Option<String>,
    pub size: PhysicalSize<u32>,
    pub position: PhysicalPosition<i32>,
    pub scale_factor: f64,
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

/// Try to restore focus to the window behind
fn focus_window_behind(window: &Window<Wry>) {
    if let Ok((owner_id, window_id)) = get_window_behind(window) {
        if let Ok((ax_app_ref, ax_window_ref)) =
            get_axuielements(owner_id, window_id, window.app_handle())
        {
            if bring_window_to_top(ax_app_ref, ax_window_ref).is_ok()
                && focus_window(ax_window_ref).is_ok()
            {}

            unsafe { CFRelease(ax_app_ref.cast()) };
        }
    }
}

#[derive(Debug)]
pub enum Error {
    CouldNotGetWindowsList,
    NoWindowBehind,
}

/// Gets a window behind the spotlight window
fn get_window_behind(window: &Window<Wry>) -> Result<(pid_t, u32), Error> {
    println!("info!(): Get the window behind the spotlight window.");

    let screen_size = {
        let monitor = get_monitor_with_cursor().unwrap();
        monitor.size.to_logical::<f64>(monitor.scale_factor)
    };
    let handle: id = window.ns_window().unwrap() as _;
    let window_number: CGWindowID = unsafe { msg_send![handle, windowNumber] };
    let window_list_options: CGWindowListOption = kCGWindowListExcludeDesktopElements
        | kCGWindowListOptionOnScreenOnly
        | kCGWindowListOptionOnScreenBelowWindow;
    let windows = unsafe { CGWindowListCopyWindowInfo(window_list_options, window_number) };

    if windows.is_null() {
        return Err(Error::CouldNotGetWindowsList);
    }

    let count = unsafe { CFArrayGetCount(windows) };
    for i in 0..count {
        let window = unsafe { CFArrayGetValueAtIndex(windows, i) as CFDictionaryRef };
        if window.is_null() {
            continue;
        }

        let mut owner_id: *const c_void = std::ptr::null();
        if unsafe {
            CFDictionaryGetValueIfPresent(window, kCGWindowOwnerPID as *mut c_void, &mut owner_id)
        } == 0
        {
            continue;
        }
        if owner_id.is_null() {
            continue;
        }
        let owner_id = match cgnumber_to::<i32>(owner_id) {
            Ok(num) => num,
            Err(_) => continue,
        } as pid_t;

        let mut window_id: *const c_void = std::ptr::null();
        if unsafe {
            CFDictionaryGetValueIfPresent(window, kCGWindowNumber as *mut c_void, &mut window_id)
        } == 0
        {
            continue;
        }
        if window_id.is_null() {
            continue;
        }
        let window_id = match cgnumber_to::<u32>(window_id) {
            Ok(num) => num,
            Err(_) => continue,
        };

        let mut window_layer: *const c_void = std::ptr::null();
        if unsafe {
            CFDictionaryGetValueIfPresent(window, kCGWindowLayer as *mut c_void, &mut window_layer)
        } == 0
        {
            continue;
        }
        if window_layer.is_null() {
            continue;
        }
        let window_layer = match cgnumber_to::<CGWindowLevel>(window_layer) {
            Ok(num) => num,
            Err(_) => continue,
        };

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
        let window_rect = match CGRect::from_dict_representation(window_bounds.deref()) {
            None => {
                continue;
            }
            Some(rect) => rect,
        };

        let floating_window_level = unsafe {
            CGWindowLevelForKey(_CGWindowLevelKey::FloatingWindowLevelKey as CGWindowLevelKey)
        };
        let main_menu_window_level = unsafe {
            CGWindowLevelForKey(_CGWindowLevelKey::MainMenuWindowLevelKey as CGWindowLevelKey)
        };

        /*
        The window layer determines the order in which windows are displayed on the screen.
        We're interested in the following window layers:
        0 - The window is behind all other windows on the screen,
            which means it might not be visible and is obscured by
            any other windows that are on top of it.
        1 - The window is displayed above the desktop background,
            but below most other windows on the screen.
            It's likely to be partially or fully obscured by other windows.
        2 - The window is displayed above most other windows on the screen,
            making it visible and accessible to the user.
        3 - The window is displayed above most other windows on the screen
            and is floating, which means it's always on top.
        24 - The window is displayed above the main menu level.
            Full-screen windows and windows opened with Cmd+tab are
            displayed at this level.
        */

        let is_fullscreen_window =
            window_layer > main_menu_window_level && screen_size.width == window_rect.size.width;
        let is_regular_window = window_layer < floating_window_level;

        if is_fullscreen_window {
            println!("info!(): Detected a fullscreen window!");
        }

        if is_regular_window {
            println!("info!(): Detected a regular window!");
        }

        if is_fullscreen_window || is_regular_window {
            let mut window_name: *const c_void = std::ptr::null();
            unsafe {
                CFDictionaryGetValueIfPresent(
                    window,
                    kCGWindowOwnerName as *mut c_void,
                    &mut window_name,
                )
            };
            if window_name.is_null() {
                unsafe {
                    CFDictionaryGetValueIfPresent(
                        window,
                        kCGWindowOwnerName as *mut c_void,
                        &mut window_name,
                    )
                };
            }
            let window_name: Option<String> = if window_name.is_null() {
                None
            } else {
                nsstring_to_string!(window_name as id)
            };

            println!(
                "info!(): My guess is this window named {:?}, with window ID: {:?} and process ID: {:?}",
                window_name.unwrap(),
                window_id,
                owner_id
            );

            unsafe {
                CFRelease(windows.cast());
            };

            return Ok((owner_id, window_id));
        }
    }

    unsafe {
        CFRelease(windows.cast());
    };

    Err(Error::NoWindowBehind)
}
