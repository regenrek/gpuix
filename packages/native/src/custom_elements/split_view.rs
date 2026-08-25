//! Native two-pane split view. Drag geometry stays in GPUI; React sees only a
//! final resize event after mouse release.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    px, relative, size, App, AvailableSpace, Bounds, CursorStyle, DispatchPhase, Element, GlobalElementId,
    Hitbox, HitboxBehavior, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, Style, Window,
};

use super::{CustomElement, CustomElementFactory, CustomRenderContext};

pub struct SplitViewFactory;

impl CustomElementFactory for SplitViewFactory {
    fn element_type(&self) -> &str {
        "split-view"
    }

    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(SplitViewElement::default())
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

impl Direction {
    fn from_prop(value: &str) -> Self {
        match value {
            "vertical" => Self::Vertical,
            _ => Self::Horizontal,
        }
    }

    fn cursor(self) -> CursorStyle {
        match self {
            Self::Horizontal => CursorStyle::ResizeLeftRight,
            Self::Vertical => CursorStyle::ResizeUpDown,
        }
    }

    fn coordinate(self, point: Point<Pixels>) -> Pixels {
        match self {
            Self::Horizontal => point.x,
            Self::Vertical => point.y,
        }
    }

    fn extent(self, bounds: Bounds<Pixels>) -> Pixels {
        match self {
            Self::Horizontal => bounds.size.width,
            Self::Vertical => bounds.size.height,
        }
    }

    fn origin(self, bounds: Bounds<Pixels>) -> Pixels {
        match self {
            Self::Horizontal => bounds.left(),
            Self::Vertical => bounds.top(),
        }
    }
}

#[derive(Clone, Copy)]
struct SplitConfig {
    direction: Direction,
    min_size: f32,
    min_second_size: f32,
    divider_size: f32,
}

/// The single source of truth for a split's primary-axis geometry. The divider
/// is removed before either pane minimum is applied, so layout, hit testing,
/// and drag updates all describe the same two panes. When both declared
/// minima do not fit, both remain hard minimums and the second pane overflows
/// the trailing edge; the outer split clips that overflow.
#[derive(Clone, Copy)]
struct SplitGeometry {
    ratio: f32,
    primary: f32,
    secondary: f32,
}

impl SplitGeometry {
    fn new(config: SplitConfig, bounds: Bounds<Pixels>, requested_ratio: f32) -> Self {
        let available = (f32::from(config.direction.extent(bounds)) - config.divider_size).max(0.0);
        let requested_primary = requested_ratio.clamp(0.0, 1.0) * available;
        let (primary, secondary) = if config.min_size + config.min_second_size <= available {
            let primary = requested_primary.clamp(config.min_size, available - config.min_second_size);
            (primary, available - primary)
        } else {
            // Do not silently shrink a declared minimum. Keep the divider in
            // the parent axis and let the trailing pane be clipped instead.
            let primary = requested_primary.max(config.min_size);
            (primary, (available - primary).max(config.min_second_size))
        };
        Self {
            ratio: if available > 0.0 { primary / available } else { 0.0 },
            primary,
            secondary,
        }
    }
}

#[derive(Default)]
struct DragState {
    active: bool,
    ratio: f32,
    bounds: Option<Bounds<Pixels>>,
}

pub struct SplitViewElement {
    direction: Direction,
    min_size: f32,
    min_second_size: f32,
    divider_size: f32,
    state: Rc<RefCell<DragState>>,
    initialized: bool,
}

impl Default for SplitViewElement {
    fn default() -> Self {
        Self {
            direction: Direction::Horizontal,
            min_size: 0.0,
            min_second_size: 0.0,
            divider_size: 1.0,
            state: Rc::new(RefCell::new(DragState { active: false, ratio: 0.5, bounds: None })),
            initialized: false,
        }
    }
}

impl SplitViewElement {
    fn config(&self) -> SplitConfig {
        SplitConfig {
            direction: self.direction,
            min_size: self.min_size.max(0.0),
            min_second_size: self.min_second_size.max(0.0),
            divider_size: self.divider_size.max(1.0),
        }
    }
}

impl CustomElement for SplitViewElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        use gpui::prelude::*;

