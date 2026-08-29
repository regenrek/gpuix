//! Real, renderer-local macOS ScreenCaptureKit plumbing. No source identity
//! crosses this module's public callers.
use super::{
    unavailable, AppshotPermission, AppshotPermissionStatus, AppshotSelection, AppshotState,
};
use block::ConcreteBlock;
use cocoa::base::{id, nil, BOOL, NO, YES};
use core_graphics::access::ScreenCaptureAccess;
use futures::channel::oneshot;
use napi::bindgen_prelude::{Buffer, Result};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use std::{
    collections::HashMap,
    ffi::c_void,
    ptr,
    sync::{Arc, Mutex, Once},
};

const SINGLE_WINDOW_MODE: usize = 1;
const WINDOW_STYLE: isize = 1;
const PNG_FILE_TYPE: usize = 4;
static PICKER_INIT: Once = Once::new();
static mut PICKER_CLASS: *const Class = ptr::null();

// Keep the framework in the production Mach-O image. The class reference is
// deliberately used rather than relying on Objective-C's dynamic lookup,
// which would otherwise let the linker remove ScreenCaptureKit.
#[link(name = "ScreenCaptureKit", kind = "framework")]
extern "C" {
    #[link_name = "OBJC_CLASS_$_SCContentSharingPicker"]
    static SC_CONTENT_SHARING_PICKER_CLASS: *const c_void;
}

#[link(name = "System")]
extern "C" {
    #[link_name = "_dispatch_main_q"]
    static DISPATCH_MAIN_QUEUE: c_void;
    fn dispatch_async_f(queue: *mut c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}

extern "C" fn run_on_renderer_main(context: *mut c_void) {
    unsafe {
        let callback: Box<Box<dyn FnOnce() + Send>> = Box::from_raw(context.cast());
        callback();
    }
}

/// ScreenCaptureKit invokes completions on its own queue. The renderer and
/// AppKit owner are the main queue driven by `GpuixRenderer::tick`; no
/// platform object is inspected, encoded, or released on the callback queue.
fn on_renderer_main(callback: impl FnOnce() + Send + 'static) {
    unsafe {
        let callback: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(callback));
        dispatch_async_f(
            std::ptr::addr_of!(DISPATCH_MAIN_QUEUE).cast_mut(),
            Box::into_raw(callback).cast(),
            run_on_renderer_main,
        );
    }
}

fn require_screencapturekit() {
    unsafe {
        let _ = ptr::read_volatile(&SC_CONTENT_SHARING_PICKER_CLASS);
    }
}

pub(crate) fn preflight_permission() -> AppshotPermission {
    let granted = ScreenCaptureAccess.preflight();
    AppshotPermission {
        status: if granted {
            AppshotPermissionStatus::Granted
        } else {
            AppshotPermissionStatus::Missing
        },
        restart_required: false,
    }
}
pub(crate) fn request_permission() -> AppshotPermission {
    let before = ScreenCaptureAccess.preflight();
    let granted = ScreenCaptureAccess.request();
    AppshotPermission {
        status: if granted {
            AppshotPermissionStatus::Granted
        } else {
            AppshotPermissionStatus::Missing
        },
        restart_required: !before && granted,
    }
}
pub(crate) unsafe fn release_object(value: usize) {
    let object = value as id;
    if object != nil {
        let _: () = msg_send![object, release];
    }
}

