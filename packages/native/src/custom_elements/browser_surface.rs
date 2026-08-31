//! `<browser-surface>` — a macOS WKWebView hosted by GPUI's three-plane compositor.
//!
//! React supplies browser intent only. This element derives bounds and the
//! effective ancestor clip during GPUI prepaint, then updates the native surface
//! directly without a JavaScript geometry callback, polling loop, or fallback.

use std::{collections::HashMap, rc::Rc};

use gpui::{
    px, Bounds, BrowserActionDecision, BrowserActionRequest, BrowserActionResolution,
    BrowserNavigationIntent, BrowserSurfaceEvent, BrowserSurfaceObserver, NativeSurfaceGeometry,
    Pixels, PlatformBrowserSurface,
};
use serde::Deserialize;

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
    profile_id: String,
    visible: bool,
    focus: bool,
    did_focus: bool,
    navigation_intent: Option<NavigationIntentProp>,
    applied_navigation_intent_id: String,
    pending_navigation_intents: HashMap<String, BrowserNavigationIntent>,
    action_decision: Option<BrowserActionDecisionProp>,
    applied_action_decision_id: String,
    clear_data_request_id: String,
    applied_clear_data_request_id: String,
    surface_profile_id: String,
    surface: Option<Rc<dyn PlatformBrowserSurface>>,
}

