//! A bounded, noninteractive retained drawing surface.
//!
//! Commands are validated as a complete snapshot before replacing the retained
//! list. Invalid snapshots clear the previous frame, which prevents stale
//! graphics from surviving a rejected React update.

use std::{
    collections::HashSet,
    io::{self, Write},
    sync::Arc,
    time::Duration,
};

use gpui::{prelude::*, Bounds, Pixels};
use web_time::Instant;

use super::{CustomElement, CustomElementFactory, CustomRenderContext};

const MAX_SNAPSHOT_BYTES: usize = 256 * 1024;
const MAX_COMMANDS: usize = 2_048;
const MAX_PARTICLES: usize = 256;
const MAX_ID_BYTES: usize = 96;
const MIN_PARTICLE_DURATION_MS: u64 = 16;
const MAX_PARTICLE_DURATION_MS: u64 = 60_000;

pub struct CanvasFactory;

impl CustomElementFactory for CanvasFactory {
    fn element_type(&self) -> &str {
        "canvas"
    }

    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(CanvasElement::default())
    }
}

#[derive(Clone)]
enum CanvasCommand {
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: gpui::Rgba,
    },
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: gpui::Rgba,
    },
    Circle {
        center: Point,
        radius: f32,
        color: gpui::Rgba,
    },
    Particle {
        from: Point,
        to: Point,
        radius: f32,
        color: gpui::Rgba,
        duration_ms: u64,
        phase_ms: u64,
    },
}

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

pub struct CanvasElement {
    commands: Arc<Vec<CanvasCommand>>,
    visible: bool,
    running: bool,
    elapsed: Duration,
    last_now: Option<Instant>,
    reset_playback: bool,
}

impl Default for CanvasElement {
    fn default() -> Self {
        Self {
            commands: Arc::new(Vec::new()),
            visible: true,
            running: true,
            elapsed: Duration::ZERO,
            last_now: None,
            reset_playback: false,
        }
    }
}

impl CanvasElement {
    fn replace_commands(&mut self, value: serde_json::Value) {
        match decode_snapshot(&value) {
            Ok(commands) => {
                self.commands = Arc::new(commands);
                self.reset_playback = true;
            }
            Err(reason) => {
                log::warn!("Canvas command snapshot rejected: {reason}");
                self.commands = Arc::new(Vec::new());
                self.reset_playback = true;
            }
        }
    }

    fn elapsed_ms(&mut self, now: Instant) -> u64 {
        if self.reset_playback {
            self.elapsed = Duration::ZERO;
            self.reset_playback = false;
            self.last_now = Some(now);
            return 0;
        }
        if self.visible && self.running {
            if let Some(last_now) = self.last_now {
                self.elapsed = self
                    .elapsed
                    .saturating_add(now.saturating_duration_since(last_now));
            }
        }
        self.last_now = Some(now);
        self.elapsed.as_millis().min(u128::from(u64::MAX)) as u64
    }

    fn has_particles(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command, CanvasCommand::Particle { .. }))
    }
}

impl CustomElement for CanvasElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        window: &mut gpui::Window,
        _cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        if !self.visible {
            self.elapsed_ms(ctx.now);
            return gpui::Empty.into_any_element();
        }
        let elapsed_ms = self.elapsed_ms(ctx.now);
        if self.running && self.has_particles() {
            window.request_animation_frame();
        }

        let commands = self.commands.clone();
        let drawing = gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                for command in commands.iter() {
                    paint_command(command, bounds, elapsed_ms, window);
                }
            },
        )
        .size_full();

        let mut shell = gpui::div()
            .relative()
            .child(crate::automation::bounds_tracker(ctx.id, None))
            .child(drawing);
        if let Some(style) = ctx.style {
            shell = crate::renderer::apply_styles(shell, style);
        }
        shell.overflow_hidden().into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "commands" => self.replace_commands(value),
            "visible" => {
                self.visible = value.as_bool().unwrap_or(true);
                self.last_now = None;
            }
            "motion" => {
                self.running = value.as_str().unwrap_or("running") == "running";
                self.last_now = None;
            }
            _ => {}
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        &["commands", "visible", "motion"]
    }
    fn supported_events(&self) -> &'static [&'static str] {
        &[]
    }
    fn destroy(&mut self) {}
}