struct PickerRequest {
    state: Arc<Mutex<AppshotState>>,
    sender: Option<oneshot::Sender<Result<AppshotSelection>>>,
    picker: id,
}
unsafe fn request(this: &Object) -> &mut PickerRequest {
    &mut *(*this.get_ivar::<*mut c_void>("request") as *mut PickerRequest)
}
unsafe fn take_picker_request(this: &Object) -> Option<Box<PickerRequest>> {
    let pointer = *this.get_ivar::<*mut c_void>("request");
    if pointer.is_null() {
        return None;
    }
    (*(this as *const Object as *mut Object)).set_ivar("request", ptr::null_mut::<c_void>());
    let request = Box::from_raw(pointer as *mut PickerRequest);
    let _: () = msg_send![request.picker, removeObserver:this];
    let _: () = msg_send![request.picker, setActive:NO];
    if request
        .state
        .lock()
        .unwrap()
        .untrack_picker_observer(this as *const _ as usize)
    {
        release_callback_object(this as *const _ as usize);
    }
    Some(request)
}
fn retain_callback_object(value: id) -> usize {
    unsafe {
        let _: id = msg_send![value, retain];
        value as usize
    }
}
fn release_callback_object(value: usize) {
    unsafe {
        let _: () = msg_send![value as id, release];
    }
}
extern "C" fn picker_cancelled(this: &Object, _: Sel, _: id, _: id) {
    let observer = retain_callback_object(this as *const _ as id);
    on_renderer_main(move || unsafe {
        let this = &*(observer as *const Object);
        if let Some(mut request) = take_picker_request(this) {
            if let Some(sender) = request.sender.take() {
                sender.send(Ok(AppshotState::cancelled())).ok();
            }
        }
        release_callback_object(observer);
    });
}
extern "C" fn picker_selected(this: &Object, _: Sel, _: id, filter: id, _: id) {
    let observer = retain_callback_object(this as *const _ as id);
    let filter = if filter == nil {
        None
    } else {
        Some(retain_callback_object(filter))
    };
    on_renderer_main(move || unsafe {
        let this = &*(observer as *const Object);
        if let Some(mut request) = take_picker_request(this) {
            let result = match filter {
                Some(filter) => Ok(request.state.lock().unwrap().issue_selected_filter(filter)),
                None => Err(unavailable()),
            };
            if let Some(sender) = request.sender.take() {
                sender.send(result).ok();
            }
        }
        release_callback_object(observer);
    });
}
extern "C" fn picker_failed(this: &Object, _: Sel, _: id) {
    let observer = retain_callback_object(this as *const _ as id);
    on_renderer_main(move || unsafe {
        let this = &*(observer as *const Object);
        if let Some(mut request) = take_picker_request(this) {
            if let Some(sender) = request.sender.take() {
                sender.send(Err(unavailable())).ok();
            }
        }
        release_callback_object(observer);
    });
}
unsafe fn picker_class() -> *const Class {
    PICKER_INIT.call_once(|| {
        let mut decl = ClassDecl::new("GpuixAppshotPickerObserver", class!(NSObject))
            .expect("picker observer");
        decl.add_ivar::<*mut c_void>("request");
        decl.add_method(
            sel!(contentSharingPicker:didCancelForStream:),
            picker_cancelled as extern "C" fn(&Object, Sel, id, id),
        );
        decl.add_method(
            sel!(contentSharingPicker:didUpdateWithFilter:forStream:),
            picker_selected as extern "C" fn(&Object, Sel, id, id, id),
        );
        decl.add_method(
            sel!(contentSharingPickerStartDidFailWithError:),
            picker_failed as extern "C" fn(&Object, Sel, id),
        );
        PICKER_CLASS = decl.register();
    });
    PICKER_CLASS
}

/// Apple controls explicit selection. This starts at window style and permits
/// exactly its single-window mode. Completion settles the N-API promise; it
/// never spins a nested CoreFoundation loop on the JavaScript thread.
pub(crate) fn select_window(
    state: Arc<Mutex<AppshotState>>,
    sender: oneshot::Sender<Result<AppshotSelection>>,
) {
    on_renderer_main(move || unsafe {
        require_screencapturekit();
        let picker: id = msg_send![class!(SCContentSharingPicker), sharedPicker];
        if picker == nil {
            sender.send(Err(unavailable())).ok();
            return;
        }
        let config: id = msg_send![class!(SCContentSharingPickerConfiguration), alloc];
        let config: id = msg_send![config, init];
        if config == nil {
            sender.send(Err(unavailable())).ok();
            return;
        }
        let _: () = msg_send![config, setAllowedPickerModes:SINGLE_WINDOW_MODE];
        let _: () = msg_send![config, setAllowsChangingSelectedContent:NO];
        let _: () = msg_send![picker, setDefaultConfiguration:config];
        let _: () = msg_send![config, release];
        let observer: id = msg_send![picker_class(), alloc];
        let observer: id = msg_send![observer, init];
        if observer == nil {
            sender.send(Err(unavailable())).ok();
            return;
        }
        let boxed = Box::new(PickerRequest {
            state: state.clone(),
            sender: Some(sender),
            picker,
        });
        observer
            .as_mut()
            .expect("checked observer")
            .set_ivar("request", Box::into_raw(boxed) as *mut c_void);
        let _: () = msg_send![picker, addObserver:observer];
        state
            .lock()
            .unwrap()
            .track_picker_observer(retain_callback_object(observer));
        let _: () = msg_send![observer, release];
        let _: () = msg_send![picker, setActive:YES];
        let _: () = msg_send![picker, presentPickerUsingContentStyle:WINDOW_STYLE];
    });
}