impl Default for BrowserSurfaceElement {
    fn default() -> Self {
        Self {
            profile_id: String::new(),
            visible: true,
            focus: false,
            did_focus: false,
            navigation_intent: None,
            applied_navigation_intent_id: String::new(),
            pending_navigation_intents: HashMap::new(),
            action_decision: None,
            applied_action_decision_id: String::new(),
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
        self.did_focus = false;
        self.pending_navigation_intents.clear();
        if !preserve_applied_requests {
            self.applied_navigation_intent_id.clear();
            self.applied_action_decision_id.clear();
            self.applied_clear_data_request_id.clear();
        }
    }

    fn request_navigation_intent(&mut self, event_callback: &Option<crate::renderer::EventCallback>, element_id: u64) {
        let Some(intent) = self.navigation_intent.as_ref() else {
            return;
        };
        if intent.request_id.is_empty() || intent.request_id == self.applied_navigation_intent_id {
            return;
        }
        let request_id = intent.request_id.clone();
        let Some(intent) = intent.into_navigation_intent() else {
            return;
        };
        self.pending_navigation_intents.insert(request_id.clone(), intent.clone());
        emit_browser_event(
            event_callback,
            element_id,
            BrowserSurfaceEvent::ActionRequested(BrowserActionRequest::NavigationIntent {
                request_id: request_id.clone(),
                intent,
                profile_id: self.profile_id.clone(),
            }),
        );
        self.applied_navigation_intent_id = request_id;
    }

    fn apply_action_decision(&mut self, surface: &dyn PlatformBrowserSurface) {
        let Some(decision) = self.action_decision.as_ref() else {
            return;
        };
        if decision.request_id.is_empty() || decision.request_id == self.applied_action_decision_id {
            return;
        }
        let Some(resolution) = decision.into_resolution() else {
            return;
        };
        let accepted = if let Some(intent) = self.pending_navigation_intents.get(&resolution.request_id).cloned() {
            match resolution.decision {
                BrowserActionDecision::Allow => {
                    self.pending_navigation_intents.remove(&resolution.request_id);
                    match intent {
                        BrowserNavigationIntent::Navigate { url } => surface.load_url(&url),
                        BrowserNavigationIntent::Back => surface.go_back(),
                        BrowserNavigationIntent::Forward => surface.go_forward(),
                        BrowserNavigationIntent::Reload => surface.reload(),
                    }
                    true
                }
                BrowserActionDecision::Cancel => {
                    self.pending_navigation_intents.remove(&resolution.request_id);
                    true
                }
                BrowserActionDecision::Download | BrowserActionDecision::Save { .. } => false,
            }
        } else {
            surface.resolve_action(resolution)
        };
        if accepted {
            self.applied_action_decision_id.clone_from(&decision.request_id);
        }
    }

    fn apply_explicit_requests(
        &mut self,
        surface: &dyn PlatformBrowserSurface,
        event_callback: &Option<crate::renderer::EventCallback>,
        element_id: u64,
    ) {
        self.request_navigation_intent(event_callback, element_id);
        self.apply_action_decision(surface);
        if !self.clear_data_request_id.is_empty()
            && self.clear_data_request_id != self.applied_clear_data_request_id
        {
            surface.clear_data(&self.clear_data_request_id);
            self.applied_clear_data_request_id
                .clone_from(&self.clear_data_request_id);
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NavigationIntentProp {
    request_id: String,
    kind: String,
    url: Option<String>,
}

impl NavigationIntentProp {
    fn into_navigation_intent(&self) -> Option<BrowserNavigationIntent> {
        match self.kind.as_str() {
            "navigate" => self.url.as_ref().filter(|url| !url.trim().is_empty()).map(|url| {
                BrowserNavigationIntent::Navigate { url: url.clone() }
            }),
            "back" => Some(BrowserNavigationIntent::Back),
            "forward" => Some(BrowserNavigationIntent::Forward),
            "reload" => Some(BrowserNavigationIntent::Reload),
            _ => None,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserActionDecisionProp {
    request_id: String,
    decision: String,
    destination_url: Option<String>,
}

impl BrowserActionDecisionProp {
    fn into_resolution(&self) -> Option<BrowserActionResolution> {
        let decision = match self.decision.as_str() {
            "allow" => BrowserActionDecision::Allow,
            "cancel" => BrowserActionDecision::Cancel,
            "download" => BrowserActionDecision::Download,
            "save" => self
                .destination_url
                .as_ref()
                .filter(|url| !url.trim().is_empty())
                .map(|url| BrowserActionDecision::Save {
                    destination_url: url.clone(),
                })?,
            _ => return None,
        };
        Some(BrowserActionResolution {
            request_id: self.request_id.clone(),
            decision,
        })
    }
}

fn effective_browser_clip(
    bounds: Bounds<Pixels>,
    cumulative_content_mask: Bounds<Pixels>,
) -> Bounds<Pixels> {
    bounds.intersect(&cumulative_content_mask)
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
        let observer_callback = event_callback.clone();
        let observer: BrowserSurfaceObserver = Rc::new(move |event| {
            emit_browser_event(&observer_callback, element_id, event);
        });
        let Some(surface) = self.ensure_surface(window, Some(observer)) else {
            return gpui::Empty.into_any_element();
        };
        if self.focus && !self.did_focus {
            surface.focus();
            self.did_focus = true;
        } else if !self.focus {
            self.did_focus = false;
        }
        self.apply_explicit_requests(surface.as_ref(), &event_callback, element_id);

        let visible = self.visible;
        let corner_radius = ctx
            .style
            .and_then(|style| style.border_radius)
            .unwrap_or_default()
            .max(0.0) as f32;
        let geometry_surface = surface.clone();
        let geometry_probe = gpui::canvas(
            move |bounds, window, _cx| {
                let clip = effective_browser_clip(bounds, window.content_mask().bounds);
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
            "profileId" => self.profile_id = value.as_str().unwrap_or_default().to_string(),
            "visible" => self.visible = value.as_bool().unwrap_or(true),
            "focus" => self.focus = value.as_bool().unwrap_or(false),
            "navigationIntent" => self.navigation_intent = serde_json::from_value(value).ok(),
            "actionDecision" => self.action_decision = serde_json::from_value(value).ok(),
            "clearDataRequestId" => {
                self.clear_data_request_id = value.as_str().unwrap_or_default().to_string()
            }
            _ => {}
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        &[
            "profileId",
            "visible",
            "focus",
            "navigationIntent",
            "actionDecision",
            "clearDataRequestId",
        ]
    }

    fn supported_events(&self) -> &'static [&'static str] {
        &[
            "browserNavigation",
            "browserLoading",
            "browserActionRequested",
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
            BrowserSurfaceEvent::ActionRequested(request) => {
                payload.browser_request_id = Some(request.request_id().to_string());
                payload.browser_action_kind = Some(request.kind().to_string());
                match request {
                    BrowserActionRequest::NavigationIntent { intent, .. } => {
                        payload.browser_navigation_intent = Some(match intent {
                            BrowserNavigationIntent::Navigate { url } => {
                                payload.browser_url = Some(url);
                                "navigate"
                            }
                            BrowserNavigationIntent::Back => "back",
                            BrowserNavigationIntent::Forward => "forward",
                            BrowserNavigationIntent::Reload => "reload",
                        }
                        .to_string());
                    }
                    BrowserActionRequest::NavigationAction {
                        url,
                        is_main_frame,
                        should_perform_download,
                        ..
                    } => {
                        payload.browser_url = Some(url);
                        payload.browser_is_main_frame = Some(is_main_frame);
                        payload.browser_should_perform_download = Some(should_perform_download);
                    }
                    BrowserActionRequest::NavigationResponse {
                        url,
                        can_show_mime_type,
                        ..
                    } => {
                        payload.browser_url = Some(url);
                        payload.browser_can_show_mime_type = Some(can_show_mime_type);
                    }
                    BrowserActionRequest::DownloadDestination {
                        download_id,
                        suggested_filename,
                        ..
                    } => {
                        payload.browser_download_id = Some(download_id);
                        payload.browser_suggested_filename = Some(suggested_filename);
                    }
                }
            }
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
            BrowserSurfaceEvent::DataCleared { request_id, .. } => {
                payload.browser_request_id = Some(request_id);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};
    use std::sync::{Arc, Mutex};

    #[test]
    fn effective_clip_intersects_the_prepaint_bounds_with_the_cumulative_mask() {
        let bounds = Bounds::new(point(px(20.), px(30.)), size(px(200.), px(120.)));
        let cumulative_mask = Bounds::new(point(px(50.), px(10.)), size(px(90.), px(80.)));

        assert_eq!(
            effective_browser_clip(bounds, cumulative_mask),
            Bounds::new(point(px(50.), px(30.)), size(px(90.), px(60.))),
        );
    }

    #[test]
    fn profile_recreation_discards_pending_navigation_intents_but_preserves_consumed_ids() {
        let mut element = BrowserSurfaceElement {
            profile_id: "profile-b".into(),
            surface_profile_id: "profile-a".into(),
            applied_navigation_intent_id: "request-a".into(),
            pending_navigation_intents: HashMap::from([(
                "request-a".into(),
                BrowserNavigationIntent::Navigate {
                    url: "https://example.com".into(),
                },
            )]),
            ..Default::default()
        };

        element.destroy_surface(true);

        assert!(element.pending_navigation_intents.is_empty());
        assert_eq!(element.applied_navigation_intent_id, "request-a");
    }

    #[test]
    fn action_events_preserve_webkit_download_facts() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let callback: crate::renderer::EventCallback = Arc::new(move |event| {
            captured.lock().unwrap().push(event);
        });

        emit_browser_event(
            &Some(callback.clone()),
            7,
            BrowserSurfaceEvent::ActionRequested(BrowserActionRequest::NavigationAction {
                request_id: "action".into(),
                url: "https://example.com/file".into(),
                is_main_frame: true,
                should_perform_download: true,
                profile_id: "profile".into(),
            }),
        );
        emit_browser_event(
            &Some(callback),
            7,
            BrowserSurfaceEvent::ActionRequested(BrowserActionRequest::NavigationResponse {
                request_id: "response".into(),
                url: "https://example.com/file".into(),
                can_show_mime_type: false,
                profile_id: "profile".into(),
            }),
        );

        let events = events.lock().unwrap();
        assert_eq!(events[0].browser_should_perform_download, Some(true));
        assert_eq!(events[0].browser_can_show_mime_type, None);
        assert_eq!(events[1].browser_should_perform_download, None);
        assert_eq!(events[1].browser_can_show_mime_type, Some(false));
    }
}
