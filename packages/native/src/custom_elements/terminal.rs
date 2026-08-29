//! `<terminal>` — a desktop-only native terminal emulator surface.
//!
//! GPUIX owns VT parsing, cell rendering, keyboard translation, scrollback,
//! and viewport measurement. The host owns PTY identity and lifecycle and feeds
//! output through `GpuixRenderer::terminal_write`, so terminal throughput never
//! becomes React prop churn.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, Rgb};
use base64::Engine as _;
use gpui::{
    div, font, point, px, App, Bounds, Font, FontStyle, FontWeight, Hsla, InputHandler,
    KeyDownEvent, Pixels, Point, ScrollDelta, StrikethroughStyle, TextRun, UTF16Selection,
    UnderlineStyle, Window,
};

use super::{CustomElement, CustomElementFactory, CustomRenderContext};
use crate::renderer::emit_event_full;
use crate::theme::Theme;

const INITIAL_ROWS: usize = 24;
const INITIAL_COLS: usize = 80;

pub struct TerminalFactory;

impl CustomElementFactory for TerminalFactory {
    fn element_type(&self) -> &str {
        "terminal"
    }

    fn create(&self, _id: u64) -> Box<dyn CustomElement> {
        Box::new(TerminalElement::new())
    }
}

#[derive(Clone, Copy)]
struct TerminalSize {
    rows: usize,
    cols: usize,
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

struct TerminalState {
    term: Term<VoidListener>,
    parser: Processor,
    size: TerminalSize,
}

impl TerminalState {
    fn new() -> Self {
        let size = TerminalSize {
            rows: INITIAL_ROWS,
            cols: INITIAL_COLS,
        };
        Self::with_size(size)
    }

    fn with_size(size: TerminalSize) -> Self {
        let mut config = Config::default();
        config.osc52 = alacritty_terminal::term::Osc52::Disabled;
        Self {
            term: Term::new(config, &size, VoidListener),
            parser: Processor::new(),
            size,
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
        self.term.scroll_display(Scroll::Bottom);
    }

    fn reset(&mut self) {
        *self = Self::with_size(self.size);
    }

    fn resize(&mut self, rows: usize, cols: usize) -> bool {
        let next = TerminalSize { rows, cols };
        if next.rows == self.size.rows && next.cols == self.size.cols {
            return false;
        }
        self.term.resize(next);
        self.size = next;
        true
    }
}

pub struct TerminalElement {
    state: Rc<RefCell<TerminalState>>,
    ime: Rc<RefCell<TerminalImeState>>,
    theme: Theme,
}

impl TerminalElement {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(TerminalState::new())),
            ime: Rc::new(RefCell::new(TerminalImeState::default())),
            theme: Theme::dark(),
        }
    }
}