/// A renderer can be dropped while Apple's picker is still visible. The
/// state-owned retain gives teardown one exact path: remove observer, dismiss
/// the picker, settle the promise, and release the retained observer.
pub(crate) fn dispose_appshot_resources(filters: Vec<usize>, observers: Vec<usize>) {
    on_renderer_main(move || unsafe {
        for observer in observers {
            let this = &*(observer as *const Object);
            if let Some(mut request) = take_picker_request(this) {
                if let Some(sender) = request.sender.take() {
                    sender.send(Err(unavailable())).ok();
                }
            }
            release_callback_object(observer);
        }
        for filter in filters {
            release_object(filter);
        }
    });
}

unsafe fn png_bytes(image: id) -> Option<Buffer> {
    if image == nil {
        return None;
    }
    let bitmap: id = msg_send![class!(NSBitmapImageRep), alloc];
    let bitmap: id = msg_send![bitmap, initWithCGImage:image];
    if bitmap == nil {
        return None;
    }
    let data: id = msg_send![bitmap, representationUsingType:PNG_FILE_TYPE properties:nil];
    if data == nil {
        let _: () = msg_send![bitmap, release];
        return None;
    }
    let length: usize = msg_send![data, length];
    let bytes: *const u8 = msg_send![data, bytes];
    let png = (!bytes.is_null() && length >= 8)
        .then(|| Buffer::from(std::slice::from_raw_parts(bytes, length).to_vec()));
    let _: () = msg_send![bitmap, release];
    png
}
fn capture_filter(filter: usize, sender: oneshot::Sender<Result<Buffer>>) {
    unsafe {
        require_screencapturekit();
        let config: id = msg_send![class!(SCStreamConfiguration), alloc];
        let config: id = msg_send![config, init];
        if config == nil {
            release_object(filter);
            sender.send(Err(unavailable())).ok();
            return;
        }
        let pending = Arc::new(Mutex::new(Some((filter, sender))));
        let completion_pending = pending.clone();
        let completion = ConcreteBlock::new(move |image: id, error: id| {
            let image = (error == nil && image != nil).then(|| retain_callback_object(image));
            let completion_pending = completion_pending.clone();
            on_renderer_main(move || {
                let Some((filter, sender)) = completion_pending.lock().unwrap().take() else {
                    return;
                };
                let result = image
                    .and_then(|image| unsafe { png_bytes(image as id) })
                    .ok_or_else(unavailable);
                if let Some(image) = image {
                    release_callback_object(image);
                }
                unsafe {
                    release_object(filter);
                }
                sender.send(result).ok();
            });
        })
        .copy();
        let _: () = msg_send![class!(SCScreenshotManager), captureImageWithFilter:filter as id configuration:config completionHandler:completion];
        let _: () = msg_send![config, release];
    }
}
pub(crate) fn capture_selected(
    state: &Arc<Mutex<AppshotState>>,
    handle: &str,
    sender: oneshot::Sender<Result<Buffer>>,
) {
    match state.lock().unwrap().take_filter(handle) {
        Ok(filter) => on_renderer_main(move || capture_filter(filter, sender)),
        Err(error) => {
            sender.send(Err(error)).ok();
        }
    }
}

