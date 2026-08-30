//! `<browser-surface>` — a macOS WKWebView hosted by GPUI's three-plane compositor.
//!
//! React supplies browser intent only. This element derives bounds and the
//! effective ancestor clip during GPUI prepaint, then updates the native surface
//! directly without a JavaScript geometry callback, polling loop, or fallback.

use std::rc::Rc;

use gpui::{
    px, BrowserSurfaceEvent, BrowserSurfaceObserver, NativeSurfaceGeometry, PlatformBrowserSurface,
};

use super::{CustomElement, CustomElementFactory, CustomRenderContext};

pub struct BrowserSurfaceFactory;

impl CustomElementFactory for BrowserSurfaceFactory {
    fn element_type(&self) -> &str {
        "browser-surface"
    }

    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(BrowserSurfaceElement::default())
    }
}

struct BrowserSurfaceElement {
    url: String,
    profile_id: String,
    visible: bool,
    focus: bool,
    loaded_url: String,
    did_focus: bool,
    back_request_id: String,
    applied_back_request_id: String,
    forward_request_id: String,
    applied_forward_request_id: String,
    reload_request_id: String,
    applied_reload_request_id: String,
    clear_data_request_id: String,
    applied_clear_data_request_id: String,
    surface_profile_id: String,
    surface: Option<Rc<dyn PlatformBrowserSurface>>,
}

impl Default for BrowserSurfaceElement {
    fn default() -> Self {
        Self {
            url: String::new(),
            profile_id: String::new(),
            visible: true,
            focus: false,
            loaded_url: String::new(),
            did_focus: false,
            back_request_id: String::new(),
            applied_back_request_id: String::new(),
            forward_request_id: String::new(),
            applied_forward_request_id: String::new(),
            reload_request_id: String::new(),
            applied_reload_request_id: String::new(),
            clear_data_request_id: String::new(),
            applied_clear_data_request_id: String::new(),
            surface_profile_id: String::new(),
            surface: None,
        }
    }
}

impl BrowserSurfaceElement {
    fn ensure_surface(
        &mut self,
        window: &gpui::Window,
        observer: Option<BrowserSurfaceObserver>,
    ) -> Option<Rc<dyn PlatformBrowserSurface>> {
        if self.surface.is_some() && self.surface_profile_id != self.profile_id {
            self.destroy_surface(true);
        }
        if self.surface.is_none() {
            if self.profile_id.is_empty() {
                return None;
            }
            self.surface = window.create_browser_surface(&self.profile_id);
            if let Some(surface) = &self.surface {
                surface.set_observer(observer);
                self.surface_profile_id.clone_from(&self.profile_id);
            }
        }
        self.surface.clone()
    }

    fn destroy_surface(&mut self, preserve_applied_requests: bool) {
        if let Some(surface) = self.surface.take() {
            surface.destroy();
        }
        self.surface_profile_id.clear();
        self.loaded_url.clear();
        self.did_focus = false;
        if !preserve_applied_requests {
            self.applied_back_request_id.clear();
            self.applied_forward_request_id.clear();
            self.applied_reload_request_id.clear();
            self.applied_clear_data_request_id.clear();
        }
    }

    fn apply_explicit_requests(&mut self, surface: &dyn PlatformBrowserSurface) {
        if !self.back_request_id.is_empty() && self.back_request_id != self.applied_back_request_id
        {
            surface.go_back();
            self.applied_back_request_id
                .clone_from(&self.back_request_id);
        }
        if !self.forward_request_id.is_empty()
            && self.forward_request_id != self.applied_forward_request_id
        {
            surface.go_forward();
            self.applied_forward_request_id
                .clone_from(&self.forward_request_id);
        }
        if !self.reload_request_id.is_empty()
            && self.reload_request_id != self.applied_reload_request_id
        {
            surface.reload();
            self.applied_reload_request_id
                .clone_from(&self.reload_request_id);
        }
        if !self.clear_data_request_id.is_empty()
            && self.clear_data_request_id != self.applied_clear_data_request_id
        {
            surface.clear_data(&self.clear_data_request_id);
            self.applied_clear_data_request_id
                .clone_from(&self.clear_data_request_id);
        }
    }
}