impl CustomElement for TerminalElement {
    fn render(
        &mut self,
        ctx: CustomRenderContext,
        window: &mut gpui::Window,
        _cx: &mut gpui::Context<crate::renderer::GpuixView>,
    ) -> gpui::AnyElement {
        use gpui::prelude::*;

        let theme = self.theme.clone();
        let metrics = theme.metrics;
        let mono = font(theme.font_mono.clone());
        let frame = terminal_frame(&self.state.borrow(), &theme, &mono);

        let font_id = window.text_system().resolve_font(&mono);
        let cell_width = match window
            .text_system()
            .ch_advance(font_id, px(metrics.terminal_text_size))
        {
            Ok(width) => Some(f32::from(width)),
            Err(error) => {
                log::error!("failed to measure terminal cell width: {error:#}");
                None
            }
        };

        let mut content = div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .px(px(metrics.terminal_padding_x))
            .py(px(metrics.terminal_padding_y))
            .font_family(theme.font_mono.clone())
            .text_size(px(metrics.terminal_text_size))
            .line_height(px(metrics.terminal_line_height))
            .whitespace_nowrap();

        for (row_ix, row) in frame.rows.into_iter().enumerate() {
            content = content.child(
                div()
                    .h(px(metrics.terminal_line_height))
                    .flex_none()
                    .child(ctx.text(row_ix, row.text, Some(row.runs))),
            );
        }

        let marked_text = self.ime.borrow().marked_text.clone();
        if let (Some((row, col)), Some(cell_width)) = (frame.cursor, cell_width) {
            if !marked_text.is_empty() {
                let marked_len = marked_text.len();
                content = content.child(
                    div()
                        .absolute()
                        .left(px(metrics.terminal_padding_x + col as f32 * cell_width))
                        .top(px(
                            metrics.terminal_padding_y + row as f32 * metrics.terminal_line_height
                        ))
                        .h(px(metrics.terminal_line_height))
                        .bg(theme.bg)
                        .child(ctx.chrome_text(
                            marked_text,
                            Some(vec![TextRun {
                                len: marked_len,
                                font: mono.clone(),
                                color: theme.text,
                                background_color: Some(theme.bg),
                                underline: Some(UnderlineStyle {
                                    thickness: px(1.0),
                                    color: Some(theme.caret),
                                    wavy: false,
                                }),
                                strikethrough: None,
                            }]),
                        )),
                );
            }
        }

        let state = self.state.clone();
        let callback = ctx.event_callback.clone();
        let emits_resize = ctx.events.contains("terminalResize");
        let id = ctx.id;
        let resize_probe = gpui::canvas(
            move |bounds, window, _cx| {
                let Some(cell_width) = cell_width.filter(|width| *width > 0.0) else {
                    return;
                };
                let width = f32::from(bounds.size.width);
                let height = f32::from(bounds.size.height);
                let content_width = (width - metrics.terminal_padding_x * 2.0).max(cell_width);
                let content_height =
                    (height - metrics.terminal_padding_y * 2.0).max(metrics.terminal_line_height);
                let cols = (content_width / cell_width).floor().max(2.0) as usize;
                let rows = (content_height / metrics.terminal_line_height)
                    .floor()
                    .max(2.0) as usize;
                if state.borrow_mut().resize(rows, cols) {
                    if emits_resize {
                        emit_event_full(&callback, id, "terminalResize", |payload| {
                            payload.rows = Some(rows as f64);
                            payload.cols = Some(cols as f64);
                        });
                    }
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let focus_handle = ctx.focus_handle.cloned();
        let callback = ctx.event_callback.clone();
        let ime = self.ime.clone();
        let cursor = frame.cursor;
        let input_probe = gpui::canvas(
            move |bounds, _window, _cx| bounds,
            move |bounds, _, window, cx| {
                let (Some(focus_handle), Some(cell_width), Some((row, col))) =
                    (focus_handle, cell_width, cursor)
                else {
                    return;
                };
                let cursor_bounds = Bounds::new(
                    point(
                        bounds.origin.x + px(metrics.terminal_padding_x + col as f32 * cell_width),
                        bounds.origin.y
                            + px(metrics.terminal_padding_y
                                + row as f32 * metrics.terminal_line_height),
                    ),
                    gpui::size(px(cell_width), px(metrics.terminal_line_height)),
                );
                window.handle_input(
                    &focus_handle,
                    TerminalInputHandler {
                        callback,
                        element_id: id,
                        ime,
                        element_bounds: bounds,
                        cursor_bounds,
                    },
                    cx,
                );
            },
        )
        .absolute()
        .size_full();

        let state = self.state.clone();
        let interaction_layer =
            div()
                .absolute()
                .size_full()
                .on_scroll_wheel(move |event, window, _cx| {
                    let delta = match event.delta {
                        ScrollDelta::Pixels(point) => {
                            let lines = f32::from(point.y) / metrics.terminal_line_height;
                            lines.round() as i32
                        }
                        ScrollDelta::Lines(point) => point.y.round() as i32,
                    };
                    if delta != 0 {
                        state.borrow_mut().term.scroll_display(Scroll::Delta(delta));
                        window.refresh();
                    }
                });

        let element_id = gpui::SharedString::from(format!("__gpuix_terminal_{}", ctx.id));
        let mut root = div()
            .id(element_id)
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(theme.bg)
            .child(crate::automation::bounds_tracker(ctx.id, None))
            .child(content)
            .child(resize_probe)
            .child(input_probe)
            .child(interaction_layer);
        if let Some(style) = ctx.style {
            root = crate::renderer::apply_styles(root, style);
        }
        if let Some(focus_handle) = ctx.focus_handle {
            root = root.track_focus(focus_handle);
            let focus_handle = focus_handle.clone();
            root = root.on_click(move |_event, window, cx| {
                window.focus(&focus_handle, cx);
            });
        }

        if ctx.events.contains("terminalInput") {
            let callback = ctx.event_callback.clone();
            let id = ctx.id;
            root = root.on_key_down(move |event, _window, cx| {
                let Some(bytes) = terminal_key_bytes(event) else {
                    return;
                };
                emit_terminal_input(&callback, id, &bytes);
                cx.stop_propagation();
            });
        }

        root.into_any_element()
    }

    fn set_prop(&mut self, key: &str, value: serde_json::Value) {
        if key == "theme" {
            self.theme = Theme::from_prop(Some(&value));
        }
    }

    fn supported_props(&self) -> &'static [&'static str] {
        &["theme"]
    }

    fn supported_events(&self) -> &'static [&'static str] {
        &["terminalInput", "terminalResize"]
    }

    fn destroy(&mut self) {}

    fn command(&mut self, command: &str, payload: &[u8]) -> Result<(), String> {
        match command {
            "write" => {
                self.state.borrow_mut().write(payload);
                Ok(())
            }
            "reset" => {
                self.state.borrow_mut().reset();
                Ok(())
            }
            _ => Err(format!("unsupported terminal command: {command}")),
        }
    }
}

#[derive(Default)]
struct TerminalImeState {
    marked_text: String,
    selected_range: Range<usize>,
}

impl TerminalImeState {
    fn replace_marked(&mut self, text: &str, selected_range: Option<Range<usize>>) {
        self.marked_text.clear();
        self.marked_text.push_str(text);
        let len = text.encode_utf16().count();
        self.selected_range = selected_range
            .map(|range| range.start.min(len)..range.end.min(len))
            .unwrap_or(len..len);
    }

