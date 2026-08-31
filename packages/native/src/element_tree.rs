/// Event types for Rust → JS communication.
/// Element IDs are f64 (JS numbers) — lossless for integers up to 2^53.
///
/// EventPayload is the single struct sent across the napi boundary for ALL
/// event types. Fields are optional — each event type populates only the
/// fields it needs. This avoids N different napi structs while keeping the
/// FFI surface small.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use napi_derive::napi;

/// Event payload sent back to JS when a user interacts with an element.
#[derive(Debug, Clone)]
#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    derive(serde::Serialize)
)]
#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), napi(object))]
pub struct EventPayload {
    /// Numeric element ID (matches the ID assigned in JS via createElement).
    pub element_id: f64,

    /// Event type string — matches the key used in EVENT_PROPS on the JS side.
    /// e.g. "click", "mouseDown", "mouseEnter", "keyDown", "scroll", etc.
    pub event_type: String,

    // ── Mouse position ───────────────────────────────────────────────
    /// Mouse X position in window coordinates (pixels).
    pub x: Option<f64>,
    /// Mouse Y position in window coordinates (pixels).
    pub y: Option<f64>,

    // ── Mouse button ─────────────────────────────────────────────────
    /// Which mouse button: 0=left, 1=middle, 2=right.
    /// Populated for: mouseDown, mouseUp, click, mouseDownOutside, contextMenu.
    pub button: Option<u32>,

    /// Number of consecutive clicks (1=single, 2=double, 3=triple).
    /// Populated for: mouseDown, mouseUp, click.
    pub click_count: Option<u32>,

    /// Whether this is a right-click (convenience for click events).
    /// true when button==2 or ClickEvent::is_right_click().
    pub is_right_click: Option<bool>,

    /// Which mouse button is currently held during a mouseMove.
    /// Same encoding as `button`: 0=left, 1=middle, 2=right.
    /// Populated for: mouseMove.
    pub pressed_button: Option<u32>,

    // ── Keyboard ─────────────────────────────────────────────────────
    /// Key name, e.g. "a", "enter", "escape", "down", "left", "f1".
    /// Populated for: keyDown, keyUp.
    pub key: Option<String>,

    /// The character produced by the key press (e.g. "ß" for option-s).
    /// May differ from `key` when modifiers are active.
    /// Populated for: keyDown, keyUp.
    pub key_char: Option<String>,

    /// Whether this is a key-repeat event (key held down).
    /// Populated for: keyDown.
    pub is_held: Option<bool>,

    // ── Scroll ───────────────────────────────────────────────────────
    /// Scroll delta on the X axis (pixels or lines, see `precise`).
    /// Populated for: scroll.
    pub delta_x: Option<f64>,

    /// Scroll delta on the Y axis (pixels or lines, see `precise`).
    /// Populated for: scroll.
    pub delta_y: Option<f64>,

    /// true = pixel-precise (trackpad), false = line-based (mouse wheel).
    /// Populated for: scroll.
    pub precise: Option<bool>,

    /// Touch phase for scroll: "started", "moved", "ended".
    /// Populated for: scroll (trackpad gestures).
    pub touch_phase: Option<String>,

    // ── Hover ────────────────────────────────────────────────────────
    /// true = mouse entered element, false = mouse left element.
    /// Populated for: mouseEnter, mouseLeave.
    pub hovered: Option<bool>,

    // ── Custom element payloads ──────────────────────────────────────
    /// Element-defined string payload.
    /// Populated for: `<diff>` toggleFile (the file path), showMore (the
    /// hidden line count), and lineClick (the line text); `<markdown>`
    /// linkClick (the URL).
    pub value: Option<String>,

    /// Line number on the pre-change side. Populated for: `<diff>` lineClick.
    pub old_line: Option<f64>,

    /// Line number on the post-change side. Populated for: `<diff>` lineClick.
    pub new_line: Option<f64>,

    /// First visible logical index. Populated for: `<virtual-list>` visibleRange.
    pub start_index: Option<f64>,