impl CustomElement for BrowserSurfaceElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        window: &mut gpui::Window,
        _cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        use gpui::prelude::*;

        let event_callback = ctx.event_callback.clone();
        let element_id = ctx.id;
        let observer: BrowserSurfaceObserver = Rc::new(move |event| {
            emit_browser_event(&event_callback, element_id, event);
        });
        let Some(surface) = self.ensure_surface(window, Some(observer)) else {
            return gpui::Empty.into_any_element();
        };
        if self.loaded_url != self.url {
            surface.set_url(&self.url);
            self.loaded_url.clone_from(&self.url);
        }
        if self.focus && !self.did_focus {
            surface.focus();
            self.did_focus = true;
        } else if !self.focus {
            self.did_focus = false;
        }
        self.apply_explicit_requests(surface.as_ref());

        let visible = self.visible;
        let corner_radius = ctx
            .style
            .and_then(|style| style.border_radius)
            .unwrap_or_default()
            .max(0.0) as f32;
        let geometry_surface = surface.clone();
        let geometry_probe = gpui::canvas(
            move |bounds, window, _cx| {
                let clip = bounds.intersect(&window.content_mask().bounds);
                geometry_surface.set_geometry(NativeSurfaceGeometry {
                    bounds,
                    clip,
                    corner_radius: px(corner_radius),
                    visible,
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let mut element = gpui::div()
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(geometry_probe);
        if let Some(style) = ctx.style {
            element = crate::renderer::apply_styles(element, style);
        }
        element.into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "url" => self.url = value.as_str().unwrap_or_default().to_string(),
            "profileId" => self.profile_id = value.as_str().unwrap_or_default().to_string(),
            "visible" => self.visible = value.as_bool().unwrap_or(true),
            "focus" => self.focus = value.as_bool().unwrap_or(false),
            "backRequestId" => {
                self.back_request_id = value.as_str().unwrap_or_default().to_string()
            }
            "forwardRequestId" => {
                self.forward_request_id = value.as_str().unwrap_or_default().to_string()
            }
            "reloadRequestId" => {
                self.reload_request_id = value.as_str().unwrap_or_default().to_string()
            }
            "clearDataRequestId" => {
                self.clear_data_request_id = value.as_str().unwrap_or_default().to_string()
            }
            _ => {}
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        &[
            "url",
            "profileId",
            "visible",
            "focus",
            "backRequestId",
            "forwardRequestId",
            "reloadRequestId",
            "clearDataRequestId",
        ]
    }

    fn supported_events(&self) -> &'static [&'static str] {
        &[
            "browserNavigation",
            "browserLoading",
            "browserDownload",
            "browserDataCleared",
        ]
    }

    fn destroy(&mut self) {
        self.destroy_surface(false);
    }
}

fn emit_browser_event(
    callback: &Option<crate::renderer::EventCallback>,
    element_id: u64,
    event: BrowserSurfaceEvent,
) {
    crate::renderer::emit_event_full(callback, element_id, event.event_type(), |payload| {
        payload.browser_profile_id = Some(event.profile_id().to_string());
        match event {
            BrowserSurfaceEvent::Navigation {
                url,
                can_go_back,
                can_go_forward,
                ..
            } => {
                payload.browser_url = Some(url);
                payload.browser_can_go_back = Some(can_go_back);
                payload.browser_can_go_forward = Some(can_go_forward);
            }
            BrowserSurfaceEvent::Loading {
                is_loading,
                url,
                can_go_back,
                can_go_forward,
                ..
            } => {
                payload.browser_is_loading = Some(is_loading);
                payload.browser_url = Some(url);
                payload.browser_can_go_back = Some(can_go_back);
                payload.browser_can_go_forward = Some(can_go_forward);
            }
            BrowserSurfaceEvent::Download {
                download_id,
                suggested_filename,
                ..
            } => {
                payload.browser_download_id = Some(download_id);
                payload.browser_suggested_filename = Some(suggested_filename);
            }
            BrowserSurfaceEvent::DataCleared { request_id, .. } => {
                payload.browser_request_id = Some(request_id);
            }
        }
    });
}