    fn clear(&mut self) {
        self.marked_text.clear();
        self.selected_range = 0..0;
    }

    fn marked_range(&self) -> Option<Range<usize>> {
        (!self.marked_text.is_empty()).then(|| 0..self.marked_text.encode_utf16().count())
    }
}

struct TerminalInputHandler {
    callback: Option<crate::renderer::EventCallback>,
    element_id: u64,
    ime: Rc<RefCell<TerminalImeState>>,
    element_bounds: Bounds<Pixels>,
    cursor_bounds: Bounds<Pixels>,
}

impl InputHandler for TerminalInputHandler {
    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.ime.borrow().selected_range.clone(),
            reversed: false,
        })
    }

    fn marked_text_range(&mut self, _window: &mut Window, _cx: &mut App) -> Option<Range<usize>> {
        self.ime.borrow().marked_range()
    }

    fn text_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    fn replace_text_in_range(
        &mut self,
        _replacement_range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        _cx: &mut App,
    ) {
        self.ime.borrow_mut().clear();
        if !text.is_empty() {
            let input = text.replace("\r\n", "\r").replace('\n', "\r");
            emit_terminal_input(&self.callback, self.element_id, input.as_bytes());
        }
        window.refresh();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        _cx: &mut App,
    ) {
        self.ime
            .borrow_mut()
            .replace_marked(new_text, new_selected_range);
        window.refresh();
    }

    fn unmark_text(&mut self, window: &mut Window, _cx: &mut App) {
        self.ime.borrow_mut().clear();
        window.refresh();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        let mut bounds = self.cursor_bounds;
        bounds.origin.x += bounds.size.width * range_utf16.start as f32;
        Some(bounds)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<usize> {
        let cell_width = f32::from(self.cursor_bounds.size.width);
        if cell_width <= 0.0 {
            return Some(0);
        }
        let offset = ((f32::from(point.x - self.cursor_bounds.origin.x) / cell_width).floor()
            as isize)
            .max(0) as usize;
        Some(offset.min(self.ime.borrow().marked_text.encode_utf16().count()))
    }

    fn element_bounds(&mut self, _window: &mut Window, _cx: &mut App) -> Option<Bounds<Pixels>> {
        Some(self.element_bounds)
    }
}

fn emit_terminal_input(
    callback: &Option<crate::renderer::EventCallback>,
    element_id: u64,
    bytes: &[u8],
) {
    emit_event_full(callback, element_id, "terminalInput", |payload| {
        payload.data_base64 = Some(base64::engine::general_purpose::STANDARD.encode(bytes));
    });
}

struct TerminalRow {
    text: String,
    runs: Vec<TextRun>,
}

struct TerminalFrame {
    rows: Vec<TerminalRow>,
    cursor: Option<(usize, usize)>,
}

#[derive(Clone, PartialEq)]
struct CellStyle {
    foreground: Hsla,
    background: Hsla,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
}