fn decode_snapshot(value: &serde_json::Value) -> Result<Vec<CanvasCommand>, &'static str> {
    serde_json::to_writer(BoundedSnapshotWriter::default(), value)
        .map_err(|_| "command snapshot exceeds 256 KiB")?;
    let commands = value.as_array().ok_or("commands must be an array")?;
    if commands.len() > MAX_COMMANDS {
        return Err("command snapshot exceeds 2048 commands");
    }
    let mut ids = HashSet::with_capacity(commands.len());
    let mut particles = 0usize;
    let mut decoded = Vec::with_capacity(commands.len());
    for command in commands {
        let command = command.as_object().ok_or("command must be an object")?;
        let id = command
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= MAX_ID_BYTES)
            .ok_or("command id must be a nonempty bounded string")?;
        if !ids.insert(id.to_owned()) {
            return Err("command ids must be unique");
        }
        let color = command
            .get("color")
            .and_then(serde_json::Value::as_str)
            .and_then(crate::color::parse_color_rgba)
            .ok_or("command color is invalid")?;
        let kind = command
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or("command type is missing")?;
        let decoded_command = match kind {
            "line" => CanvasCommand::Line {
                from: point(command, "from")?,
                to: point(command, "to")?,
                width: normalized_number(command, "width")?,
                color,
            },
            "rect" => {
                let width = normalized_number(command, "width")?;
                let height = normalized_number(command, "height")?;
                let radius = command
                    .get("radius")
                    .map(finite_number)
                    .transpose()?
                    .unwrap_or(0.0)
                    .clamp(0.0, width.min(height) / 2.0);
                CanvasCommand::Rect {
                    x: normalized_number(command, "x")?,
                    y: normalized_number(command, "y")?,
                    width,
                    height,
                    radius,
                    color,
                }
            }
            "circle" => CanvasCommand::Circle {
                center: point(command, "center")?,
                radius: normalized_number(command, "radius")?,
                color,
            },
            "particle" => {
                particles += 1;
                if particles > MAX_PARTICLES {
                    return Err("command snapshot exceeds 256 particles");
                }
                let duration_ms = command
                    .get("durationMs")
                    .and_then(serde_json::Value::as_u64)
                    .filter(|duration| {
                        (MIN_PARTICLE_DURATION_MS..=MAX_PARTICLE_DURATION_MS).contains(duration)
                    })
                    .ok_or("particle duration must be between 16 and 60000 milliseconds")?;
                let phase_ms = command
                    .get("phaseMs")
                    .map(|value| value.as_u64().ok_or("particle phase must be an integer"))
                    .transpose()?
                    .unwrap_or(0)
                    % duration_ms;
                CanvasCommand::Particle {
                    from: point(command, "from")?,
                    to: point(command, "to")?,
                    radius: normalized_number(command, "radius")?,
                    color,
                    duration_ms,
                    phase_ms,
                }
            }
            _ => return Err("unsupported canvas command type"),
        };
        decoded.push(decoded_command);
    }
    Ok(decoded)
}

#[derive(Default)]
struct BoundedSnapshotWriter {
    written: usize,
}

impl Write for BoundedSnapshotWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = MAX_SNAPSHOT_BYTES.saturating_sub(self.written);
        if bytes.len() > remaining {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "canvas command snapshot exceeds 256 KiB",
            ));
        }
        self.written += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn point(
    command: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Point, &'static str> {
    let point = command
        .get(name)
        .and_then(serde_json::Value::as_object)
        .ok_or("point must be an object")?;
    Ok(Point {
        x: normalized_number(point, "x")?,
        y: normalized_number(point, "y")?,
    })
}

fn normalized_number(
    command: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<f32, &'static str> {
    let value = finite_number(command.get(name).ok_or("normalized number is missing")?)?;
    if !(0.0..=1.0).contains(&value) {
        return Err("normalized number must be between zero and one");
    }
    Ok(value)
}

fn finite_number(value: &serde_json::Value) -> Result<f32, &'static str> {
    let value = value.as_f64().ok_or("number is invalid")?;
    if !value.is_finite() || value > f64::from(f32::MAX) || value < f64::from(f32::MIN) {
        return Err("number must be finite");
    }
    Ok(value as f32)
}

fn canvas_point(bounds: Bounds<Pixels>, point: Point) -> gpui::Point<Pixels> {
    gpui::point(
        bounds.origin.x + bounds.size.width * point.x,
        bounds.origin.y + bounds.size.height * point.y,
    )
}

