use std::{collections::HashMap, sync::Mutex};

use accessibility_sys::{
    kAXErrorSuccess, kAXFrontmostAttribute, kAXMainAttribute, kAXRaiseAction, kAXWindowsAttribute,
    AXError, AXUIElementCopyAttributeValue, AXUIElementCreateApplication, AXUIElementPerformAction,
    AXUIElementRef, AXUIElementSetAttributeValue,
};
use cocoa::{base::id, foundation::NSArray};
use core_foundation::{
    base::{CFRelease, CFRetain, CFTypeRef, TCFType},
    boolean::kCFBooleanTrue,
    string::CFString,
};
use core_graphics::display::CGWindowID;
use tauri::{AppHandle, Manager};

#[allow(non_camel_case_types)]
type pid_t = i32;

#[derive(Debug)]
pub enum Error {
    CouldNotFocusWindow,
    CouldNotGetWindowArray,
    CouldNotGetWindowsAccessibility,
    WindowNotFound(u32),
    CouldNotBringWindowToFront,
}

/// A wrapper of AXUIElementRef that marks it as safe to Send across threads.
pub struct AXUIElementRefHandle(pub *mut accessibility_sys::__AXUIElement);
unsafe impl Send for AXUIElementRefHandle {}

#[derive(Default)]
pub struct Store {
    cached_windows: HashMap<u32, AXUIElementRefHandle>,
}

#[derive(Default)]
pub struct State(pub Mutex<Store>);

pub fn query_accessibility_permissions(prompt: bool) -> bool {
    if prompt {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
    } else {
        macos_accessibility_client::accessibility::application_is_trusted()
    }
}

extern "C" {
    pub fn _AXUIElementGetWindow(element: AXUIElementRef, out: *mut CGWindowID) -> AXError;
}

/// Get reference to Window's accessibility object with the specified window and process(owner) id.
///
/// # Arguments
/// * `owner_id` - The process ID of the window's owner.
/// * `window_id` - The window ID of the window.
///
fn get_axwindow(owner_id: pid_t, window_id: u32) -> Result<AXUIElementRefHandle, Error> {
    let ax_application = unsafe { AXUIElementCreateApplication(owner_id) };
    let mut windows_ref: CFTypeRef = std::ptr::null();

    if ax_application.is_null() {
        return Err(Error::CouldNotGetWindowArray);
    }

    if unsafe {
        AXUIElementCopyAttributeValue(
            ax_application,
            CFString::new(kAXWindowsAttribute).as_concrete_TypeRef(),
            &mut windows_ref as *mut CFTypeRef,
        )
    } != kAXErrorSuccess
    {
        unsafe {
            CFRelease(ax_application.cast());
        }

        return Err(Error::CouldNotGetWindowsAccessibility);
    }

    if windows_ref.is_null() {
        unsafe {
            CFRelease(windows_ref.cast());
            CFRelease(ax_application.cast());
        }

        return Err(Error::CouldNotGetWindowsAccessibility);
    }

    let windows_nsarray = windows_ref as id;

    let count = unsafe { NSArray::count(windows_nsarray) };

    for i in 0..count {
        let ax_window = unsafe { NSArray::objectAtIndex(windows_nsarray, i) };

        let ax_window_id = {
            let mut id: CGWindowID = 0;
            if unsafe { _AXUIElementGetWindow(ax_window as AXUIElementRef, &mut id) }
                != kAXErrorSuccess
            {
                continue;
            }
            id
        };

        if ax_window_id == window_id {
            unsafe {
                CFRetain(ax_window.cast());
                CFRelease(windows_ref.cast());
                CFRelease(ax_application.cast());
            }

            return Ok(AXUIElementRefHandle(ax_window as AXUIElementRef));
        }
    }

    unsafe {
        CFRelease(windows_ref.cast());
        CFRelease(ax_application.cast());
    }

    Err(Error::WindowNotFound(window_id))
}

/// Gets the reference to the accessibility object of a window.
/// It caches the reference on first call.
/// Other calls return the cached reference to the accessibility object.
///
/// Note: Make sure to release ax_app_ref after use.
///
/// # Arguments
/// * `owner_id` - The process id of the window's owner.
/// * `window_id` - The window id of the window.
/// * `app_handle` - The tauri app handle.
/// # Returns
/// * `AXUIElementRef` - The accessibility object of the window's app.
/// * `AXUIElementRef` - The accessibility object of the window.
///
pub fn get_axuielements(
    owner_id: pid_t,
    window_id: u32,
    app_handle: tauri::AppHandle,
) -> Result<(AXUIElementRef, AXUIElementRef), Error> {
    let is_cached = app_handle
        .app_handle()
        .state::<State>()
        .0
        .lock()
        .unwrap()
        .cached_windows
        .contains_key(&window_id);

    if !is_cached {
        cache_axwindow(owner_id, window_id, app_handle.app_handle());
    }

    let ax_app_ref = unsafe { AXUIElementCreateApplication(owner_id) } as AXUIElementRef;
    let state = app_handle.state::<State>();
    let ax_windows_cache = &state.0.lock().unwrap().cached_windows;

    let ax_window_ref = ax_windows_cache
        .get(&window_id)
        .ok_or(Error::WindowNotFound(window_id))?;
    Ok((ax_app_ref, ax_window_ref.0))
}

pub fn bring_window_to_top(
    ax_app_ref: AXUIElementRef,
    ax_window_ref: AXUIElementRef,
) -> Result<(), Error> {
    if unsafe {
        AXUIElementSetAttributeValue(
            ax_app_ref,
            CFString::new(kAXFrontmostAttribute).as_concrete_TypeRef(),
            kCFBooleanTrue as _,
        )
    } != kAXErrorSuccess
    {
        return Err(Error::CouldNotBringWindowToFront);
    }

    if unsafe {
        AXUIElementSetAttributeValue(
            ax_window_ref,
            CFString::new(kAXMainAttribute).as_concrete_TypeRef(),
            kCFBooleanTrue as _,
        )
    } != kAXErrorSuccess
    {
        return Err(Error::CouldNotBringWindowToFront);
    }

    Ok(())
}

pub fn focus_window(ax_window_ref: AXUIElementRef) -> Result<(), Error> {
    if unsafe {
        AXUIElementPerformAction(
            ax_window_ref,
            CFString::new(kAXRaiseAction).as_concrete_TypeRef(),
        )
    } != kAXErrorSuccess
    {
        return Err(Error::CouldNotFocusWindow);
    }

    Ok(())
}

fn cache_axwindow(owner_id: pid_t, window_id: u32, app_handle: AppHandle) {
    let is_cached = app_handle
        .app_handle()
        .state::<State>()
        .0
        .lock()
        .unwrap()
        .cached_windows
        .contains_key(&window_id);

    if !is_cached {
        if let Ok(ax_window_ref) = get_axwindow(owner_id, window_id) {
            app_handle
                .state::<State>()
                .0
                .lock()
                .unwrap()
                .cached_windows
                .insert(window_id, ax_window_ref);
        }
    }
}
