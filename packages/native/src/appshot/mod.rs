//! Native, privacy-preserving appshot capability.
//!
//! The JavaScript ABI deliberately exposes no shareable-content identity. A
//! selected ScreenCaptureKit filter is kept behind a renderer-local, one-shot
//! handle and is never serialized or logged.

#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(all(feature = "test-support", target_os = "macos"))]
mod test;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::collections::{HashMap, HashSet};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use napi::bindgen_prelude::{Buffer, Error, Result};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use napi_derive::napi;

/// Closed, native-owned privacy status vocabulary. Platform failures never
/// cross this boundary as strings.
#[cfg_attr(
    not(all(target_arch = "wasm32", target_os = "unknown")),
    napi(string_enum = "lowercase")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppshotPermissionStatus {
    Granted,
    Missing,
}

/// Closed picker result vocabulary. A selected source is represented only by
/// an opaque one-shot handle.
#[cfg_attr(
    not(all(target_arch = "wasm32", target_os = "unknown")),
    napi(string_enum = "lowercase")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppshotSelectionStatus {
    Selected,
    Cancelled,
}

/// Only the permission decision crosses the native ABI. `restart_required`
/// communicates the TCC transition without exposing platform error details.
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), napi(object))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppshotPermission {
    pub status: AppshotPermissionStatus,
    pub restart_required: bool,
}

/// The picker result deliberately has no source metadata. A handle is present
/// only for a selected source and becomes invalid after its first capture or
/// explicit disposal.
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), napi(object))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppshotSelection {
    pub status: AppshotSelectionStatus,
    pub handle: Option<String>,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
/// Renderer-local ownership for opaque appshot handles and shortcut tokens.
/// Values are intentionally opaque: no native source identity is ever copied
/// into a string that can reach JS, a fixture, a log, or persistence.
pub(crate) struct AppshotState {
    next_handle: u64,
    pub(crate) next_shortcut: u64,
    /// Native ScreenCaptureKit filters are renderer-local and are never
    /// converted to source metadata. The integer is an Objective-C retain
    /// pointer on macOS and is consumed exactly once.
    #[cfg(target_os = "macos")]
    filters: HashMap<String, usize>,
    #[cfg(target_os = "macos")]
    picker_observers: HashSet<usize>,
    handles: HashSet<String>,
    pub(crate) shortcuts: HashMap<String, String>,
    #[cfg(target_os = "macos")]
    pub(crate) shortcut_refs: HashMap<String, (u32, usize)>,
    #[cfg(all(feature = "test-support", target_os = "macos"))]
    test_selection: Option<bool>,
    #[cfg(all(feature = "test-support", target_os = "macos"))]
    test_permission: bool,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl Default for AppshotState {
    fn default() -> Self {
        Self {
            next_handle: 1,
            next_shortcut: 1,
            #[cfg(target_os = "macos")]
            filters: HashMap::new(),
            #[cfg(target_os = "macos")]
            picker_observers: HashSet::new(),
            handles: HashSet::new(),
            shortcuts: HashMap::new(),
            #[cfg(target_os = "macos")]
            shortcut_refs: HashMap::new(),
            #[cfg(all(feature = "test-support", target_os = "macos"))]
            test_selection: None,
            #[cfg(all(feature = "test-support", target_os = "macos"))]
            test_permission: true,
        }
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl AppshotState {
    pub(crate) fn issue_selected_handle(&mut self) -> AppshotSelection {
        let handle = format!("appshot-{}", self.next_handle);
        self.next_handle += 1;
        self.handles.insert(handle.clone());
        AppshotSelection {
            status: AppshotSelectionStatus::Selected,
            handle: Some(handle),
        }
    }

    pub(crate) fn cancelled() -> AppshotSelection {
        AppshotSelection {
            status: AppshotSelectionStatus::Cancelled,
            handle: None,
        }
    }

    pub(crate) fn consume_handle(&mut self, handle: &str) -> Result<()> {
        if self.handles.remove(handle) {
            Ok(())
        } else {
            Err(Error::from_reason("The appshot handle is unavailable"))
        }
    }

    /// Takes ownership of an already-retained `SCContentFilter` and returns a
    /// token that contains no source identity. The filter is released after
    /// the first capture, explicit disposal, or renderer drop.
    #[cfg(target_os = "macos")]
    pub(crate) fn issue_selected_filter(&mut self, filter: usize) -> AppshotSelection {
        let handle = format!("appshot-{}", self.next_handle);
        self.next_handle += 1;
        self.filters.insert(handle.clone(), filter);
        AppshotSelection {
            status: AppshotSelectionStatus::Selected,
            handle: Some(handle),
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn take_filter(&mut self, handle: &str) -> Result<usize> {
        self.filters.remove(handle).ok_or_else(unavailable)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn track_picker_observer(&mut self, observer: usize) {
        self.picker_observers.insert(observer);
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn untrack_picker_observer(&mut self, observer: usize) -> bool {
        self.picker_observers.remove(&observer)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn take_picker_observers(&mut self) -> Vec<usize> {
        self.picker_observers.drain().collect()
    }

    pub(crate) fn dispose_handle(&mut self, handle: &str) {
        #[cfg(target_os = "macos")]
        if let Some(filter) = self.filters.remove(handle) {
            unsafe { macos::release_object(filter) };
        }
        self.handles.remove(handle);
    }

    pub(crate) fn register_shortcut(&mut self, shortcut: String) -> Result<String> {
        if shortcut.trim().is_empty() || self.shortcuts.values().any(|value| value == &shortcut) {
            return Err(Error::from_reason("The global shortcut is unavailable"));
        }
        let token = format!("shortcut-{}", self.next_shortcut);
        self.next_shortcut += 1;
        self.shortcuts.insert(token.clone(), shortcut);
        Ok(token)
    }

    pub(crate) fn unregister_shortcut(&mut self, token: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // The deterministic TestGpuixRenderer intentionally has no Carbon
            // registration; production registrations always have a native ref.
            #[cfg(feature = "test-support")]
            if !self.shortcut_refs.contains_key(token) {
                return self
                    .shortcuts
                    .remove(token)
                    .map(|_| ())
                    .ok_or_else(unavailable);
            }
            return macos::unregister_shortcut(self, token);
        }
        #[cfg(not(target_os = "macos"))]
        if self.shortcuts.remove(token).is_some() {
            Ok(())
        } else {
            Err(Error::from_reason(
                "The global shortcut token is unavailable",
            ))
        }
    }

    pub(crate) fn dispose_all(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let filters: Vec<usize> = self.filters.drain().map(|(_, filter)| filter).collect();
            let observers = self.take_picker_observers();
            if !filters.is_empty() || !observers.is_empty() {
                macos::dispose_appshot_resources(filters, observers);
            }
        }
        self.handles.clear();
        #[cfg(target_os = "macos")]
        macos::dispose_shortcuts(self);
        self.shortcuts.clear();
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) fn one_shot_png() -> Buffer {
    // Deterministic test-only 1×1 transparent PNG. Production bytes always
    // come from SCScreenshotManager and never from the renderer surface.
    const TEST_PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0\x1f\x15\xc4\x89\0\0\0\x0dIDAT\x08\xd7c\xf8\xcf\xc0\xf0\x1f\0\x05\0\x01\xff\x89\x99=\x1d\0\0\0\0IEND\xaeB`\x82";
    Buffer::from(TEST_PNG.to_vec())
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) fn unavailable() -> Error {
    Error::from_reason("Appshot is unavailable")
}