fn paint_command(
    command: &CanvasCommand,
    bounds: Bounds<Pixels>,
    elapsed_ms: u64,
    window: &mut gpui::Window,
) {
    let shortest_side = f32::from(bounds.size.width).min(f32::from(bounds.size.height));
    match command {
        CanvasCommand::Line {
            from,
            to,
            width,
            color,
        } => {
            let mut path = gpui::PathBuilder::stroke(gpui::px(width * shortest_side));
            path.move_to(canvas_point(bounds, *from));
            path.line_to(canvas_point(bounds, *to));
            if let Ok(path) = path.build() {
                window.paint_path(path, *color);
            }
        }
        CanvasCommand::Rect {
            x,
            y,
            width,
            height,
            radius,
            color,
        } => window.paint_quad(gpui::quad(
            Bounds {
                origin: canvas_point(bounds, Point { x: *x, y: *y }),
                size: gpui::size(bounds.size.width * *width, bounds.size.height * *height),
            },
            gpui::px(radius * shortest_side),
            *color,
            gpui::px(0.0),
            gpui::transparent_black(),
            Default::default(),
        )),
        CanvasCommand::Circle {
            center,
            radius,
            color,
        } => {
            let radius_px = radius * shortest_side;
            let center = canvas_point(bounds, *center);
            window.paint_quad(gpui::quad(
                Bounds {
                    origin: gpui::point(
                        center.x - gpui::px(radius_px),
                        center.y - gpui::px(radius_px),
                    ),
                    size: gpui::size(gpui::px(radius_px * 2.0), gpui::px(radius_px * 2.0)),
                },
                gpui::px(radius_px),
                *color,
                gpui::px(0.0),
                gpui::transparent_black(),
                Default::default(),
            ));
        }
        CanvasCommand::Particle {
            from,
            to,
            radius,
            color,
            duration_ms,
            phase_ms,
        } => {
            let time_in_cycle = elapsed_ms % duration_ms;
            let progress = ((time_in_cycle + phase_ms) % duration_ms) as f32 / *duration_ms as f32;
            paint_command(
                &CanvasCommand::Circle {
                    center: Point {
                        x: from.x + (to.x - from.x) * progress,
                        y: from.y + (to.y - from.y) * progress,
                    },
                    radius: *radius,
                    color: *color,
                },
                bounds,
                elapsed_ms,
                window,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn particle(id: &str) -> serde_json::Value {
        json!({ "type": "particle", "id": id, "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 }, "radius": 0.1, "color": "#ffffff", "durationMs": 100 })
    }
    #[test]
    fn accepts_the_four_retained_command_types_in_source_order() {
        let snapshot = json!([
            { "type": "line", "id": "line", "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 }, "width": 0.01, "color": "#f00" },
            { "type": "rect", "id": "rect", "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0, "color": "#0f0" },
            { "type": "circle", "id": "circle", "center": { "x": 0.5, "y": 0.5 }, "radius": 0.1, "color": "#00f" }, particle("particle"),
        ]);
        assert_eq!(decode_snapshot(&snapshot).unwrap().len(), 4);
    }
    #[test]
    fn bounds_snapshot_serialization_without_retaining_a_copy() {
        let mut writer = BoundedSnapshotWriter::default();
        let cap_sized_chunk = vec![0; MAX_SNAPSHOT_BYTES];
        assert_eq!(writer.write(&cap_sized_chunk).unwrap(), MAX_SNAPSHOT_BYTES);
        assert_eq!(writer.written, MAX_SNAPSHOT_BYTES);
        assert!(writer.write(&[0]).is_err());
        let too_large = json!([{ "id": "x".repeat(MAX_SNAPSHOT_BYTES), "type": "line" }]);
        assert!(matches!(
            decode_snapshot(&too_large),
            Err("command snapshot exceeds 256 KiB")
        ));
    }
    #[test]
    fn accepts_2048_commands_and_rejects_2049_before_retaining_them() {
        let commands = serde_json::Value::Array((0..MAX_COMMANDS).map(|index| json!({ "type": "line", "id": format!("line-{index}"), "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 }, "width": 0.01, "color": "#fff" })).collect());
        assert_eq!(decode_snapshot(&commands).unwrap().len(), MAX_COMMANDS);
        let mut too_many = commands.as_array().unwrap().clone();
        too_many.push(json!({ "type": "line", "id": "line-over-cap", "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 }, "width": 0.01, "color": "#fff" }));
        assert!(matches!(
            decode_snapshot(&serde_json::Value::Array(too_many)),
            Err("command snapshot exceeds 2048 commands")
        ));
    }
    #[test]
    fn rejection_reasons_cover_ids_geometry_and_particle_caps() {
        let duplicate = json!([particle("same"), particle("same")]);
        assert!(matches!(
            decode_snapshot(&duplicate),
            Err("command ids must be unique")
        ));
        let invalid_geometry = json!([{ "type": "circle", "id": "circle", "center": { "x": 2.0, "y": 0.5 }, "radius": 0.1, "color": "#fff" }]);
        assert!(matches!(
            decode_snapshot(&invalid_geometry),
            Err("normalized number must be between zero and one")
        ));
        let particles = serde_json::Value::Array(
            (0..=MAX_PARTICLES)
                .map(|index| particle(&format!("particle-{index}")))
                .collect(),
        );
        assert!(matches!(
            decode_snapshot(&particles),
            Err("command snapshot exceeds 256 particles")
        ));
    }
}