    /// Exclusive end of the visible logical range. Populated for: visibleRange.
    pub end_index: Option<f64>,

    /// Final normalized primary-pane size. Populated by `<split-view>` only
    /// after a native drag commits; pointer moves never cross the FFI boundary.
    pub ratio: Option<f64>,

    /// Committed serializable layout emitted by `<dock-workspace>`. Native
    /// pointer tracking never crosses the FFI boundary; this is sent only
    /// after a semantic mutation commits.
    pub layout: Option<String>,

    /// Base64-encoded bytes emitted by `<terminal>` keyboard input.
    pub data_base64: Option<String>,

    /// Terminal viewport row count emitted after native measurement.
    pub rows: Option<f64>,

    /// Terminal viewport column count emitted after native measurement.
    pub cols: Option<f64>,

    // ── Browser surface observations ────────────────────────────────
    /// Current URL reported by a native BrowserSurface navigation or loading event.
    pub browser_url: Option<String>,
    /// Whether the native BrowserSurface is currently loading.
    pub browser_is_loading: Option<bool>,
    /// Whether the native BrowserSurface can navigate backward.
    pub browser_can_go_back: Option<bool>,
    /// Whether the native BrowserSurface can navigate forward.
    pub browser_can_go_forward: Option<bool>,
    /// Immutable BrowserSurface profile identity.
    pub browser_profile_id: Option<String>,
    /// Platform-generated identifier for an observed download.
    pub browser_download_id: Option<String>,
    /// Suggested filename reported by WebKit for an observed download.
    pub browser_suggested_filename: Option<String>,
    /// Caller-generated request identity for an explicit browser-data clear.
    pub browser_request_id: Option<String>,
    /// Browser action category requiring one matching host decision.
    pub browser_action_kind: Option<String>,
    /// Host navigation intent category when browser_action_kind is navigationIntent.
    pub browser_navigation_intent: Option<String>,
    /// Whether a requested WebKit navigation targets the main frame.
    pub browser_is_main_frame: Option<bool>,
    /// Whether WebKit classifies a navigation action as a download.
    pub browser_should_perform_download: Option<bool>,
    /// Whether WebKit can render a navigation response in the browser surface.
    pub browser_can_show_mime_type: Option<bool>,

    // ── Modifiers ────────────────────────────────────────────────────
    pub modifiers: Option<EventModifiers>,
}

impl Default for EventPayload {
    fn default() -> Self {
        Self {
            element_id: 0.0,
            event_type: String::new(),
            x: None,
            y: None,
            button: None,
            click_count: None,
            is_right_click: None,
            pressed_button: None,
            key: None,
            key_char: None,
            is_held: None,
            delta_x: None,
            delta_y: None,
            precise: None,
            touch_phase: None,
            hovered: None,
            value: None,
            old_line: None,
            new_line: None,
            start_index: None,
            end_index: None,
            ratio: None,
            layout: None,
            data_base64: None,
            rows: None,
            cols: None,
            browser_url: None,
            browser_is_loading: None,
            browser_can_go_back: None,
            browser_can_go_forward: None,
            browser_profile_id: None,
            browser_download_id: None,
            browser_suggested_filename: None,
            browser_request_id: None,
            browser_action_kind: None,
            browser_navigation_intent: None,
            browser_is_main_frame: None,
            browser_should_perform_download: None,
            browser_can_show_mime_type: None,
            modifiers: None,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    derive(serde::Serialize)
)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), napi(object))]
pub struct EventModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

impl Default for EventModifiers {
    fn default() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            cmd: false,
        }
    }
}

/// Convert GPUI Modifiers → our napi EventModifiers.
impl From<gpui::Modifiers> for EventModifiers {
    fn from(m: gpui::Modifiers) -> Self {
        Self {
            shift: m.shift,
            ctrl: m.control,
            alt: m.alt,
            cmd: m.platform, // platform = Cmd on macOS, Win on Windows
        }
    }
}
