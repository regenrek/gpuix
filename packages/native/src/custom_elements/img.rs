/// Image custom elements for raster images and tintable SVG icons.
///
/// This provides a native `<img>` for GPUIX React apps while keeping the same
/// custom-element prop pipeline (`setCustomProp`/`custom_props`).
use super::{CustomElement, CustomElementFactory, CustomRenderContext};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::sync::Arc;

const MAX_INLINE_IMAGE_BYTES: usize = 32 * 1024 * 1024;

pub struct ImgFactory;

pub struct SvgFactory;

impl CustomElementFactory for SvgFactory {
    fn element_type(&self) -> &str {
        "svg"
    }

    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(SvgElement::default())
    }
}

impl CustomElementFactory for ImgFactory {
    fn element_type(&self) -> &str {
        "img"
    }

    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(ImgElement::default())
    }
}

#[derive(Debug, Clone)]
enum ImgObjectFit {
    Fill,
    Contain,
    Cover,
    ScaleDown,
    None,
}

impl Default for ImgObjectFit {
    fn default() -> Self {
        Self::Contain
    }
}

impl ImgObjectFit {
    fn from_str(value: &str) -> Self {
        match value {
            "fill" => Self::Fill,
            "cover" => Self::Cover,
            "scaleDown" => Self::ScaleDown,
            "none" => Self::None,
            _ => Self::Contain,
        }
    }

    fn as_gpui(&self) -> gpui::ObjectFit {
        match self {
            Self::Fill => gpui::ObjectFit::Fill,
            Self::Contain => gpui::ObjectFit::Contain,
            Self::Cover => gpui::ObjectFit::Cover,
            Self::ScaleDown => gpui::ObjectFit::ScaleDown,
            Self::None => gpui::ObjectFit::None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImgElement {
    src: String,
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    preview_handle: Option<String>,
    inline_image: Option<Arc<gpui::Image>>,
    inline_error: bool,
    object_fit: ImgObjectFit,
}

impl ImgElement {
    fn load_src(&mut self, src: String) {
        let inline = decode_inline_image(&src);
        self.inline_error = src.starts_with("data:") && inline.is_none();
        self.inline_image =
            inline.map(|(format, bytes)| Arc::new(gpui::Image::from_bytes(format, bytes)));
        self.src = src;
    }
}

fn decode_inline_image(src: &str) -> Option<(gpui::ImageFormat, Vec<u8>)> {
    let payload = src.strip_prefix("data:")?;
    let (meta, encoded) = payload.split_once(',')?;
    let mime_type = meta.strip_suffix(";base64")?;
    let format = gpui::ImageFormat::from_mime_type(mime_type)?;
    let bytes = BASE64.decode(encoded).ok()?;
    if bytes.is_empty() || bytes.len() > MAX_INLINE_IMAGE_BYTES {
        return None;
    }
    Some((format, bytes))
}

impl CustomElement for ImgElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        use gpui::prelude::*;

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let has_preview = self.preview_handle.is_some();
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let has_preview = false;

        if self.src.trim().is_empty() && !has_preview {
            let mut fallback = gpui::div()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgba(0x1f2230ff))
                .border(gpui::px(1.0))
                .border_color(gpui::rgba(0x5d6481ff))
                .text_color(gpui::rgba(0xa4accdff))
                .child("img: no src");

            if let Some(style) = ctx.style {
                fallback = crate::renderer::apply_styles(fallback, style);
            }

            return fallback.into_any_element();
        }

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let source: gpui::ImageSource = if let Some(handle) = &self.preview_handle {
            let Some(image) = ctx.appshot.lock().unwrap().preview_image(handle) else {
                return missing_preview(ctx.style);
            };
            image.into()
        } else if let Some(image) = &self.inline_image {
            image.clone().into()
        } else if self.inline_error {
            let mut fallback = gpui::div()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgba(0x1f2230ff))
                .border(gpui::px(1.0))
                .border_color(gpui::rgba(0x5d6481ff))
                .text_color(gpui::rgba(0xa4accdff))
                .child("img: invalid inline source");
            if let Some(style) = ctx.style {
                fallback = crate::renderer::apply_styles(fallback, style);
            }
            return fallback.into_any_element();
        } else {
            std::path::PathBuf::from(self.src.clone()).into()
        };
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let source: gpui::ImageSource = if let Some(image) = &self.inline_image {
            image.clone().into()
        } else if self.inline_error {
            let mut fallback = gpui::div()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgba(0x1f2230ff))
                .border(gpui::px(1.0))
                .border_color(gpui::rgba(0x5d6481ff))
                .text_color(gpui::rgba(0xa4accdff))
                .child("img: invalid inline source");
            if let Some(style) = ctx.style {
                fallback = crate::renderer::apply_styles(fallback, style);
            }
            return fallback.into_any_element();
        } else {
            std::path::PathBuf::from(self.src.clone()).into()
        };
        let mut el = gpui::img(source)
            .object_fit(self.object_fit.as_gpui())
            .with_fallback(|| {
                gpui::div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(gpui::rgba(0x1f2230ff))
                    .border(gpui::px(1.0))
                    .border_color(gpui::rgba(0x5d6481ff))
                    .text_color(gpui::rgba(0xa4accdff))
                    .child("img: load failed")
                    .into_any_element()
            });

        if let Some(style) = ctx.style {
            el = crate::renderer::apply_styles(el, style);
        }

        el.into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "src" => self.load_src(value.as_str().unwrap_or("").to_string()),
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            "appshotPreviewHandle" => {
                self.preview_handle = value.as_str().map(ToOwned::to_owned);
            }
            "objectFit" => {
                self.object_fit = value
                    .as_str()
                    .map(ImgObjectFit::from_str)
                    .unwrap_or_default()
            }
            _ => {}
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        return &["src", "appshotPreviewHandle", "objectFit"];
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        &["src", "objectFit"]
    }

    fn supported_events(&self) -> &'static [&'static str] {
        &[]
    }

    fn destroy(&mut self) {}
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn missing_preview(style: Option<&crate::style::StyleDesc>) -> gpui::AnyElement {
    use gpui::prelude::*;

    let mut fallback = gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::rgba(0x1f2230ff))
        .border(gpui::px(1.0))
        .border_color(gpui::rgba(0x5d6481ff))
        .text_color(gpui::rgba(0xa4accdff))
        .child("img: preview unavailable");
    if let Some(style) = style {
        fallback = crate::renderer::apply_styles(fallback, style);
    }
    fallback.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_image_requires_supported_bounded_base64_data_uri() {
        let (format, bytes) = decode_inline_image("data:image/png;base64,iVBORw0KGgo=")
            .expect("valid PNG data URI should decode");
        assert_eq!(format, gpui::ImageFormat::Png);
        assert_eq!(bytes, b"\x89PNG\r\n\x1a\n");
        assert!(decode_inline_image("data:text/plain;base64,aGk=").is_none());
        assert!(decode_inline_image("data:image/png,not-base64").is_none());
        assert!(decode_inline_image("data:image/png;base64,").is_none());
    }
}