/// The only internal source lookup: one SCShareableContent request, match
/// frontmost PID to one active/on-screen window, then drop every identity
/// object while retaining only the opaque filter for one capture.
pub(crate) fn capture_frontmost(sender: oneshot::Sender<Result<Buffer>>) {
    on_renderer_main(move || unsafe {
        require_screencapturekit();
        let pending = Arc::new(Mutex::new(Some(sender)));
        let completion_pending = pending.clone();
        let completion = ConcreteBlock::new(move |content: id, error: id| {
            let content = (error == nil && content != nil).then(|| retain_callback_object(content));
            let completion_pending = completion_pending.clone();
            on_renderer_main(move || {
                let Some(sender) = completion_pending.lock().unwrap().take() else {
                    return;
                };
                let filter = content.and_then(|content| unsafe { frontmost_filter(content as id) });
                if let Some(content) = content {
                    release_callback_object(content);
                }
                match filter {
                    Some(filter) => capture_filter(filter, sender),
                    None => {
                        sender.send(Err(unavailable())).ok();
                    }
                }
            });
        })
        .copy();
        let _: () = msg_send![class!(SCShareableContent), getShareableContentExcludingDesktopWindows:YES onScreenWindowsOnly:YES completionHandler:completion];
    });
}
unsafe fn frontmost_filter(content: id) -> Option<usize> {
    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let app: id = msg_send![workspace, frontmostApplication];
    if app == nil {
        return None;
    }
    let pid: i32 = msg_send![app, processIdentifier];
    let windows: id = msg_send![content, windows];
    let count: usize = msg_send![windows, count];
    let mut found: id = nil;
    for index in 0..count {
        let window: id = msg_send![windows, objectAtIndex:index];
        let on_screen: BOOL = msg_send![window, isOnScreen];
        let active: BOOL = msg_send![window, isActive];
        let owner: id = msg_send![window, owningApplication];
        let owner_pid: i32 = if owner == nil {
            -1
        } else {
            msg_send![owner, processID]
        };
        if on_screen == YES && active == YES && owner_pid == pid {
            if found != nil {
                return None;
            }
            found = window;
        }
    }
    if found == nil {
        return None;
    }
    let filter: id = msg_send![class!(SCContentFilter), alloc];
    let filter: id = msg_send![filter, initWithDesktopIndependentWindow:found];
    (filter != nil).then_some(filter as usize)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EventHotKeyId {
    signature: u32,
    id: u32,
}
type EventHotKeyRef = *mut c_void;
type EventRef = *mut c_void;
type EventHandlerCallRef = *mut c_void;
type EventHandlerRef = *mut c_void;
#[repr(C)]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}
type EventHandlerProc = extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> i32;
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn GetApplicationEventTarget() -> *mut c_void;
    fn InstallEventHandler(
        target: *mut c_void,
        handler: EventHandlerProc,
        count: u32,
        events: *const EventTypeSpec,
        user_data: *mut c_void,
        out: *mut EventHandlerRef,
    ) -> i32;
    fn GetEventParameter(
        event: EventRef,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        size: u32,
        actual_size: *mut u32,
        out: *mut c_void,
    ) -> i32;
    fn RegisterEventHotKey(
        code: u32,
        modifiers: u32,
        id: EventHotKeyId,
        target: *mut c_void,
        options: u32,
        out: *mut EventHotKeyRef,
    ) -> i32;
    fn UnregisterEventHotKey(hotkey: EventHotKeyRef) -> i32;
}
static HOTKEY_INIT: Once = Once::new();

struct HotkeyCallback {
    token: String,
    callback: Arc<dyn Fn(String) + Send + Sync>,
}

struct HotkeyCallbacks {
    next_id: u32,
    callbacks: HashMap<u32, HotkeyCallback>,
}

impl Default for HotkeyCallbacks {
    fn default() -> Self {
        Self {
            next_id: 1,
            callbacks: HashMap::new(),
        }
    }
}

static HOTKEY_CALLBACKS: std::sync::OnceLock<Mutex<HotkeyCallbacks>> = std::sync::OnceLock::new();

fn allocate_hotkey_id() -> Result<u32> {
    let mut callbacks = HOTKEY_CALLBACKS
        .get()
        .ok_or_else(unavailable)?
        .lock()
        .unwrap();
    let id = callbacks.next_id;
    callbacks.next_id = callbacks.next_id.checked_add(1).ok_or_else(unavailable)?;
    Ok(id)
}