fn terminal_frame(state: &TerminalState, theme: &Theme, mono: &Font) -> TerminalFrame {
    let content = state.term.renderable_content();
    let cursor = content.cursor;
    let cursor_visible = cursor.shape != alacritty_terminal::vte::ansi::CursorShape::Hidden;
    let mut rows = Vec::<TerminalRow>::new();
    let mut line = None;
    let mut current = TerminalRow {
        text: String::new(),
        runs: Vec::new(),
    };
    let mut last_style: Option<CellStyle> = None;
    let mut cursor_cell = None;

    for indexed in content.display_iter {
        if line != Some(indexed.point.line.0) {
            if line.is_some() {
                rows.push(current);
                current = TerminalRow {
                    text: String::new(),
                    runs: Vec::new(),
                };
                last_style = None;
            }
            line = Some(indexed.point.line.0);
        }

        let cell = indexed.cell;
        if indexed.point == cursor.point {
            cursor_cell = Some((rows.len(), indexed.point.column.0));
        }
        if cell
            .flags
            .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }
        let is_cursor = cursor_visible && cursor.point == indexed.point;
        let mut foreground = terminal_color(cell.fg, theme);
        let mut background = terminal_color(cell.bg, theme);
        if cell.flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut foreground, &mut background);
        }
        if is_cursor {
            background = theme.caret;
            foreground = theme.bg;
        }
        if cell.flags.contains(Flags::DIM) {
            foreground = foreground.alpha(0.65);
        }
        let style = CellStyle {
            foreground,
            background,
            bold: cell.flags.contains(Flags::BOLD),
            italic: cell.flags.contains(Flags::ITALIC | Flags::BOLD_ITALIC),
            underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
            strike: cell.flags.contains(Flags::STRIKEOUT),
        };

        let start = current.text.len();
        let character = if cell.flags.contains(Flags::HIDDEN) {
            ' '
        } else {
            cell.c
        };
        current.text.push(character);
        if let Some(zerowidth) = cell.zerowidth() {
            current.text.extend(zerowidth);
        }
        let len = current.text.len() - start;
        if last_style.as_ref() == Some(&style) {
            if let Some(run) = current.runs.last_mut() {
                run.len += len;
            }
        } else {
            current.runs.push(text_run(len, mono, &style));
            last_style = Some(style);
        }
    }
    if line.is_some() {
        rows.push(current);
    }
    TerminalFrame {
        rows,
        cursor: cursor_cell,
    }
}

fn text_run(len: usize, mono: &Font, style: &CellStyle) -> TextRun {
    let mut font = mono.clone();
    if style.bold {
        font.weight = FontWeight::BOLD;
    }
    if style.italic {
        font.style = FontStyle::Italic;
    }
    TextRun {
        len,
        font,
        color: style.foreground,
        background_color: Some(style.background),
        underline: style.underline.then_some(UnderlineStyle {
            thickness: px(1.0),
            color: Some(style.foreground),
            wavy: false,
        }),
        strikethrough: style.strike.then_some(StrikethroughStyle {
            thickness: px(1.0),
            color: Some(style.foreground),
        }),
    }
}

fn terminal_color(color: Color, theme: &Theme) -> Hsla {
    match color {
        Color::Spec(rgb) => rgb_color(rgb),
        Color::Indexed(index) => indexed_color(index),
        Color::Named(named) => named_color(named, theme),
    }
}

fn rgb_color(color: Rgb) -> Hsla {
    gpui::rgb(u32::from_be_bytes([0, color.r, color.g, color.b])).into()
}

fn indexed_color(index: u8) -> Hsla {
    if index < 16 {
        return palette(index as usize);
    }
    if index < 232 {
        let value = index - 16;
        let channel = |part: u8| if part == 0 { 0 } else { 55 + part * 40 };
        return rgb_color(Rgb {
            r: channel(value / 36),
            g: channel((value % 36) / 6),
            b: channel(value % 6),
        });
    }
    let grey = 8 + (index - 232) * 10;
    rgb_color(Rgb {
        r: grey,
        g: grey,
        b: grey,
    })
}

