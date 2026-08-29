//! Generic controlled native docking workbench.
//!
//! React supplies a serializable string-ID tree and panel content. Rust owns
//! normalization, hit testing, pointer capture, previews, and commit events.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use gpui::{
    px, relative, size, App, AvailableSpace, Bounds, DispatchPhase, Element, GlobalElementId,
    HitboxBehavior, HitboxId, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, Style, Styled, Window,
};
use serde::{Deserialize, Serialize};

use super::{CustomElement, CustomElementFactory, CustomRenderContext};

const TAB_HEIGHT: f32 = 30.0;
const DROP_BAND: f32 = 0.24;
const DIVIDER_HIT_SLOP: f32 = 3.0;

pub struct DockWorkspaceFactory;

impl CustomElementFactory for DockWorkspaceFactory {
    fn element_type(&self) -> &str {
        "dock-workspace"
    }
    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(DockWorkspaceElement::default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum DockLayout {
    Tabs {
        id: String,
        panels: Vec<String>,
        #[serde(default)]
        active: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        zoomed: Option<String>,
    },
    Split {
        id: String,
        direction: DockDirection,
        ratio: f32,
        first: Box<DockLayout>,
        second: Box<DockLayout>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        zoomed: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum DockDirection {
    Horizontal,
    Vertical,
}

impl DockDirection {
    fn flex(self, el: gpui::Div) -> gpui::Div {
        match self {
            Self::Horizontal => el.flex_row(),
            Self::Vertical => el.flex_col(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropTarget {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone)]
struct TabBounds {
    node_id: String,
    panel_id: String,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct TabGroupBounds {
    node_id: String,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct SplitBounds {
    node_id: String,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct DividerBounds {
    node_id: String,
    direction: DockDirection,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
enum DockControl {
    Close(String),
    ToggleZoom(String),
}

#[derive(Clone)]
struct ControlBounds {
    control: DockControl,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct DragState {
    panel_id: String,
    source_node: String,
    preview: DropTarget,
    target_node: String,
    position: Point<Pixels>,
}

#[derive(Clone)]
struct ResizeState {
    node_id: String,
    direction: DockDirection,
    bounds: Bounds<Pixels>,
    initial_ratio: f32,
}

#[derive(Clone)]
enum Interaction {
    Resize(ResizeState),
}

#[derive(Default)]
struct DockState {
    requested_layout: Option<DockLayout>,
    layout: Option<DockLayout>,
    panel_ids: Vec<String>,
    labels: HashMap<String, String>,
    bounds: Option<Bounds<Pixels>>,
    tab_bounds: Vec<TabBounds>,
    tab_group_bounds: Vec<TabGroupBounds>,
    split_bounds: Vec<SplitBounds>,
    divider_bounds: Vec<DividerBounds>,
    control_bounds: Vec<ControlBounds>,
    interaction: Option<Interaction>,
    drag: Option<DragState>,
    closable: HashSet<String>,
    pending_focus: Option<String>,
    interaction_hitbox: Option<HitboxId>,
}

pub struct DockWorkspaceElement {
    state: Rc<RefCell<DockState>>,
    focus_panel_id: Option<String>,
}

impl Default for DockWorkspaceElement {
    fn default() -> Self {
        Self {
            state: Rc::new(RefCell::new(DockState::default())),
            focus_panel_id: None,
        }
    }
}

fn normalize(
    layout: DockLayout,
    allowed: &HashSet<String>,
    nodes: &mut HashSet<String>,
    panels_seen: &mut HashSet<String>,
) -> Option<DockLayout> {
    match layout {
        DockLayout::Tabs {
            id,
            panels,
            active,
            zoomed,
        } => {
            if id.is_empty() || !nodes.insert(id.clone()) {
                return None;
            }
            if panels.is_empty()
                || panels
                    .iter()
                    .any(|panel| !allowed.contains(panel) || !panels_seen.insert(panel.clone()))
            {
                return None;
            }
            if zoomed.as_ref().is_some_and(|panel| !panels.contains(panel)) {
                return None;
            }
            let active = active
                .filter(|panel| panels.contains(panel))
                .or_else(|| panels.first().cloned());
            Some(DockLayout::Tabs {
                id,
                panels,
                active,
                zoomed,
            })
        }
        DockLayout::Split {
            id,
            direction,
            ratio,
            first,
            second,
            zoomed,
        } => {
            if id.is_empty() || !nodes.insert(id.clone()) {
                return None;
            }
            let first = normalize(*first, allowed, nodes, panels_seen);
            let second = normalize(*second, allowed, nodes, panels_seen);
            match (first, second) {
                (Some(first), Some(second)) => Some(DockLayout::Split {
                    id,
                    direction,
                    ratio: ratio.clamp(0.1, 0.9),
                    first: Box::new(first),
                    second: Box::new(second),
                    zoomed,
                }),
                _ => None,
            }
        }
    }
}

fn normalized(layout: DockLayout, allowed: &HashSet<String>) -> Option<DockLayout> {
    let layout = normalize(layout, allowed, &mut HashSet::new(), &mut HashSet::new())?;
    let root_zoomed = zoomed_panel(&layout);
    (zoom_count(&layout) == usize::from(root_zoomed.is_some())
        && root_zoomed.is_none_or(|panel_id| contains_panel(&layout, panel_id)))
    .then_some(layout)
}

fn has_unique_panel_ids(panel_ids: &[String]) -> bool {
    panel_ids.iter().collect::<HashSet<_>>().len() == panel_ids.len()
}

fn allowed_panels(state: &DockState) -> HashSet<String> {
    state.panel_ids.iter().cloned().collect()
}

fn activate(layout: &mut DockLayout, node_id: &str, panel_id: &str) -> bool {
    match layout {
        DockLayout::Tabs {
            id, panels, active, ..
        } if id == node_id && panels.contains(&panel_id.to_string()) => {
            *active = Some(panel_id.to_string());
            true
        }
        DockLayout::Split { first, second, .. } => {
            activate(first, node_id, panel_id) || activate(second, node_id, panel_id)
        }
        _ => false,
    }
}

fn activate_panel(layout: &mut DockLayout, panel_id: &str) -> bool {
    match layout {
        DockLayout::Tabs { panels, active, .. } if panels.contains(&panel_id.to_string()) => {
            *active = Some(panel_id.to_string());
            true
        }
        DockLayout::Split { first, second, .. } => {
            activate_panel(first, panel_id) || activate_panel(second, panel_id)
        }
        _ => false,
    }
}

fn remove_panel(layout: DockLayout, panel_id: &str) -> Option<DockLayout> {
    match layout {
        DockLayout::Tabs {
            id,
            panels,
            active,
            zoomed,
        } => {
            let panels: Vec<_> = panels
                .into_iter()
                .filter(|panel| panel != panel_id)
                .collect();
            (!panels.is_empty()).then(|| DockLayout::Tabs {
                id,
                active: active
                    .filter(|panel| panel != panel_id)
                    .or_else(|| panels.first().cloned()),
                panels,
                zoomed: zoomed.filter(|panel| panel != panel_id),
            })
        }
        DockLayout::Split {
            id,
            direction,
            ratio,
            first,
            second,
            zoomed,
        } => {
            let first = remove_panel(*first, panel_id);
            let second = remove_panel(*second, panel_id);
            match (first, second) {
                (Some(first), Some(second)) => Some(DockLayout::Split {
                    id,
                    direction,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                    zoomed: zoomed.filter(|panel| panel != panel_id),
                }),
                (Some(child), None) | (None, Some(child)) => Some(child),
                (None, None) => None,
            }
        }
    }
}

fn insert_panel(
    layout: DockLayout,
    target_node: &str,
    panel_id: String,
    target: DropTarget,
) -> Option<DockLayout> {
    let mut node_ids = HashSet::new();
    collect_node_ids(&layout, &mut node_ids);
    insert_panel_with_ids(layout, target_node, panel_id, target, &mut node_ids)
}

fn insert_panel_with_ids(
    layout: DockLayout,
    target_node: &str,
    panel_id: String,
    target: DropTarget,
    node_ids: &mut HashSet<String>,
) -> Option<DockLayout> {
    match layout {
        DockLayout::Tabs {
            id,
            mut panels,
            active,
            zoomed,
        } if id == target_node => match target {
            DropTarget::Center => {
                panels.push(panel_id.clone());
                Some(DockLayout::Tabs {
                    id,
                    panels,
                    active: Some(panel_id),
                    zoomed,
                })
            }
            DropTarget::Left | DropTarget::Right | DropTarget::Top | DropTarget::Bottom => {
                let direction = match target {
                    DropTarget::Left | DropTarget::Right => DockDirection::Horizontal,
                    _ => DockDirection::Vertical,
                };
                let new_tabs = DockLayout::Tabs {
                    id: fresh_node_id(node_ids, &id, "tabs"),
                    panels: vec![panel_id],
                    active: None,
                    zoomed: None,
                };
                let old_tabs = DockLayout::Tabs {
                    id: id.clone(),
                    panels,
                    active,
                    zoomed,
                };
                let (first, second) = match target {
                    DropTarget::Left | DropTarget::Top => (new_tabs, old_tabs),
                    _ => (old_tabs, new_tabs),
                };
                Some(DockLayout::Split {
                    id: fresh_node_id(node_ids, &id, "split"),
                    direction,
                    ratio: 0.5,
                    first: Box::new(first),
                    second: Box::new(second),
                    zoomed: None,
                })
            }
        },
        DockLayout::Split {
            id,
            direction,
            ratio,
            first,
            second,
            zoomed,
        } => {
            if contains_node(&first, target_node) {
                Some(DockLayout::Split {
                    id,
                    direction,
                    ratio,
                    first: Box::new(insert_panel_with_ids(
                        *first,
                        target_node,
                        panel_id,
                        target,
                        node_ids,
                    )?),
                    second,
                    zoomed,
                })
            } else if contains_node(&second, target_node) {
                Some(DockLayout::Split {
                    id,
                    direction,
                    ratio,
                    first,
                    second: Box::new(insert_panel_with_ids(
                        *second,
                        target_node,
                        panel_id,
                        target,
                        node_ids,
                    )?),
                    zoomed,
                })
            } else {
                None
            }
        }
        DockLayout::Tabs { .. } => None,
    }
}

fn contains_node(layout: &DockLayout, node_id: &str) -> bool {
    match layout {
        DockLayout::Tabs { id, .. } => id == node_id,
        DockLayout::Split {
            id, first, second, ..
        } => id == node_id || contains_node(first, node_id) || contains_node(second, node_id),
    }
}

fn contains_tabs_node(layout: &DockLayout, node_id: &str) -> bool {
    match layout {
        DockLayout::Tabs { id, .. } => id == node_id,
        DockLayout::Split { first, second, .. } => {
            contains_tabs_node(first, node_id) || contains_tabs_node(second, node_id)
        }
    }
}

fn fresh_node_id(node_ids: &mut HashSet<String>, seed: &str, kind: &str) -> String {
    let prefix = format!("{seed}:{kind}");
    let mut ordinal = 1_u64;
    loop {
        let candidate = format!("{prefix}:{ordinal}");
        if node_ids.insert(candidate.clone()) {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn collect_node_ids(layout: &DockLayout, ids: &mut HashSet<String>) {
    match layout {
        DockLayout::Tabs { id, .. } => {
            ids.insert(id.clone());
        }
        DockLayout::Split {
            id, first, second, ..
        } => {
            ids.insert(id.clone());
            collect_node_ids(first, ids);
            collect_node_ids(second, ids);
        }
    }
}

fn set_ratio(layout: &mut DockLayout, node_id: &str, ratio: f32) -> bool {
    match layout {
        DockLayout::Split {
            id,
            ratio: current,
            first,
            second,
            ..
        } if id == node_id => {
            *current = ratio.clamp(0.1, 0.9);
            true
        }
        DockLayout::Split { first, second, .. } => {
            set_ratio(first, node_id, ratio) || set_ratio(second, node_id, ratio)
        }
        DockLayout::Tabs { .. } => false,
    }
}

fn ratio_for(layout: &DockLayout, node_id: &str) -> Option<f32> {
    match layout {
        DockLayout::Split {
            id,
            ratio,
            first,
            second,
            ..
        } => (id == node_id)
            .then_some(*ratio)
            .or_else(|| ratio_for(first, node_id))
            .or_else(|| ratio_for(second, node_id)),
        DockLayout::Tabs { .. } => None,
    }
}

fn contains_panel(layout: &DockLayout, panel_id: &str) -> bool {
    match layout {
        DockLayout::Tabs { panels, .. } => panels.iter().any(|panel| panel == panel_id),
        DockLayout::Split { first, second, .. } => {
            contains_panel(first, panel_id) || contains_panel(second, panel_id)
        }
    }
}

fn set_zoomed(layout: &mut DockLayout, panel_id: &str) -> bool {
    if !contains_panel(layout, panel_id) {
        return false;
    }
    match layout {
        DockLayout::Tabs { zoomed, .. } | DockLayout::Split { zoomed, .. } => {
            *zoomed = (zoomed.as_deref() != Some(panel_id)).then(|| panel_id.to_string());
        }
    }
    true
}

fn zoomed_panel(layout: &DockLayout) -> Option<&str> {
    match layout {
        DockLayout::Tabs { zoomed, .. } | DockLayout::Split { zoomed, .. } => zoomed.as_deref(),
    }
}

fn zoom_count(layout: &DockLayout) -> usize {
    match layout {
        DockLayout::Tabs { zoomed, .. } => usize::from(zoomed.is_some()),
        DockLayout::Split {
            zoomed,
            first,
            second,
            ..
        } => usize::from(zoomed.is_some()) + zoom_count(first) + zoom_count(second),
    }
}

fn zoomed_tabs(layout: &DockLayout, panel_id: &str) -> Option<DockLayout> {
    match layout {
        DockLayout::Tabs { id, panels, .. } if panels.iter().any(|panel| panel == panel_id) => {
            Some(DockLayout::Tabs {
                id: id.clone(),
                panels: vec![panel_id.to_string()],
                active: Some(panel_id.to_string()),
                zoomed: None,
            })
        }
        DockLayout::Split { first, second, .. } => {
            zoomed_tabs(first, panel_id).or_else(|| zoomed_tabs(second, panel_id))
        }
        DockLayout::Tabs { .. } => None,
    }
}

fn drop_target(bounds: Bounds<Pixels>, position: Point<Pixels>) -> DropTarget {
    let x = f32::from(position.x - bounds.left()) / f32::from(bounds.size.width).max(1.0);
    let y = f32::from(position.y - bounds.top()) / f32::from(bounds.size.height).max(1.0);
    if x <= DROP_BAND {
        DropTarget::Left
    } else if x >= 1.0 - DROP_BAND {
        DropTarget::Right
    } else if y <= DROP_BAND {
        DropTarget::Top
    } else if y >= 1.0 - DROP_BAND {
        DropTarget::Bottom
    } else {
        DropTarget::Center
    }
}

fn live_destination_preview_bounds(
    drag: &DragState,
    tab_group_bounds: &[TabGroupBounds],
) -> Option<Bounds<Pixels>> {
    (!drag.target_node.is_empty())
        .then(|| {
            tab_group_bounds
                .iter()
                .find(|entry| entry.node_id == drag.target_node)
                .map(|entry| entry.bounds)
        })
        .flatten()
}

fn serialize(layout: &DockLayout) -> String {
    serde_json::to_string(layout).expect("DockLayout is always serializable")
}

impl DockWorkspaceElement {
    fn normalize_requested_layout(&self) {
        let mut state = self.state.borrow_mut();
        if !has_unique_panel_ids(&state.panel_ids) {
            state.layout = None;
            return;
        }
        let allowed = allowed_panels(&state);
        state.layout = state
            .requested_layout
            .clone()
            .and_then(|layout| normalized(layout, &allowed));
    }
}

impl CustomElement for DockWorkspaceElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        window: &mut Window,
        cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        use gpui::prelude::*;
        let state = self.state.clone();
        {
            let mut state = state.borrow_mut();
            // These records describe one painted frame only. Keeping them
            // across a React layout replacement makes old groups look live.
            state.tab_bounds.clear();
            state.tab_group_bounds.clear();
            state.split_bounds.clear();
            state.divider_bounds.clear();
            state.control_bounds.clear();
        }
        let ordered_ids = self.state.borrow().panel_ids.clone();
        let mut panels = HashMap::new();
        if has_unique_panel_ids(&ordered_ids) {
            for (index, child) in ctx.children.into_iter().enumerate() {
                if let Some(id) = ordered_ids.get(index) {
                    panels.insert(id.clone(), child);
                }
            }
        }
        let content = self
            .state
            .borrow()
            .layout
            .clone()
            .map(|layout| {
                let visible = zoomed_panel(&layout)
                    .and_then(|panel_id| zoomed_tabs(&layout, panel_id))
                    .unwrap_or_else(|| layout.clone());
                build_layout(
                    &visible,
                    &mut panels,
                    &state,
                    ctx.id,
                    ctx.event_callback,
                    ctx.events.contains("layoutChange"),
                )
            })
            .unwrap_or_else(|| gpui::Empty.into_any_element());
        let mut outer = gpui::div()
            .id((
                gpui::ElementId::from(("dock-workspace", ctx.id)),
                ctx.id.to_string(),
            ))
            .relative()
            .flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden();
        if let Some(style) = ctx.style {
            outer = crate::renderer::apply_styles(outer, style);
        }
        if let Some(handle) = ctx.focus_handle {
            outer = outer.track_focus(handle);
            let pending_focus = self.state.borrow().pending_focus.clone();
            if let Some(panel_id) = pending_focus {
                if self
                    .state
                    .borrow_mut()
                    .layout
                    .as_mut()
                    .is_some_and(|layout| activate_panel(layout, &panel_id))
                {
                    self.state.borrow_mut().pending_focus = None;
                    window.focus(handle, cx);
                }
            }
        }
        if ctx.events.contains("keyDown") {
            let callback = ctx.event_callback.clone();
            let workspace_id = ctx.id;
            outer = outer.on_key_down(move |event, _window, _cx| {
                crate::renderer::emit_event_full(&callback, workspace_id, "keyDown", |payload| {
                    payload.key = Some(event.keystroke.key.clone());
                    payload.key_char = event.keystroke.key_char.clone();
                    payload.is_held = Some(event.is_held);
                    payload.modifiers = Some(event.keystroke.modifiers.into());
                });
            });
        }
        outer
            .child(content)
            .child(
                gpui::div()
                    .absolute()
                    .size_full()
                    .child(DockInteractionLayer {
                        state,
                        id: ctx.id,
                        emits_resize: ctx.events.contains("layoutChange"),
                        callback: ctx.event_callback.clone(),
                    }),
            )
            .into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "layout" => {
                self.state.borrow_mut().requested_layout = serde_json::from_value(value).ok();
                self.normalize_requested_layout();
            }
            "panelIds" => {
                let ids = value
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                self.state.borrow_mut().panel_ids = ids;
                self.normalize_requested_layout();
            }
            "labels" => {
                self.state.borrow_mut().labels = serde_json::from_value(value).unwrap_or_default()
            }
            "closable" => {
                self.state.borrow_mut().closable =
                    serde_json::from_value::<HashMap<String, bool>>(value)
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|(id, closable)| closable.then_some(id))
                        .collect();
            }
            "focusPanelId" => {
                self.focus_panel_id = value.as_str().map(str::to_string);
                self.state.borrow_mut().pending_focus = self.focus_panel_id.clone();
            }
            _ => {}
        }
    }
    fn supported_props(&self) -> &'static [&'static str] {
        &[
            "layout",
            "panelIds",
            "labels",
            "closable",
            "focusPanelId",
            "accessibilityRole",
            "accessibilityName",
        ]
    }
    fn supported_events(&self) -> &'static [&'static str] {
        &["layoutChange", "keyDown"]
    }
    fn destroy(&mut self) {
        let mut state = self.state.borrow_mut();
        state.interaction = None;
        state.drag = None;
    }
    fn destroy_with_window(&mut self, window: &mut Window) {
        if self.state.borrow().drag.is_some() || self.state.borrow().interaction.is_some() {
            window.release_pointer();
        }
        self.destroy();
    }
}

fn apply_control(
    state: &Rc<RefCell<DockState>>,
    control: DockControl,
    workspace_id: u64,
    callback: &Option<crate::renderer::EventCallback>,
    emits_resize: bool,
) {
    let mut guard = state.borrow_mut();
    let before = guard.layout.clone();
    match control {
        DockControl::Close(panel_id) => {
            if let Some(next) = guard
                .layout
                .clone()
                .and_then(|layout| remove_panel(layout, &panel_id))
            {
                guard.layout = Some(next);
            }
        }
        DockControl::ToggleZoom(panel_id) => {
            if let Some(layout) = guard.layout.as_mut() {
                set_zoomed(layout, &panel_id);
            }
        }
    }
    if emits_resize && guard.layout != before {
        if let Some(layout) = guard.layout.as_ref() {
            crate::renderer::emit_event_full(callback, workspace_id, "layoutChange", |payload| {
                payload.layout = Some(serialize(layout))
            });
        }
    }
}

fn begin_interaction(
    state: &Rc<RefCell<DockState>>,
    position: Point<Pixels>,
    workspace_id: u64,
    callback: &Option<crate::renderer::EventCallback>,
    emits_resize: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let (bounds, hitbox) = {
        let state = state.borrow();
        (state.bounds, state.interaction_hitbox)
    };
    let (Some(bounds), Some(hitbox)) = (bounds, hitbox) else {
        return;
    };
    if !bounds.contains(&position) {
        return;
    }
    let control = state
        .borrow()
        .control_bounds
        .iter()
        .find(|entry| entry.bounds.contains(&position))
        .map(|entry| entry.control.clone());
    if let Some(control) = control {
        apply_control(state, control, workspace_id, callback, emits_resize);
        window.refresh();
        cx.stop_propagation();
        return;
    }
    let divider = state
        .borrow()
        .divider_bounds
        .iter()
        .find(|entry| entry.bounds.contains(&position))
        .cloned();
    if let Some(divider) = divider {
        let split_bounds = state
            .borrow()
            .split_bounds
            .iter()
            .find(|entry| entry.node_id == divider.node_id)
            .map(|entry| entry.bounds);
        let initial_ratio = state
            .borrow()
            .layout
            .as_ref()
            .and_then(|layout| ratio_for(layout, &divider.node_id));
        if let (Some(bounds), Some(initial_ratio)) = (split_bounds, initial_ratio) {
            state.borrow_mut().interaction = Some(Interaction::Resize(ResizeState {
                node_id: divider.node_id,
                direction: divider.direction,
                bounds,
                initial_ratio,
            }));
            window.capture_pointer(hitbox);
            window.refresh();
            cx.stop_propagation();
            return;
        }
    }
    let source = state
        .borrow()
        .tab_bounds
        .iter()
        .find(|entry| entry.bounds.contains(&position))
        .cloned();
    if let Some(source) = source {
        state.borrow_mut().drag = Some(DragState {
            panel_id: source.panel_id,
            source_node: source.node_id,
            preview: DropTarget::Center,
            target_node: String::new(),
            position,
        });
        window.capture_pointer(hitbox);
        window.refresh();
        cx.stop_propagation();
    }
}

fn build_layout(
    layout: &DockLayout,
    panels: &mut HashMap<String, gpui::AnyElement>,
    state: &Rc<RefCell<DockState>>,
    workspace_id: u64,
    callback: &Option<crate::renderer::EventCallback>,
    emits_resize: bool,
) -> gpui::AnyElement {
    use gpui::prelude::*;
    match layout {
        DockLayout::Split {
            id,
            direction,
            ratio,
            first,
            second,
            ..
        } => {
            let first = build_layout(first, panels, state, workspace_id, callback, emits_resize);
            let second = build_layout(second, panels, state, workspace_id, callback, emits_resize);
            let mut root = direction.flex(gpui::div().flex().size_full().min_w_0().min_h_0());
            match direction {
                DockDirection::Horizontal => {
                    root = root
                        .child(
                            gpui::div()
                                .w(gpui::relative(*ratio))
                                .min_w_0()
                                .min_h_0()
                                .child(first),
                        )
                        .child(gpui::div().w(px(1.0)).child(DividerBoundsTracker {
                            state: state.clone(),
                            node_id: id.clone(),
                            direction: *direction,
                        }))
                        .child(gpui::div().flex_grow(1.0).min_w_0().min_h_0().child(second));
                }
                DockDirection::Vertical => {
                    root = root
                        .child(
                            gpui::div()
                                .h(gpui::relative(*ratio))
                                .min_w_0()
                                .min_h_0()
                                .child(first),
                        )
                        .child(gpui::div().h(px(1.0)).child(DividerBoundsTracker {
                            state: state.clone(),
                            node_id: id.clone(),
                            direction: *direction,
                        }))
                        .child(gpui::div().flex_grow(1.0).min_w_0().min_h_0().child(second));
                }
            }
            root.relative()
                .child(
                    gpui::div()
                        .absolute()
                        .size_full()
                        .child(SplitBoundsTracker {
                            state: state.clone(),
                            node_id: id.clone(),
                        }),
                )
                .into_any_element()
        }
        DockLayout::Tabs {
            id,
            panels: panel_ids,
            active,
            ..
        } => {
            let active = active.as_ref().or_else(|| panel_ids.first());
            let mut headers = gpui::div()
                .flex()
                .flex_row()
                .h(px(TAB_HEIGHT))
                .flex_none()
                .overflow_hidden();
            for panel_id in panel_ids {
                let label = state
                    .borrow()
                    .labels
                    .get(panel_id)
                    .cloned()
                    .unwrap_or_else(|| panel_id.clone());
                let is_active = active == Some(panel_id);
                let tab_state = state.clone();
                let tab_id = id.clone();
                let panel = panel_id.clone();
                let tab_callback = callback.clone();
                let closable = state.borrow().closable.contains(panel_id);
                headers = headers.child(
                    gpui::div()
                        .id((
                            gpui::ElementId::from(("dock-tab", workspace_id)),
                            format!("{id}:{panel_id}"),
                        ))
                        .h_full()
                        .relative()
                        .px_2()
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .bg(if is_active {
                            gpui::rgba(0x303746ff)
                        } else {
                            gpui::rgba(0x20242cff)
                        })
                        .on_click(move |_event, _window, _cx| {
                            let mut guard = tab_state.borrow_mut();
                            let before = guard.layout.clone();
                            if let Some(layout) = guard.layout.as_mut() {
                                activate(layout, &tab_id, &panel);
                            }
                            if emits_resize && guard.layout != before {
                                if let Some(layout) = guard.layout.as_ref() {
                                    crate::renderer::emit_event_full(
                                        &tab_callback,
                                        workspace_id,
                                        "layoutChange",
                                        |payload| payload.layout = Some(serialize(layout)),
                                    );
                                }
                            }
                        })
                        .child(crate::text::chrome_text(label.into(), None))
                        .when(closable, |tab| {
                            tab.child(
                                gpui::div()
                                    .id((
                                        gpui::ElementId::from(("dock-close", workspace_id)),
                                        format!("{id}:{panel_id}"),
                                    ))
                                    .ml_1()
                                    .px_1()
                                    .relative()
                                    .cursor_pointer()
                                    .child(crate::text::chrome_text("×".into(), None))
                                    .child(gpui::div().absolute().size_full().child(
                                        ControlBoundsTracker {
                                            state: state.clone(),
                                            control: DockControl::Close(panel_id.clone()),
                                        },
                                    )),
                            )
                        })
                        .child({
                            gpui::div()
                                .id((
                                    gpui::ElementId::from(("dock-zoom", workspace_id)),
                                    format!("{id}:{panel_id}"),
                                ))
                                .ml_1()
                                .px_1()
                                .relative()
                                .cursor_pointer()
                                .child(crate::text::chrome_text("↗".into(), None))
                                .child(gpui::div().absolute().size_full().child(
                                    ControlBoundsTracker {
                                        state: state.clone(),
                                        control: DockControl::ToggleZoom(panel_id.clone()),
                                    },
                                ))
                        })
                        .child(gpui::div().absolute().size_full().child(TabBoundsTracker {
                            state: state.clone(),
                            node_id: id.clone(),
                            panel_id: panel_id.clone(),
                        })),
                );
            }
            let body = active
                .and_then(|panel_id| panels.remove(panel_id))
                .unwrap_or_else(|| gpui::Empty.into_any_element());
            gpui::div()
                .relative()
                .flex()
                .flex_col()
                .size_full()
                .min_w_0()
                .min_h_0()
                .child(headers)
                .child(gpui::div().flex_grow(1.0).min_w_0().min_h_0().child(body))
                .child(
                    gpui::div()
                        .absolute()
                        .size_full()
                        .child(TabGroupBoundsTracker {
                            state: state.clone(),
                            node_id: id.clone(),
                        }),
                )
                .into_any_element()
        }
    }
}

struct TabBoundsTracker {
    state: Rc<RefCell<DockState>>,
    node_id: String,
    panel_id: String,
}

struct TabGroupBoundsTracker {
    state: Rc<RefCell<DockState>>,
    node_id: String,
}

impl Element for TabGroupBoundsTracker {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (
            window.request_measured_layout(style, |known, available, _, _| {
                size(
                    known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                    known.height.unwrap_or(match available.height {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                )
            }),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) -> Bounds<Pixels> {
        bounds
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) {
        let mut state = self.state.borrow_mut();
        state
            .tab_group_bounds
            .retain(|entry| entry.node_id != self.node_id);
        state.tab_group_bounds.push(TabGroupBounds {
            node_id: self.node_id.clone(),
            bounds: *bounds,
        });
    }
}

impl IntoElement for TabGroupBoundsTracker {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

struct SplitBoundsTracker {
    state: Rc<RefCell<DockState>>,
    node_id: String,
}

impl Element for SplitBoundsTracker {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (
            window.request_measured_layout(style, |known, available, _, _| {
                size(
                    known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                    known.height.unwrap_or(match available.height {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                )
            }),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) -> Bounds<Pixels> {
        bounds
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) {
        let mut state = self.state.borrow_mut();
        state
            .split_bounds
            .retain(|entry| entry.node_id != self.node_id);
        state.split_bounds.push(SplitBounds {
            node_id: self.node_id.clone(),
            bounds: *bounds,
        });
    }
}

impl IntoElement for SplitBoundsTracker {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
impl Element for TabBoundsTracker {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (
            window.request_measured_layout(style, |known, available, _, _| {
                size(
                    known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(v) => v,
                        _ => px(0.),
                    }),
                    known.height.unwrap_or(match available.height {
                        AvailableSpace::Definite(v) => v,
                        _ => px(0.),
                    }),
                )
            }),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) -> Bounds<Pixels> {
        bounds
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut Bounds<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) {
        let mut state = self.state.borrow_mut();
        state
            .tab_bounds
            .retain(|entry| !(entry.node_id == self.node_id && entry.panel_id == self.panel_id));
        state.tab_bounds.push(TabBounds {
            node_id: self.node_id.clone(),
            panel_id: self.panel_id.clone(),
            bounds: *prepaint,
        });
    }
}
impl IntoElement for TabBoundsTracker {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

struct DividerBoundsTracker {
    state: Rc<RefCell<DockState>>,
    node_id: String,
    direction: DockDirection,
}

struct ControlBoundsTracker {
    state: Rc<RefCell<DockState>>,
    control: DockControl,
}

impl Element for ControlBoundsTracker {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (
            window.request_measured_layout(style, |known, available, _, _| {
                size(
                    known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                    known.height.unwrap_or(match available.height {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                )
            }),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) -> Bounds<Pixels> {
        bounds
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) {
        self.state.borrow_mut().control_bounds.push(ControlBounds {
            control: self.control.clone(),
            bounds: *bounds,
        });
    }
}

impl IntoElement for ControlBoundsTracker {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DividerBoundsTracker {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (
            window.request_measured_layout(style, |known, available, _, _| {
                size(
                    known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                    known.height.unwrap_or(match available.height {
                        AvailableSpace::Definite(value) => value,
                        _ => px(0.0),
                    }),
                )
            }),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) -> Bounds<Pixels> {
        bounds
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        _: &mut App,
    ) {
        let hit_bounds = match self.direction {
            DockDirection::Horizontal => Bounds {
                origin: Point {
                    x: bounds.left() - px(DIVIDER_HIT_SLOP),
                    y: bounds.top(),
                },
                size: size(
                    bounds.size.width + px(DIVIDER_HIT_SLOP * 2.0),
                    bounds.size.height,
                ),
            },
            DockDirection::Vertical => Bounds {
                origin: Point {
                    x: bounds.left(),
                    y: bounds.top() - px(DIVIDER_HIT_SLOP),
                },
                size: size(
                    bounds.size.width,
                    bounds.size.height + px(DIVIDER_HIT_SLOP * 2.0),
                ),
            },
        };
        let mut state = self.state.borrow_mut();
        state
            .divider_bounds
            .retain(|entry| entry.node_id != self.node_id);
        state.divider_bounds.push(DividerBounds {
            node_id: self.node_id.clone(),
            direction: self.direction,
            bounds: hit_bounds,
        });
    }
}

impl IntoElement for DividerBoundsTracker {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

struct DockInteractionLayer {
    state: Rc<RefCell<DockState>>,
    id: u64,
    emits_resize: bool,
    callback: Option<crate::renderer::EventCallback>,
}
struct DockInteractionPrepaint {
    bounds: Bounds<Pixels>,
}
impl Element for DockInteractionLayer {
    type RequestLayoutState = ();
    type PrepaintState = DockInteractionPrepaint;
    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (
            window.request_measured_layout(style, |known, available, _, _| {
                size(
                    known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(v) => v,
                        _ => px(0.),
                    }),
                    known.height.unwrap_or(match available.height {
                        AvailableSpace::Definite(v) => v,
                        _ => px(0.),
                    }),
                )
            }),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) -> DockInteractionPrepaint {
        self.state.borrow_mut().bounds = Some(bounds);
        // The interaction layer observes tab, divider, and control geometry, but it
        // must not occlude the active panel. A blocking full-workspace hitbox makes
        // nested inputs and buttons paint correctly while preventing them from ever
        // receiving pointer focus.
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        self.state.borrow_mut().interaction_hitbox = Some(hitbox.id);
        if self.state.borrow().drag.is_some() {
            window.capture_pointer(hitbox.id);
        }
        DockInteractionPrepaint { bounds }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut DockInteractionPrepaint,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let state = self.state.clone();
        let bounds = prepaint.bounds;
        let view = window.current_view();
        let id = self.id;
        let callback = self.callback.clone();
        let emits_resize = self.emits_resize;
        window.on_mouse_event({
            let state = state.clone();
            let callback = callback.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == MouseButton::Left
                    && bounds.contains(&event.position)
                {
                    begin_interaction(
                        &state,
                        event.position,
                        id,
                        &callback,
                        emits_resize,
                        window,
                        cx,
                    );
                }
            }
        });
        window.on_mouse_event({
            let state = state.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                let resize = match state.borrow().interaction.clone() {
                    Some(Interaction::Resize(resize)) => Some(resize),
                    _ => None,
                };
                if phase == DispatchPhase::Bubble && resize.is_some() {
                    let resize = resize.expect("checked above");
                    let (origin, extent, coordinate) = match resize.direction {
                        DockDirection::Horizontal => (
                            resize.bounds.left(),
                            resize.bounds.size.width,
                            event.position.x,
                        ),
                        DockDirection::Vertical => (
                            resize.bounds.top(),
                            resize.bounds.size.height,
                            event.position.y,
                        ),
                    };
                    let ratio = f32::from(coordinate - origin) / f32::from(extent).max(1.0);
                    if let Some(layout) = state.borrow_mut().layout.as_mut() {
                        set_ratio(layout, &resize.node_id, ratio);
                    }
                    cx.notify(view);
                    return;
                }
                if phase == DispatchPhase::Bubble && state.borrow().drag.is_some() {
                    if !bounds.contains(&event.position) {
                        state.borrow_mut().drag = None;
                        window.release_pointer();
                        cx.notify(view);
                        return;
                    }
                    let target = state
                        .borrow()
                        .tab_group_bounds
                        .iter()
                        .find(|entry| entry.bounds.contains(&event.position))
                        .cloned();
                    let mut guard = state.borrow_mut();
                    let drag = guard.drag.as_mut().expect("drag state exists");
                    drag.position = event.position;
                    if let Some(target) = target {
                        drag.target_node = target.node_id;
                        drag.preview = drop_target(target.bounds, event.position);
                    } else {
                        drag.target_node.clear();
                    }
                    cx.notify(view);
                }
            }
        });
        window.on_mouse_event({
            let state = state.clone();
            let callback = self.callback.clone();
            let id = self.id;
            let emits_resize = self.emits_resize;
            move |event: &MouseUpEvent, phase, window, cx| {
                let resize = match state.borrow().interaction.clone() {
                    Some(Interaction::Resize(resize)) => Some(resize),
                    _ => None,
                };
                if phase == DispatchPhase::Bubble
                    && event.button == MouseButton::Left
                    && resize.is_some()
                {
                    let resize = resize.expect("checked above");
                    state.borrow_mut().interaction = None;
                    window.release_pointer();
                    let layout = state.borrow().layout.clone();
                    if emits_resize
                        && layout
                            .as_ref()
                            .and_then(|layout| ratio_for(layout, &resize.node_id))
                            .is_some_and(|ratio| {
                                (ratio - resize.initial_ratio).abs() > f32::EPSILON
                            })
                    {
                        let layout = layout.expect("checked above");
                        crate::renderer::emit_event_full(
                            &callback,
                            id,
                            "layoutChange",
                            |payload| {
                                payload.layout = Some(serialize(&layout));
                            },
                        );
                    }
                    cx.notify(view);
                    cx.stop_propagation();
                    return;
                }
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || state.borrow().drag.is_none()
                {
                    return;
                }
                let drag = state.borrow_mut().drag.take().unwrap();
                window.release_pointer();
                let layout = state.borrow().layout.clone();
                let destination = layout.as_ref().and_then(|layout| {
                    state
                        .borrow()
                        .tab_group_bounds
                        .iter()
                        .find(|entry| entry.bounds.contains(&event.position))
                        .filter(|entry| contains_tabs_node(layout, &entry.node_id))
                        .map(|entry| {
                            (
                                entry.node_id.clone(),
                                drop_target(entry.bounds, event.position),
                            )
                        })
                });
                let Some((destination, target)) = destination else {
                    cx.notify(view);
                    cx.stop_propagation();
                    return;
                };
                if destination == drag.source_node && target == DropTarget::Center {
                    let mut state = state.borrow_mut();
                    let before = state.layout.clone();
                    if let Some(layout) = state.layout.as_mut() {
                        activate(layout, &drag.source_node, &drag.panel_id);
                    }
                    if emits_resize && state.layout != before {
                        if let Some(layout) = state.layout.as_ref() {
                            crate::renderer::emit_event_full(
                                &callback,
                                id,
                                "layoutChange",
                                |payload| payload.layout = Some(serialize(layout)),
                            );
                        }
                    }
                    cx.notify(view);
                    cx.stop_propagation();
                    return;
                }
                let next = layout.and_then(|layout| {
                    (contains_tabs_node(&layout, &destination)
                        && contains_panel(&layout, &drag.panel_id))
                    .then_some(layout)
                    .and_then(|layout| remove_panel(layout, &drag.panel_id))
                    .and_then(|layout| insert_panel(layout, &destination, drag.panel_id, target))
                });
                let allowed = allowed_panels(&state.borrow());
                if let Some(next) = next.and_then(|layout| normalized(layout, &allowed)) {
                    state.borrow_mut().layout = Some(next.clone());
                    if emits_resize {
                        crate::renderer::emit_event_full(
                            &callback,
                            id,
                            "layoutChange",
                            |payload| payload.layout = Some(serialize(&next)),
                        );
                    }
                }
                cx.notify(view);
                cx.stop_propagation();
            }
        });
        window.on_mouse_event({
            let state = state.clone();
            move |_: &MouseExitEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && (state.borrow().drag.is_some() || state.borrow().interaction.is_some())
                {
                    let mut guard = state.borrow_mut();
                    guard.drag = None;
                    guard.interaction = None;
                    window.release_pointer();
                    cx.notify(view);
                }
            }
        });
        let drag = state.borrow().drag.clone();
        if let Some(drag) = drag {
            let color = gpui::rgba(0x7c86ff55);
            if let Some(target_bounds) =
                live_destination_preview_bounds(&drag, &state.borrow().tab_group_bounds)
            {
                let rect = match drag.preview {
                    DropTarget::Center => target_bounds,
                    DropTarget::Left => Bounds::new(
                        target_bounds.origin,
                        size(target_bounds.size.width / 2., target_bounds.size.height),
                    ),
                    DropTarget::Right => Bounds::new(
                        gpui::point(target_bounds.center().x, target_bounds.top()),
                        size(target_bounds.size.width / 2., target_bounds.size.height),
                    ),
                    DropTarget::Top => Bounds::new(
                        target_bounds.origin,
                        size(target_bounds.size.width, target_bounds.size.height / 2.),
                    ),
                    DropTarget::Bottom => Bounds::new(
                        gpui::point(target_bounds.left(), target_bounds.center().y),
                        size(target_bounds.size.width, target_bounds.size.height / 2.),
                    ),
                };
                window.paint_quad(gpui::fill(rect, color));
            }
            let ghost = Bounds::new(
                gpui::point(drag.position.x + px(8.0), drag.position.y + px(8.0)),
                size(px(96.0), px(24.0)),
            );
            window.paint_quad(gpui::fill(ghost, gpui::rgba(0x303746e6)));
        }
    }
}
impl IntoElement for DockInteractionLayer {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tabs(id: &str, panels: &[&str]) -> DockLayout {
        DockLayout::Tabs {
            id: id.into(),
            panels: panels.iter().map(|id| (*id).into()).collect(),
            active: None,
            zoomed: None,
        }
    }
    #[test]
    fn normalizes_duplicate_panels_and_collapses_empty_branches() {
        let allowed = HashSet::from(["a".into(), "b".into()]);
        let layout = DockLayout::Split {
            id: "root".into(),
            direction: DockDirection::Horizontal,
            ratio: 2.0,
            first: Box::new(tabs("one", &["a", "a", "missing"])),
            second: Box::new(tabs("two", &["missing"])),
            zoomed: None,
        };
        assert_eq!(normalized(layout, &allowed), None);
    }
    #[test]
    fn drop_creates_a_stable_split_without_losing_existing_panel() {
        let layout = insert_panel(tabs("root", &["a"]), "root", "b".into(), DropTarget::Right)
            .expect("live tab target accepts the panel");
        let json = serialize(&layout);
        assert!(json.contains("\"a\"") && json.contains("\"b\"") && json.contains("\"split\""));
    }

    #[test]
    fn rejects_duplicate_panels_across_tab_groups_without_discarding_a_branch() {
        let layout = DockLayout::Split {
            id: "root".into(),
            direction: DockDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(tabs("left", &["a"])),
            second: Box::new(tabs("right", &["a"])),
            zoomed: None,
        };
        assert_eq!(normalized(layout, &HashSet::from(["a".to_string()])), None,);
    }

    #[test]
    fn repeated_edge_docking_creates_collision_free_node_ids() {
        let first = insert_panel(tabs("root", &["a"]), "root", "b".into(), DropTarget::Right)
            .expect("root is a live tab target");
        let second = insert_panel(first, "root", "c".into(), DropTarget::Right)
            .expect("root remains a live tab target");
        let mut ids = HashSet::new();
        collect_node_ids(&second, &mut ids);
        assert_eq!(ids.len(), 5);
        assert!(
            normalized(second, &HashSet::from(["a".into(), "b".into(), "c".into()]),).is_some()
        );
    }

    #[test]
    fn close_collapses_a_split_and_zoom_is_serialized() {
        let layout = DockLayout::Split {
            id: "root".into(),
            direction: DockDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(tabs("left", &["a"])),
            second: Box::new(tabs("right", &["b"])),
            zoomed: None,
        };
        let mut layout = remove_panel(layout, "b").expect("a remains");
        assert!(matches!(layout, DockLayout::Tabs { .. }));
        assert!(set_zoomed(&mut layout, "a"));
        assert!(serialize(&layout).contains("\"zoomed\":\"a\""));
    }

    #[test]
    fn rejects_non_root_zoom_state() {
        let mut child = tabs("child", &["a"]);
        set_zoomed(&mut child, "a");
        let layout = DockLayout::Split {
            id: "root".into(),
            direction: DockDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(child),
            second: Box::new(tabs("other", &["b"])),
            zoomed: None,
        };
        assert!(normalized(layout, &HashSet::from(["a".into(), "b".into()])).is_none());
    }

    #[test]
    fn edge_target_uses_the_destination_bounds() {
        let bounds = Bounds::new(
            gpui::point(px(100.0), px(100.0)),
            size(px(200.0), px(100.0)),
        );
        assert_eq!(
            drop_target(bounds, gpui::point(px(110.0), px(150.0))),
            DropTarget::Left
        );
        assert_eq!(
            drop_target(bounds, gpui::point(px(200.0), px(150.0))),
            DropTarget::Center
        );
    }

    #[test]
    fn invalid_drag_target_has_no_destination_preview_bounds() {
        let drag = DragState {
            panel_id: "a".into(),
            source_node: "left".into(),
            preview: DropTarget::Center,
            target_node: "stale".into(),
            position: gpui::point(px(400.0), px(180.0)),
        };
        let live = [TabGroupBounds {
            node_id: "right".into(),
            bounds: Bounds::new(gpui::point(px(400.0), px(0.0)), size(px(400.0), px(400.0))),
        }];

        assert!(live_destination_preview_bounds(&drag, &[]).is_none());
        assert!(live_destination_preview_bounds(&drag, &live).is_none());
    }
}