extern "C" fn hotkey_handler(_: EventHandlerCallRef, event: EventRef, _: *mut c_void) -> i32 {
    let mut id = EventHotKeyId {
        signature: 0,
        id: 0,
    };
    unsafe {
        let _ = GetEventParameter(
            event,
            0x2d2d2d2d,
            0x686b6964,
            ptr::null_mut(),
            std::mem::size_of::<EventHotKeyId>() as u32,
            ptr::null_mut(),
            &mut id as *mut _ as *mut c_void,
        );
    }
    if id.signature == 0x4750_5558 {
        if let Some((token, callback)) = HOTKEY_CALLBACKS.get().and_then(|callbacks| {
            callbacks
                .lock()
                .unwrap()
                .callbacks
                .get(&id.id)
                .map(|entry| (entry.token.clone(), Arc::clone(&entry.callback)))
        }) {
            callback(token);
        }
    }
    0
}
fn install_hotkey_handler() -> Result<()> {
    let mut result = Ok(());
    HOTKEY_INIT.call_once(|| unsafe {
        let spec = EventTypeSpec {
            event_class: 0x6b657962,
            event_kind: 5,
        };
        let mut handler = ptr::null_mut();
        if InstallEventHandler(
            GetApplicationEventTarget(),
            hotkey_handler,
            1,
            &spec,
            ptr::null_mut(),
            &mut handler,
        ) != 0
        {
            result = Err(unavailable());
            return;
        }
        HOTKEY_CALLBACKS.get_or_init(|| Mutex::new(HotkeyCallbacks::default()));
    });
    if HOTKEY_CALLBACKS.get().is_some() {
        Ok(())
    } else {
        result.and(Err(unavailable()))
    }
}
fn parse_shortcut(value: &str) -> Option<(u32, u32)> {
    let mut modifiers = 0;
    let mut key = None;
    for part in value.to_ascii_lowercase().split('-') {
        match part {
            "cmd" | "command" => modifiers |= 1 << 8,
            "shift" => modifiers |= 1 << 9,
            "alt" | "option" => modifiers |= 1 << 11,
            "ctrl" | "control" => modifiers |= 1 << 12,
            "a" => key = Some(0),
            "b" => key = Some(11),
            "c" => key = Some(8),
            "d" => key = Some(2),
            "e" => key = Some(14),
            "f" => key = Some(3),
            "g" => key = Some(5),
            "h" => key = Some(4),
            "i" => key = Some(34),
            "j" => key = Some(38),
            "k" => key = Some(40),
            "l" => key = Some(37),
            "m" => key = Some(46),
            "n" => key = Some(45),
            "o" => key = Some(31),
            "p" => key = Some(35),
            "q" => key = Some(12),
            "r" => key = Some(15),
            "s" => key = Some(1),
            "t" => key = Some(17),
            "u" => key = Some(32),
            "v" => key = Some(9),
            "w" => key = Some(13),
            "x" => key = Some(7),
            "y" => key = Some(16),
            "z" => key = Some(6),
            "0" => key = Some(29),
            "1" => key = Some(18),
            "2" => key = Some(19),
            "3" => key = Some(20),
            "4" => key = Some(21),
            "5" => key = Some(23),
            "6" => key = Some(22),
            "7" => key = Some(26),
            "8" => key = Some(28),
            "9" => key = Some(25),
            _ => return None,
        }
    }
    key.map(|key| (key, modifiers))
}
pub(crate) fn register_shortcut(
    state: &mut AppshotState,
    shortcut: String,
    callback: Arc<dyn Fn(String) + Send + Sync>,
) -> Result<String> {
    let (code, modifiers) = parse_shortcut(&shortcut).ok_or_else(unavailable)?;
    if state.shortcuts.values().any(|value| value == &shortcut) {
        return Err(unavailable());
    }
    install_hotkey_handler()?;
    let number = allocate_hotkey_id()?;
    let mut hotkey = ptr::null_mut();
    if unsafe {
        RegisterEventHotKey(
            code,
            modifiers,
            EventHotKeyId {
                signature: 0x4750_5558,
                id: number,
            },
            GetApplicationEventTarget(),
            0,
            &mut hotkey,
        )
    } != 0
    {
        return Err(unavailable());
    }
    let token = match state.register_shortcut(shortcut) {
        Ok(token) => token,
        Err(error) => {
            unsafe {
                let _ = UnregisterEventHotKey(hotkey);
            }
            return Err(error);
        }
    };
    HOTKEY_CALLBACKS
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .callbacks
        .insert(
            number,
            HotkeyCallback {
                token: token.clone(),
                callback,
            },
        );
    state
        .shortcut_refs
        .insert(token.clone(), (number, hotkey as usize));
    Ok(token)
}
pub(crate) fn unregister_shortcut(state: &mut AppshotState, token: &str) -> Result<()> {
    let (number, hotkey) = state
        .shortcut_refs
        .get(token)
        .copied()
        .ok_or_else(unavailable)?;
    if unsafe { UnregisterEventHotKey(hotkey as EventHotKeyRef) } != 0 {
        return Err(unavailable());
    }
    state.shortcut_refs.remove(token);
    HOTKEY_CALLBACKS
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .callbacks
        .remove(&number);
    state.shortcuts.remove(token);
    Ok(())
}
pub(crate) fn dispose_shortcuts(state: &mut AppshotState) {
    for (_, (number, hotkey)) in state.shortcut_refs.drain() {
        unsafe {
            let _ = UnregisterEventHotKey(hotkey as EventHotKeyRef);
        }
        if let Some(callbacks) = HOTKEY_CALLBACKS.get() {
            callbacks.lock().unwrap().callbacks.remove(&number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appshot_hotkey_ids_are_process_wide_and_unique() {
        HOTKEY_CALLBACKS.get_or_init(|| Mutex::new(HotkeyCallbacks::default()));
        let first = allocate_hotkey_id().unwrap();
        let second = allocate_hotkey_id().unwrap();
        assert_ne!(first, second);
    }
}