fn named_color(color: NamedColor, theme: &Theme) -> Hsla {
    match color {
        NamedColor::Foreground | NamedColor::BrightForeground => theme.text,
        NamedColor::Background => theme.bg,
        NamedColor::Cursor => theme.caret,
        NamedColor::DimForeground => theme.text_muted,
        NamedColor::Black => palette(0),
        NamedColor::Red => palette(1),
        NamedColor::Green => palette(2),
        NamedColor::Yellow => palette(3),
        NamedColor::Blue => palette(4),
        NamedColor::Magenta => palette(5),
        NamedColor::Cyan => palette(6),
        NamedColor::White => palette(7),
        NamedColor::BrightBlack => palette(8),
        NamedColor::BrightRed => palette(9),
        NamedColor::BrightGreen => palette(10),
        NamedColor::BrightYellow => palette(11),
        NamedColor::BrightBlue => palette(12),
        NamedColor::BrightMagenta => palette(13),
        NamedColor::BrightCyan => palette(14),
        NamedColor::BrightWhite => palette(15),
        NamedColor::DimBlack => palette(0).alpha(0.65),
        NamedColor::DimRed => palette(1).alpha(0.65),
        NamedColor::DimGreen => palette(2).alpha(0.65),
        NamedColor::DimYellow => palette(3).alpha(0.65),
        NamedColor::DimBlue => palette(4).alpha(0.65),
        NamedColor::DimMagenta => palette(5).alpha(0.65),
        NamedColor::DimCyan => palette(6).alpha(0.65),
        NamedColor::DimWhite => palette(7).alpha(0.65),
    }
}

fn palette(index: usize) -> Hsla {
    const COLORS: [u32; 16] = [
        0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6, 0x666666,
        0xd54e53, 0xb9ca4a, 0xe7c547, 0x7aa6da, 0xc397d8, 0x70c0b1, 0xeaeaea,
    ];
    gpui::rgb(COLORS[index]).into()
}

fn terminal_key_bytes(event: &KeyDownEvent) -> Option<Vec<u8>> {
    let key = event.keystroke.key.as_str();
    let modifiers = event.keystroke.modifiers;
    if event.keystroke.key_char.is_some() && !modifiers.control && !modifiers.alt {
        return None;
    }
    let mut bytes = match key {
        "enter" => b"\r".to_vec(),
        "tab" => b"\t".to_vec(),
        "backspace" => vec![0x7f],
        "escape" => vec![0x1b],
        "up" => b"\x1b[A".to_vec(),
        "down" => b"\x1b[B".to_vec(),
        "right" => b"\x1b[C".to_vec(),
        "left" => b"\x1b[D".to_vec(),
        "home" => b"\x1b[H".to_vec(),
        "end" => b"\x1b[F".to_vec(),
        "pageup" => b"\x1b[5~".to_vec(),
        "pagedown" => b"\x1b[6~".to_vec(),
        "delete" => b"\x1b[3~".to_vec(),
        _ if modifiers.control => {
            let character = key.as_bytes().first().copied()?;
            if character.is_ascii_alphabetic() {
                vec![character.to_ascii_uppercase() - b'@']
            } else {
                return None;
            }
        }
        _ if modifiers.platform => return None,
        _ => event.keystroke.key_char.as_ref()?.as_bytes().to_vec(),
    };
    if modifiers.alt && key != "escape" {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vt_output_is_parsed_into_visible_rows() {
        let mut state = TerminalState::new();
        state.write(b"plain \x1b[31mred\x1b[0m");
        let frame = terminal_frame(&state, &Theme::dark(), &font("Menlo"));
        assert!(frame.rows.iter().any(|row| row.text.contains("plain red")));
        assert!(frame.rows.iter().any(|row| row.runs.len() >= 2));
    }

    #[test]
    fn resize_changes_only_real_viewport_changes() {
        let mut state = TerminalState::new();
        assert!(!state.resize(INITIAL_ROWS, INITIAL_COLS));
        assert!(state.resize(30, 100));
        assert_eq!(state.size.rows, 30);
        assert_eq!(state.size.cols, 100);
    }

    #[test]
    fn ime_state_tracks_utf16_composition_ranges() {
        let mut ime = TerminalImeState::default();
        ime.replace_marked("日本語", Some(1..2));
        assert_eq!(ime.marked_range(), Some(0..3));
        assert_eq!(ime.selected_range, 1..2);

        ime.replace_marked("😀", Some(9..12));
        assert_eq!(ime.marked_range(), Some(0..2));
        assert_eq!(ime.selected_range, 2..2);

        ime.clear();
        assert_eq!(ime.marked_range(), None);
        assert_eq!(ime.selected_range, 0..0);
    }
}