#[derive(Debug, Clone, Default)]
pub struct SvgElement {
    src: String,
    bytes: Option<std::sync::Arc<[u8]>>,
    source: String,
}

impl SvgElement {
    fn load_src(&mut self, src: String) {
        self.bytes = svg_bytes(&src).map(std::sync::Arc::from);
        self.src = src;
    }
}

fn svg_bytes(src: &str) -> Option<Vec<u8>> {
    if let Some(payload) = src.strip_prefix("data:") {
        let (meta, data) = payload.split_once(',')?;
        if !meta.starts_with("image/svg+xml") {
            return None;
        }
        return Some(percent_decode(data));
    }
    #[cfg(target_family = "wasm")]
    return None;
    #[cfg(not(target_family = "wasm"))]
    std::fs::read(src).ok()
}

fn percent_decode(input: &str) -> Vec<u8> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(
                std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                16,
            ) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    out
}

impl CustomElement for SvgElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        use gpui::prelude::*;

        let bytes = if self.source.trim().is_empty() {
            self.bytes.as_deref()
        } else {
            Some(self.source.as_bytes())
        };
        let Some(bytes) = bytes else {
            let mut empty = gpui::div();
            if let Some(style) = ctx.style {
                empty = crate::renderer::apply_styles(empty, style);
            }
            return empty.into_any_element();
        };

        let tint = ctx
            .style
            .and_then(|style| style.color.as_deref())
            .and_then(crate::color::parse_color_rgba)
            .unwrap_or_else(|| gpui::rgb(0xe2e2e2).into());
        let mut icon = gpui::svg().data(bytes).flex_none().text_color(tint);
        if let Some(style) = ctx.style {
            icon = crate::renderer::apply_styles(icon, style);
        }
        icon.into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "src" => self.load_src(value.as_str().unwrap_or_default().to_string()),
            "source" => self.source = value.as_str().unwrap_or_default().to_string(),
            _ => {}
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        &["src", "source"]
    }

    fn supported_events(&self) -> &'static [&'static str] {
        &[]
    }

    fn destroy(&mut self) {}
}