        let config = self.config();
        // A previous prepaint gives us the current definite split extent. Keep
        // the stored ratio canonical before building the panes, including for
        // controlled/default updates and a later bounds change.
        let previous_bounds = self.state.borrow().bounds;
        let geometry = previous_bounds.map(|bounds| {
            let mut state = self.state.borrow_mut();
            let geometry = SplitGeometry::new(config, bounds, state.ratio);
            state.ratio = geometry.ratio;
            geometry
        });
        let ratio = geometry.map_or(self.state.borrow().ratio, |geometry| geometry.ratio);
        // Under an undersized axis SplitGeometry deliberately keeps both
        // declared minima. Clip the deterministic trailing overflow here.
        let mut outer = gpui::div().relative().flex().min_w_0().min_h_0().overflow_hidden();
        if config.direction == Direction::Horizontal {
            outer = outer.flex_row();
        } else {
            outer = outer.flex_col();
        }
        if let Some(style) = ctx.style {
            outer = crate::renderer::apply_styles(outer, style);
        }

        let mut children = ctx.children.into_iter();
        let first = children.next().unwrap_or_else(|| gpui::Empty.into_any_element());
        let second = children.next().unwrap_or_else(|| gpui::Empty.into_any_element());
        let (first, divider, second) = match (config.direction, geometry) {
            (Direction::Horizontal, Some(geometry)) => (
                gpui::div().w(px(geometry.primary)).flex_none().min_w(px(config.min_size.min(geometry.primary))).min_h_0().flex().flex_col().child(first),
                gpui::div().w(px(config.divider_size)).flex_none().min_h_0(),
                gpui::div().w(px(geometry.secondary)).flex_none().min_w(px(config.min_second_size.min(geometry.secondary))).min_h_0().flex().flex_col().child(second),
            ),
            (Direction::Vertical, Some(geometry)) => (
                gpui::div().h(px(geometry.primary)).flex_none().min_h(px(config.min_size.min(geometry.primary))).min_w_0().flex().flex_col().child(first),
                gpui::div().h(px(config.divider_size)).flex_none().min_w_0(),
                gpui::div().h(px(geometry.secondary)).flex_none().min_h(px(config.min_second_size.min(geometry.secondary))).min_w_0().flex().flex_col().child(second),
            ),
            (Direction::Horizontal, None) if ratio <= 0.0 => (
                gpui::div().w(px(config.min_size)).flex_none().min_h_0().child(first),
                gpui::div().w(px(config.divider_size)).flex_none().min_h_0(),
                gpui::div().flex_grow(1.0).min_w(px(config.min_second_size)).min_h_0().child(second),
            ),
            (Direction::Horizontal, None) if ratio >= 1.0 => (
                gpui::div().flex_grow(1.0).min_w(px(config.min_size)).min_h_0().child(first),
                gpui::div().w(px(config.divider_size)).flex_none().min_h_0(),
                gpui::div().w(px(config.min_second_size)).flex_none().min_h_0().child(second),
            ),
            (Direction::Horizontal, None) => (
                gpui::div().w(px(config.min_size)).flex_grow(ratio).flex_shrink(1.0).min_w(px(config.min_size)).min_h_0().child(first),
                gpui::div().w(px(config.divider_size)).flex_none().min_h_0(),
                gpui::div().w(px(config.min_second_size)).flex_grow((1.0 - ratio).max(0.0)).flex_shrink(1.0).min_w(px(config.min_second_size)).min_h_0().child(second),
            ),
            (Direction::Vertical, None) if ratio <= 0.0 => (
                gpui::div().h(px(config.min_size)).flex_none().min_w_0().child(first),
                gpui::div().h(px(config.divider_size)).flex_none().min_w_0(),
                gpui::div().flex_grow(1.0).min_h(px(config.min_second_size)).min_w_0().child(second),
            ),
            (Direction::Vertical, None) if ratio >= 1.0 => (
                gpui::div().flex_grow(1.0).min_h(px(config.min_size)).min_w_0().child(first),
                gpui::div().h(px(config.divider_size)).flex_none().min_w_0(),
                gpui::div().h(px(config.min_second_size)).flex_none().min_w_0().child(second),
            ),
            (Direction::Vertical, None) => (
                gpui::div().h(px(config.min_size)).flex_grow(ratio).flex_shrink(1.0).min_h(px(config.min_size)).min_w_0().child(first),
                gpui::div().h(px(config.divider_size)).flex_none().min_w_0(),
                gpui::div().h(px(config.min_second_size)).flex_grow((1.0 - ratio).max(0.0)).flex_shrink(1.0).min_h(px(config.min_second_size)).min_w_0().child(second),
            ),
        };
        let overlay = SplitHandleElement {
            state: self.state.clone(),
            config,
            id: ctx.id,
            emits_resize: ctx.events.contains("resize"),
            callback: ctx.event_callback.clone(),
        };
        outer
            .child(first)
            .child(divider)
            .child(second)
            .child(
                gpui::div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .left_0()
                    .child(overlay),
            )
            .into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "direction" => self.direction = value.as_str().map(Direction::from_prop).unwrap_or_default(),
            "ratio" => {
                if let Some(ratio) = value.as_f64().filter(|ratio| ratio.is_finite()) {
                    self.state.borrow_mut().ratio = (ratio as f32).clamp(0.0, 1.0);
                    self.initialized = true;
                }
            }
            "defaultRatio" if !self.initialized => {
                if let Some(ratio) = value.as_f64().filter(|ratio| ratio.is_finite()) {
                    self.state.borrow_mut().ratio = (ratio as f32).clamp(0.0, 1.0);
                    self.initialized = true;
                }
            }
            "minSize" => self.min_size = value.as_f64().unwrap_or(0.0) as f32,
            "minSecondSize" => self.min_second_size = value.as_f64().unwrap_or(0.0) as f32,
            "dividerSize" => self.divider_size = value.as_f64().unwrap_or(1.0) as f32,
            _ => {}
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        &["direction", "ratio", "defaultRatio", "minSize", "minSecondSize", "dividerSize"]
    }

    fn supported_events(&self) -> &'static [&'static str] {
        &["resize"]
    }

    fn destroy(&mut self) {
        let mut state = self.state.borrow_mut();
        state.active = false;
        state.bounds = None;
    }

    fn destroy_with_window(&mut self, window: &mut Window) {
        if self.state.borrow().active {
            window.release_pointer();
        }
        self.destroy();
    }
}

struct SplitHandleElement {
    state: Rc<RefCell<DragState>>,
    config: SplitConfig,
    id: u64,
    emits_resize: bool,
    callback: Option<crate::renderer::EventCallback>,
}

struct SplitHandlePrepaint {
    bounds: Bounds<Pixels>,
    hitbox: Hitbox,
}

impl SplitHandleElement {
    fn divider_bounds(&self, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        let extent = f32::from(self.config.direction.extent(bounds));
        let origin = f32::from(self.config.direction.origin(bounds));
        let ratio = SplitGeometry::new(self.config, bounds, self.state.borrow().ratio).ratio;
        let divider_origin = origin + (extent - self.config.divider_size).max(0.0) * ratio;
        match self.config.direction {
            Direction::Horizontal => Bounds::new(
                gpui::point(px(divider_origin), bounds.top()),
                size(px(self.config.divider_size), bounds.size.height),
            ),
            Direction::Vertical => Bounds::new(
                gpui::point(bounds.left(), px(divider_origin)),
                size(bounds.size.width, px(self.config.divider_size)),
            ),
        }
    }

}

impl Element for SplitHandleElement {
    type RequestLayoutState = ();
    type PrepaintState = SplitHandlePrepaint;

    fn id(&self) -> Option<gpui::ElementId> { None }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> { None }

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
        let layout = window.request_measured_layout(style, |known, available, _, _| {
            let width = known.width.unwrap_or(match available.width { AvailableSpace::Definite(width) => width, _ => px(0.0) });
            let height = known.height.unwrap_or(match available.height { AvailableSpace::Definite(height) => height, _ => px(0.0) });
            size(width, height)
        });
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> SplitHandlePrepaint {
        let (active, geometry_changed) = {
            let mut state = self.state.borrow_mut();
            if state.active && state.bounds.is_some_and(|previous| previous != bounds) {
                state.active = false;
                window.release_pointer();
            }
            let ratio = SplitGeometry::new(self.config, bounds, state.ratio).ratio;
            let geometry_changed = state.ratio != ratio;
            state.ratio = ratio;
            state.bounds = Some(bounds);
            (state.active, geometry_changed)
        };
        // Initial/default and controlled ratios only gain a definite extent
        // during prepaint. Schedule exactly one native follow-up frame when
        // canonicalization changes it so the panes never retain raw geometry.
        if geometry_changed {
            cx.notify(window.current_view());
        }
        let divider_bounds = self.divider_bounds(bounds);
        let hitbox = window.insert_hitbox(divider_bounds, HitboxBehavior::BlockMouse);
        if active {
            // GPUI assigns hitbox IDs per frame. Rebind the native capture so
            // subsequent move/up listeners observe this frame's hitbox.
            window.capture_pointer(hitbox.id);
        }
        SplitHandlePrepaint { bounds, hitbox }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut SplitHandlePrepaint,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let state = self.state.clone();
        let config = self.config;
        let hitbox = prepaint.hitbox.clone();
        let bounds = prepaint.bounds;
        if hitbox.is_hovered(window) || state.borrow().active {
            window.set_cursor_style(config.direction.cursor(), &hitbox);
        }
        let current_view = window.current_view();
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let state = state.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && event.button == MouseButton::Left && hitbox.is_hovered(window) {
                    let mut state = state.borrow_mut();
                    state.active = true;
                    state.bounds = Some(bounds);
                    window.capture_pointer(hitbox.id);
                    cx.notify(current_view);
                    cx.stop_propagation();
                }
            }
        });
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let state = state.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && state.borrow().active && hitbox.is_hovered(window) {
                    if !bounds.contains(&event.position) {
                        state.borrow_mut().active = false;
                        window.release_pointer();
                        cx.notify(current_view);
                        return;
                    }
                    let available = (f32::from(config.direction.extent(bounds)) - config.divider_size).max(0.0);
                    if available > 0.0 {
                        let raw = f32::from(config.direction.coordinate(event.position) - config.direction.origin(bounds)) - config.divider_size / 2.0;
                        state.borrow_mut().ratio = SplitGeometry::new(config, bounds, raw / available).ratio;
                        cx.notify(current_view);
                    }
                }
            }
        });
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let state = state.clone();
            let callback = self.callback.clone();
            let id = self.id;
            let emits_resize = self.emits_resize;
            move |event: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && event.button == MouseButton::Left && state.borrow().active && hitbox.is_hovered(window) {
                    let ratio = state.borrow().ratio;
                    state.borrow_mut().active = false;
                    window.release_pointer();
                    if emits_resize {
                        crate::renderer::emit_event_full(&callback, id, "resize", |payload| payload.ratio = Some(ratio as f64));
                    }
                    cx.notify(current_view);
                    cx.stop_propagation();
                }
            }
        });
        window.on_mouse_event({
            let state = state.clone();
            move |_: &MouseExitEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && state.borrow().active {
                    state.borrow_mut().active = false;
                    window.release_pointer();
                    cx.notify(current_view);
                }
            }
        });
        let divider = self.divider_bounds(prepaint.bounds);
        let line = match self.config.direction {
            Direction::Horizontal => Bounds::new(gpui::point(divider.origin.x + px((self.config.divider_size - 1.0) / 2.0), divider.origin.y), size(px(1.0), divider.size.height)),
            Direction::Vertical => Bounds::new(gpui::point(divider.origin.x, divider.origin.y + px((self.config.divider_size - 1.0) / 2.0)), size(divider.size.width, px(1.0))),
        };
        let color = if state.borrow().active || prepaint.hitbox.is_hovered(window) { gpui::rgba(0x7c86ffff) } else { gpui::rgba(0x5d6481ff) };
        window.paint_quad(gpui::fill(line, color));
    }
}

impl IntoElement for SplitHandleElement {
    type Element = Self;
    fn into_element(self) -> Self::Element { self }
}
