use gpui::Rgba;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub type HyperlinkRef = Arc<String>;

pub const DEFAULT_COLS: usize = 80;
pub const DEFAULT_ROWS: usize = 24;
pub const DEFAULT_SCROLLBACK: usize = 5000;

#[derive(Clone, Debug, PartialEq)]
pub struct CellStyle {
    pub foreground: Rgba,
    pub background: Rgba,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub underline_style: UnderlineStyle,
    pub underline_color: Option<Rgba>,
    pub strikethrough: bool,
    pub dim: bool,
    pub inverse: bool,
    pub blink: bool,
    pub hidden: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            foreground: Rgba {
                r: 0.93,
                g: 0.93,
                b: 0.93,
                a: 1.0,
            },
            background: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            bold: false,
            italic: false,
            underline: false,
            underline_style: UnderlineStyle::None,
            underline_color: None,
            strikethrough: false,
            dim: false,
            inverse: false,
            blink: false,
            hidden: false,
        }
    }
}

impl CellStyle {
    pub fn effective_fg(&self) -> Rgba {
        if self.inverse {
            if self.background.a < 0.01 {
                Rgba {
                    r: 0.1,
                    g: 0.1,
                    b: 0.1,
                    a: 1.0,
                }
            } else {
                self.background
            }
        } else if self.hidden {
            self.background
        } else {
            let mut fg = self.foreground;
            if self.dim {
                fg.r *= 0.6;
                fg.g *= 0.6;
                fg.b *= 0.6;
            }
            fg
        }
    }

    pub fn effective_bg(&self) -> Rgba {
        if self.inverse {
            self.foreground
        } else {
            self.background
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ImageCellKind {
    #[default]
    None,
    Anchor(u32),
    Continuation(u32),
}

#[derive(Clone, Debug)]
pub struct TerminalImage {
    pub data: Arc<gpui::Image>,
    pub display_cols: usize,
    pub display_rows: usize,
}

#[derive(Clone, Debug)]
pub struct ImagePlacement {
    pub image: Arc<TerminalImage>,
    pub anchor_line: usize,
    pub anchor_col: usize,
}

#[derive(Clone, Debug)]
pub struct TerminalCell {
    pub char: char,
    pub style: CellStyle,
    pub width: u8,
    pub hyperlink: Option<HyperlinkRef>,
    pub image_cell: ImageCellKind,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            char: ' ',
            style: CellStyle::default(),
            width: 1,
            hyperlink: None,
            image_cell: ImageCellKind::None,
        }
    }
}

impl TerminalCell {
    pub fn new(char: char, style: CellStyle) -> Self {
        let width = if char.is_ascii() {
            1
        } else {
            unicode_width::UnicodeWidthChar::width(char).unwrap_or(1) as u8
        };
        Self {
            char,
            style,
            width,
            hyperlink: None,
            image_cell: ImageCellKind::None,
        }
    }

    pub fn with_hyperlink(mut self, url: Option<HyperlinkRef>) -> Self {
        self.hyperlink = url;
        self
    }
}

#[derive(Clone, Debug)]
pub struct TerminalLine {
    pub cells: Vec<TerminalCell>,
    pub wrapped: bool,
}

fn blank_cell(style: &CellStyle) -> TerminalCell {
    TerminalCell {
        char: ' ',
        style: style.clone(),
        width: 1,
        hyperlink: None,
        image_cell: ImageCellKind::None,
    }
}

impl TerminalLine {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![TerminalCell::default(); cols],
            wrapped: false,
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn get(&self, col: usize) -> Option<&TerminalCell> {
        self.cells.get(col)
    }

    pub fn set(&mut self, col: usize, cell: TerminalCell) {
        if col < self.cells.len() {
            self.cells[col] = cell;
        }
    }

    pub fn clear_with_style(&mut self, style: &CellStyle) {
        let blank = blank_cell(style);
        for cell in &mut self.cells {
            *cell = blank.clone();
        }
        self.wrapped = false;
    }

    pub fn clear_from_with_style(&mut self, col: usize, style: &CellStyle) {
        let blank = blank_cell(style);
        for i in col..self.cells.len() {
            self.cells[i] = blank.clone();
        }
    }

    pub fn clear_to_with_style(&mut self, col: usize, style: &CellStyle) {
        let blank = blank_cell(style);
        let end = col.min(self.cells.len());
        for i in 0..end {
            self.cells[i] = blank.clone();
        }
    }

    pub fn resize(&mut self, cols: usize) {
        self.cells.resize_with(cols, TerminalCell::default);
    }

    pub fn insert_cells(&mut self, col: usize, count: usize) {
        let cols = self.cells.len();
        for _ in 0..count {
            if col < cols {
                self.cells.insert(col, TerminalCell::default());
                self.cells.truncate(cols);
            }
        }
    }

    pub fn delete_cells(&mut self, col: usize, count: usize) {
        let cols = self.cells.len();
        for _ in 0..count {
            if col < self.cells.len() {
                self.cells.remove(col);
                self.cells.push(TerminalCell::default());
            }
        }
        self.cells.truncate(cols);
    }

    pub fn erase_chars(&mut self, col: usize, count: usize, style: &CellStyle) {
        let blank = blank_cell(style);
        let end = (col + count).min(self.cells.len());
        for i in col..end {
            self.cells[i] = blank.clone();
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Charset {
    #[default]
    Ascii,
    DecSpecialGraphics,
}

impl Charset {
    pub fn translate(&self, c: char) -> char {
        match self {
            Charset::Ascii => c,
            Charset::DecSpecialGraphics => match c {
                'j' => '┘',
                'k' => '┐',
                'l' => '┌',
                'm' => '└',
                'n' => '┼',
                'q' => '─',
                't' => '├',
                'u' => '┤',
                'v' => '┴',
                'w' => '┬',
                'x' => '│',
                'a' => '▒',
                'f' => '°',
                'g' => '±',
                'h' => '░',
                'i' => '⎺',
                'o' => '⎻',
                'p' => '⎼',
                'r' => '⎽',
                's' => '⎺',
                '`' => '◆',
                '~' => '·',
                'y' => '≤',
                'z' => '≥',
                '{' => 'π',
                '|' => '≠',
                '}' => '£',
                _ => c,
            },
        }
    }
}

impl CursorPosition {
    pub fn origin() -> Self {
        Self { row: 0, col: 0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

#[derive(Clone, Debug)]
struct SavedCursor {
    position: CursorPosition,
    style: CellStyle,
    origin_mode: bool,
    autowrap: bool,
    g0_charset: Charset,
    g1_charset: Charset,
    active_charset: u8,
}

#[derive(Clone, Debug)]
pub struct TerminalState {
    lines: VecDeque<TerminalLine>,
    cursor: CursorPosition,
    cols: usize,
    rows: usize,
    scroll_offset: usize,
    max_scrollback: usize,
    working_directory: PathBuf,
    is_running: bool,
    current_style: CellStyle,
    cursor_visible: bool,
    cursor_style: CursorStyle,
    saved_cursor: Option<SavedCursor>,
    title: Option<String>,

    scroll_region_top: usize,
    scroll_region_bottom: usize,
    origin_mode: bool,
    autowrap: bool,
    insert_mode: bool,

    alt_screen: Option<Box<AltScreenState>>,
    use_alt_screen: bool,

    bracketed_paste: bool,
    mouse_mode: u16,
    sgr_mouse: bool,
    focus_tracking: bool,
    application_cursor_keys: bool,

    user_scrolled: bool,

    tabs: Vec<usize>,

    g0_charset: Charset,
    g1_charset: Charset,
    active_charset: u8,
    current_hyperlink: Option<HyperlinkRef>,

    image_placements: Vec<ImagePlacement>,
    next_image_id: u32,

    sync_update_active: bool,
    sync_update_start: Option<Instant>,
    dirty_lines: HashSet<usize>,
    all_dirty: bool,
    application_keypad: bool,
    tab_stops: Vec<bool>,
    title_stack: Vec<String>,
    keyboard_mode_stack: Vec<u32>,
}

#[derive(Clone, Debug)]
struct AltScreenState {
    lines: VecDeque<TerminalLine>,
    cursor: CursorPosition,
    saved_cursor: Option<SavedCursor>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::new(DEFAULT_COLS, DEFAULT_ROWS)
    }
}

impl TerminalState {
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut lines = VecDeque::with_capacity(rows);
        for _ in 0..rows {
            lines.push_back(TerminalLine::new(cols));
        }

        let mut tabs = Vec::new();
        for i in (0..cols).step_by(8) {
            tabs.push(i);
        }

        let mut tab_stops = vec![false; cols];
        for i in (0..cols).step_by(8) {
            tab_stops[i] = true;
        }

        Self {
            lines,
            cursor: CursorPosition::origin(),
            cols,
            rows,
            scroll_offset: 0,
            max_scrollback: DEFAULT_SCROLLBACK,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            is_running: false,
            current_style: CellStyle::default(),
            cursor_visible: true,
            cursor_style: CursorStyle::default(),
            saved_cursor: None,
            title: None,
            scroll_region_top: 0,
            scroll_region_bottom: rows.saturating_sub(1),
            origin_mode: false,
            autowrap: true,
            insert_mode: false,
            alt_screen: None,
            use_alt_screen: false,
            bracketed_paste: false,
            mouse_mode: 0,
            sgr_mouse: false,
            focus_tracking: false,
            application_cursor_keys: false,
            user_scrolled: false,
            tabs,
            g0_charset: Charset::Ascii,
            g1_charset: Charset::Ascii,
            active_charset: 0,
            current_hyperlink: None,
            image_placements: Vec::new(),
            next_image_id: 0,
            sync_update_active: false,
            sync_update_start: None,
            dirty_lines: HashSet::new(),
            all_dirty: true,
            application_keypad: false,
            tab_stops,
            title_stack: Vec::new(),
            keyboard_mode_stack: Vec::new(),
        }
    }

    pub fn with_working_directory(mut self, path: PathBuf) -> Self {
        self.working_directory = path;
        self
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn cursor_style(&self) -> CursorStyle {
        self.cursor_style
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn working_directory(&self) -> &PathBuf {
        &self.working_directory
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn scrollback_lines(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }

    pub fn max_scroll_offset(&self) -> usize {
        self.scrollback_lines()
    }

    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    pub fn bracketed_paste(&self) -> bool {
        self.bracketed_paste
    }

    pub fn set_running(&mut self, running: bool) {
        self.is_running = running;
    }

    pub fn set_working_directory(&mut self, path: PathBuf) {
        self.working_directory = path;
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    pub fn push_title(&mut self) {
        if self.title_stack.len() < 10 {
            self.title_stack
                .push(self.title.clone().unwrap_or_default());
        }
    }

    pub fn pop_title(&mut self) -> Option<String> {
        self.title_stack.pop()
    }

    pub fn set_current_style(&mut self, style: CellStyle) {
        self.current_style = style;
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn set_cursor_style(&mut self, style: CursorStyle) {
        self.cursor_style = style;
    }

    pub fn current_sgr_string(&self) -> String {
        let mut parts = Vec::new();
        if self.current_style.bold {
            parts.push("1".to_string());
        }
        if self.current_style.dim {
            parts.push("2".to_string());
        }
        if self.current_style.italic {
            parts.push("3".to_string());
        }
        if self.current_style.underline {
            parts.push("4".to_string());
        }
        if self.current_style.blink {
            parts.push("5".to_string());
        }
        if self.current_style.inverse {
            parts.push("7".to_string());
        }
        if self.current_style.hidden {
            parts.push("8".to_string());
        }
        if self.current_style.strikethrough {
            parts.push("9".to_string());
        }
        if parts.is_empty() {
            "0".to_string()
        } else {
            parts.join(";")
        }
    }

    pub fn scroll_region(&self) -> (usize, usize) {
        (self.scroll_region_top, self.scroll_region_bottom)
    }

    pub fn cursor_style_code(&self) -> u8 {
        match self.cursor_style {
            CursorStyle::Block => 2,
            CursorStyle::Underline => 4,
            CursorStyle::Bar => 6,
        }
    }

    pub fn keyboard_mode_flags(&self) -> u32 {
        self.keyboard_mode_stack.last().copied().unwrap_or(0)
    }

    pub fn push_keyboard_mode(&mut self, flags: u32) {
        if self.keyboard_mode_stack.len() < 8 {
            self.keyboard_mode_stack.push(flags);
        }
    }

    pub fn pop_keyboard_mode(&mut self, n: u32) {
        for _ in 0..n {
            if self.keyboard_mode_stack.pop().is_none() {
                break;
            }
        }
    }

    pub fn set_keyboard_mode(&mut self, flags: u32, mode: u8) {
        let current = self.keyboard_mode_flags();
        let new_flags = match mode {
            1 => flags,
            2 => current | flags,
            3 => current & !flags,
            _ => flags,
        };
        if let Some(last) = self.keyboard_mode_stack.last_mut() {
            *last = new_flags;
        } else {
            self.keyboard_mode_stack.push(new_flags);
        }
    }

    pub fn set_bracketed_paste(&mut self, enabled: bool) {
        self.bracketed_paste = enabled;
    }

    pub fn mouse_mode(&self) -> u16 {
        self.mouse_mode
    }

    pub fn sgr_mouse(&self) -> bool {
        self.sgr_mouse
    }

    pub fn mouse_tracking(&self) -> bool {
        self.mouse_mode > 0
    }

    pub fn set_mouse_mode(&mut self, mode: u16, enabled: bool) {
        match mode {
            1000 | 1002 | 1003 => {
                self.mouse_mode = if enabled { mode } else { 0 };
            }
            1006 | 1015 => {
                self.sgr_mouse = enabled;
            }
            _ => {}
        }
    }

    pub fn focus_tracking(&self) -> bool {
        self.focus_tracking
    }

    pub fn set_focus_tracking(&mut self, enabled: bool) {
        self.focus_tracking = enabled;
    }

    pub fn application_cursor_keys(&self) -> bool {
        self.application_cursor_keys
    }

    pub fn set_application_cursor_keys(&mut self, enabled: bool) {
        self.application_cursor_keys = enabled;
    }

    pub fn set_g0_charset(&mut self, charset: Charset) {
        self.g0_charset = charset;
    }

    pub fn set_g1_charset(&mut self, charset: Charset) {
        self.g1_charset = charset;
    }

    pub fn shift_in(&mut self) {
        self.active_charset = 0;
    }

    pub fn shift_out(&mut self) {
        self.active_charset = 1;
    }

    fn current_charset(&self) -> Charset {
        if self.active_charset == 0 {
            self.g0_charset
        } else {
            self.g1_charset
        }
    }

    pub fn set_hyperlink(&mut self, url: Option<String>) {
        self.current_hyperlink = url.map(|s| Arc::new(s));
    }

    pub fn sync_update_active(&self) -> bool {
        self.sync_update_active
    }

    pub fn sync_update_start(&self) -> Option<Instant> {
        self.sync_update_start
    }

    pub fn set_sync_update(&mut self, active: bool) {
        if active {
            self.sync_update_active = true;
            self.sync_update_start = Some(Instant::now());
        } else {
            self.sync_update_active = false;
            self.sync_update_start = None;
            self.mark_all_dirty();
        }
    }

    pub fn mark_line_dirty(&mut self, abs_line: usize) {
        if !self.all_dirty {
            self.dirty_lines.insert(abs_line);
        }
    }

    pub fn mark_all_dirty(&mut self) {
        self.all_dirty = true;
        self.dirty_lines.clear();
    }

    pub fn clear_dirty(&mut self) {
        self.all_dirty = false;
        self.dirty_lines.clear();
    }

    pub fn set_application_keypad(&mut self, enabled: bool) {
        self.application_keypad = enabled;
    }

    pub fn set_tab_stop(&mut self) {
        let col = self.cursor.col;
        if col < self.tab_stops.len() {
            self.tab_stops[col] = true;
        }
    }

    pub fn clear_tab_stop_at_cursor(&mut self) {
        let col = self.cursor.col;
        if col < self.tab_stops.len() {
            self.tab_stops[col] = false;
        }
    }

    pub fn clear_all_tab_stops(&mut self) {
        for stop in &mut self.tab_stops {
            *stop = false;
        }
    }

    pub fn set_insert_mode(&mut self, enabled: bool) {
        self.insert_mode = enabled;
    }

    pub fn repeat_last_char(&mut self, count: usize) {
        let abs = self.viewport_to_absolute(self.cursor.row);
        let col = self.cursor.col.saturating_sub(1);
        let ch = self
            .lines
            .get(abs)
            .and_then(|l| l.get(col))
            .map(|c| c.char)
            .unwrap_or(' ');
        if ch == ' ' && self.cursor.col == 0 {
            return;
        }
        for _ in 0..count {
            self.write_char(ch);
        }
    }

    pub fn screen_alignment_test(&mut self) {
        let start = self.lines.len().saturating_sub(self.rows);
        for i in start..self.lines.len() {
            for cell in &mut self.lines[i].cells {
                cell.char = 'E';
                cell.style = CellStyle::default();
                cell.width = 1;
            }
        }
        self.cursor = CursorPosition::origin();
        self.scroll_region_top = 0;
        self.scroll_region_bottom = self.rows.saturating_sub(1);
        self.mark_all_dirty();
    }

    pub fn line(&self, index: usize) -> Option<&TerminalLine> {
        self.lines.get(index)
    }

    fn viewport_to_absolute(&self, row: usize) -> usize {
        let total = self.lines.len();
        total.saturating_sub(self.rows) + row
    }

    fn current_line_mut(&mut self) -> &mut TerminalLine {
        let idx = self.viewport_to_absolute(self.cursor.row);
        while self.lines.len() <= idx {
            self.lines.push_back(TerminalLine::new(self.cols));
        }
        &mut self.lines[idx]
    }

    pub fn write_char(&mut self, c: char) {
        if self.cursor.col >= self.cols {
            if self.autowrap {
                self.current_line_mut().wrapped = true;
                self.newline();
            } else {
                self.cursor.col = self.cols - 1;
            }
        }

        if self.insert_mode {
            let col = self.cursor.col;
            self.current_line_mut().insert_cells(col, 1);
        }

        let translated = self.current_charset().translate(c);
        let style = self.current_style.clone();
        let col = self.cursor.col;
        let cell =
            TerminalCell::new(translated, style).with_hyperlink(self.current_hyperlink.clone());
        let width = cell.width as usize;

        let abs_line = self.viewport_to_absolute(self.cursor.row);
        let line = self.current_line_mut();
        line.set(col, cell);

        for i in 1..width {
            if col + i < line.len() {
                line.set(
                    col + i,
                    TerminalCell {
                        char: ' ',
                        style: CellStyle::default(),
                        width: 0,
                        hyperlink: None,
                        image_cell: ImageCellKind::None,
                    },
                );
            }
        }

        self.mark_line_dirty(abs_line);
        self.cursor.col += width;
    }

    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '\n' => self.newline(),
                '\r' => self.carriage_return(),
                '\t' => self.tab(),
                '\x08' => self.backspace(),
                '\x07' => {}
                c if c.is_control() => {}
                c => self.write_char(c),
            }
        }
    }

    pub fn newline(&mut self) {
        self.cursor.col = 0;
        if self.cursor.row == self.scroll_region_bottom {
            self.scroll_up_region();
        } else if self.cursor.row + 1 < self.rows {
            self.cursor.row += 1;
        }

        if !self.user_scrolled {
            self.scroll_offset = 0;
        }
    }

    pub fn line_feed(&mut self) {
        if self.cursor.row == self.scroll_region_bottom {
            self.scroll_up_region();
        } else if self.cursor.row + 1 < self.rows {
            self.cursor.row += 1;
        }

        if !self.user_scrolled {
            self.scroll_offset = 0;
        }
    }

    pub fn reverse_index(&mut self) {
        if self.cursor.row == self.scroll_region_top {
            self.scroll_down_region();
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    pub fn tab(&mut self) {
        let start = self.cursor.col + 1;
        let next_tab = self.tab_stops[start..]
            .iter()
            .position(|&s| s)
            .map(|p| start + p)
            .unwrap_or(self.cols - 1);
        self.cursor.col = next_tab.min(self.cols - 1);
    }

    pub fn backspace(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    fn scroll_up_region(&mut self) {
        let top = self.scroll_region_top;
        let bottom = self.scroll_region_bottom;
        if bottom <= top {
            return;
        }

        if self.use_alt_screen || (top > 0 || bottom < self.rows.saturating_sub(1)) {
            let remove_idx = self.viewport_to_absolute(top);
            let insert_idx = self.viewport_to_absolute(bottom);
            if remove_idx < self.lines.len() {
                self.lines.remove(remove_idx);
                self.lines.insert(
                    insert_idx.min(self.lines.len()),
                    TerminalLine::new(self.cols),
                );
            }
            while self.lines.len() > self.rows + self.max_scrollback {
                self.lines.pop_front();
            }
        } else {
            self.lines.push_back(TerminalLine::new(self.cols));
            while self.lines.len() > self.rows + self.max_scrollback {
                self.lines.pop_front();
                for placement in &mut self.image_placements {
                    placement.anchor_line = placement.anchor_line.saturating_sub(1);
                }
                self.image_placements
                    .retain(|p| p.anchor_line + p.image.display_rows > 0);
            }
        }
        self.mark_all_dirty();
    }

    fn scroll_down_region(&mut self) {
        let top = self.scroll_region_top;
        let bottom = self.scroll_region_bottom;
        if bottom <= top {
            return;
        }

        let remove_idx = self.viewport_to_absolute(bottom);
        let insert_idx = self.viewport_to_absolute(top);

        if remove_idx < self.lines.len() {
            self.lines.remove(remove_idx);
            self.lines.insert(
                insert_idx.min(self.lines.len()),
                TerminalLine::new(self.cols),
            );
        }
        self.mark_all_dirty();
    }

    pub fn scroll_up_n(&mut self, n: usize) {
        for _ in 0..n {
            self.scroll_up_region();
        }
    }

    pub fn scroll_down_n(&mut self, n: usize) {
        for _ in 0..n {
            self.scroll_down_region();
        }
    }

    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        let max_row = self.rows.saturating_sub(1);
        let max_col = self.cols.saturating_sub(1);

        if self.origin_mode {
            self.cursor.row = (self.scroll_region_top + row).min(self.scroll_region_bottom);
        } else {
            self.cursor.row = row.min(max_row);
        }
        self.cursor.col = col.min(max_col);
    }

    pub fn cursor_up(&mut self, n: usize) {
        let min_row = if self.origin_mode {
            self.scroll_region_top
        } else {
            0
        };
        self.cursor.row = self.cursor.row.saturating_sub(n).max(min_row);
    }

    pub fn cursor_down(&mut self, n: usize) {
        let max_row = if self.origin_mode {
            self.scroll_region_bottom
        } else {
            self.rows.saturating_sub(1)
        };
        self.cursor.row = (self.cursor.row + n).min(max_row);
    }

    pub fn cursor_forward(&mut self, n: usize) {
        self.cursor.col = (self.cursor.col + n).min(self.cols.saturating_sub(1));
    }

    pub fn cursor_backward(&mut self, n: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    pub fn cursor_to_column(&mut self, col: usize) {
        self.cursor.col = col.saturating_sub(1).min(self.cols.saturating_sub(1));
    }

    pub fn cursor_next_line(&mut self, n: usize) {
        self.cursor_down(n);
        self.cursor.col = 0;
    }

    pub fn cursor_prev_line(&mut self, n: usize) {
        self.cursor_up(n);
        self.cursor.col = 0;
    }

    pub fn vertical_position_absolute(&mut self, row: usize) {
        self.cursor.row = row.min(self.rows.saturating_sub(1));
        let abs_line = self.viewport_to_absolute(self.cursor.row);
        self.mark_line_dirty(abs_line);
    }

    pub fn cursor_forward_tab(&mut self, n: usize) {
        for _ in 0..n {
            self.tab();
        }
    }

    pub fn cursor_backward_tab(&mut self, n: usize) {
        for _ in 0..n {
            let col = self.cursor.col;
            if col == 0 {
                break;
            }
            let prev = self.tab_stops[..col].iter().rposition(|&s| s).unwrap_or(0);
            self.cursor.col = prev;
        }
    }

    pub fn is_mode_set(&self, mode: u16) -> Option<bool> {
        match mode {
            1 => Some(self.application_cursor_keys),
            6 => Some(self.origin_mode),
            7 => Some(self.autowrap),
            25 => Some(self.cursor_visible),
            47 | 1047 | 1049 => Some(self.use_alt_screen),
            1000 => Some(self.mouse_mode >= 1000),
            1002 => Some(self.mouse_mode >= 1002),
            1003 => Some(self.mouse_mode >= 1003),
            1004 => Some(self.focus_tracking),
            1006 => Some(self.sgr_mouse),
            2004 => Some(self.bracketed_paste),
            2026 => Some(self.sync_update_active),
            _ => None,
        }
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(SavedCursor {
            position: self.cursor,
            style: self.current_style.clone(),
            origin_mode: self.origin_mode,
            autowrap: self.autowrap,
            g0_charset: self.g0_charset,
            g1_charset: self.g1_charset,
            active_charset: self.active_charset,
        });
    }

    pub fn restore_cursor(&mut self) {
        if let Some(saved) = &self.saved_cursor {
            self.cursor = saved.position;
            self.current_style = saved.style.clone();
            self.origin_mode = saved.origin_mode;
            self.autowrap = saved.autowrap;
            self.g0_charset = saved.g0_charset;
            self.g1_charset = saved.g1_charset;
            self.active_charset = saved.active_charset;
        }
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.min(self.rows.saturating_sub(1));
        let bottom = bottom.min(self.rows.saturating_sub(1));

        if top < bottom {
            self.scroll_region_top = top;
            self.scroll_region_bottom = bottom;
            self.move_cursor_to(0, 0);
        }
    }

    pub fn reset_scroll_region(&mut self) {
        self.scroll_region_top = 0;
        self.scroll_region_bottom = self.rows.saturating_sub(1);
    }

    pub fn set_origin_mode(&mut self, enabled: bool) {
        self.origin_mode = enabled;
        self.move_cursor_to(0, 0);
    }

    pub fn set_autowrap(&mut self, enabled: bool) {
        self.autowrap = enabled;
    }

    pub fn enter_alt_screen(&mut self) {
        if self.use_alt_screen {
            return;
        }

        let mut alt_lines = VecDeque::with_capacity(self.rows);
        for _ in 0..self.rows {
            alt_lines.push_back(TerminalLine::new(self.cols));
        }
        std::mem::swap(&mut self.lines, &mut alt_lines);

        self.alt_screen = Some(Box::new(AltScreenState {
            lines: alt_lines,
            cursor: self.cursor,
            saved_cursor: self.saved_cursor.take(),
        }));

        self.cursor = CursorPosition::origin();
        self.use_alt_screen = true;
        self.scroll_offset = 0;
        self.image_placements.clear();
    }

    pub fn exit_alt_screen(&mut self) {
        if !self.use_alt_screen {
            return;
        }

        if let Some(alt) = self.alt_screen.take() {
            self.lines = alt.lines;
            self.cursor = alt.cursor;
            self.saved_cursor = alt.saved_cursor;
        }

        self.use_alt_screen = false;
        self.mouse_mode = 0;
        self.sgr_mouse = false;
        self.bracketed_paste = false;
        self.origin_mode = false;
    }

    pub fn clear_screen(&mut self) {
        let start = self.lines.len().saturating_sub(self.rows);
        for i in start..self.lines.len() {
            self.lines[i].clear_with_style(&self.current_style);
        }
        self.cursor = CursorPosition::origin();
        self.image_placements
            .retain(|p| p.anchor_line + p.image.display_rows <= start);
        self.mark_all_dirty();
    }

    pub fn clear_scrollback(&mut self) {
        let scrollback = self.lines.len().saturating_sub(self.rows);
        if scrollback > 0 {
            self.lines.drain(..scrollback);
            self.image_placements
                .retain(|p| p.anchor_line >= scrollback);
            for placement in &mut self.image_placements {
                placement.anchor_line -= scrollback;
            }
        }
        self.scroll_offset = 0;
        self.user_scrolled = false;
    }

    pub fn clear_screen_above(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        let start = self.lines.len().saturating_sub(self.rows);

        for i in start..idx {
            self.lines[i].clear_with_style(&self.current_style);
        }
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_to_with_style(self.cursor.col + 1, &self.current_style);
        }
        self.mark_all_dirty();
    }

    pub fn clear_screen_below(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);

        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_from_with_style(self.cursor.col, &self.current_style);
        }
        for i in (idx + 1)..self.lines.len() {
            self.lines[i].clear_with_style(&self.current_style);
        }
        self.mark_all_dirty();
    }

    pub fn clear_to_end_of_screen(&mut self) {
        self.clear_screen_below();
    }

    pub fn clear_to_start_of_screen(&mut self) {
        self.clear_screen_above();
    }

    pub fn clear_line(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_with_style(&self.current_style);
        }
        self.mark_line_dirty(idx);
    }

    pub fn clear_to_end_of_line(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_from_with_style(self.cursor.col, &self.current_style);
        }
        self.mark_line_dirty(idx);
    }

    pub fn clear_to_start_of_line(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_to_with_style(self.cursor.col + 1, &self.current_style);
        }
        self.mark_line_dirty(idx);
    }

    pub fn erase_chars(&mut self, count: usize) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.erase_chars(self.cursor.col, count, &self.current_style);
        }
        self.mark_line_dirty(idx);
    }

    pub fn insert_lines(&mut self, count: usize) {
        let row = self.cursor.row;
        if row < self.scroll_region_top || row > self.scroll_region_bottom {
            return;
        }

        for _ in 0..count {
            let remove_idx = self.viewport_to_absolute(self.scroll_region_bottom);
            let insert_idx = self.viewport_to_absolute(row);
            if remove_idx < self.lines.len() {
                self.lines.remove(remove_idx);
            }
            self.lines.insert(
                insert_idx.min(self.lines.len()),
                TerminalLine::new(self.cols),
            );
        }
        self.mark_all_dirty();
    }

    pub fn delete_lines(&mut self, count: usize) {
        let row = self.cursor.row;
        if row < self.scroll_region_top || row > self.scroll_region_bottom {
            return;
        }

        for _ in 0..count {
            let remove_idx = self.viewport_to_absolute(row);
            let insert_idx = self.viewport_to_absolute(self.scroll_region_bottom);
            if remove_idx < self.lines.len() {
                self.lines.remove(remove_idx);
                self.lines.insert(
                    insert_idx.min(self.lines.len()),
                    TerminalLine::new(self.cols),
                );
            }
        }
        self.mark_all_dirty();
    }

    pub fn insert_chars(&mut self, count: usize) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.insert_cells(self.cursor.col, count);
        }
        self.mark_line_dirty(idx);
    }

    pub fn delete_chars(&mut self, count: usize) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.delete_cells(self.cursor.col, count);
        }
        self.mark_line_dirty(idx);
    }

    pub fn scroll_viewport_up(&mut self, lines: usize) {
        let max = self.max_scroll_offset();
        self.scroll_offset = (self.scroll_offset + lines).min(max);
        self.user_scrolled = self.scroll_offset > 0;
    }

    pub fn scroll_viewport_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.scroll_offset == 0 {
            self.user_scrolled = false;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.user_scrolled = false;
    }

    fn reflow_lines(
        lines: &mut VecDeque<TerminalLine>,
        old_cols: usize,
        new_cols: usize,
        cursor_abs: usize,
        cursor_col: usize,
    ) -> (usize, usize) {
        if old_cols == new_cols {
            return (cursor_abs, cursor_col);
        }

        let mut new_cursor_row = cursor_abs;
        let mut new_cursor_col = cursor_col;

        if new_cols < old_cols {
            let mut i = 0;
            while i < lines.len() {
                let content_end = lines[i]
                    .cells
                    .iter()
                    .rposition(|c| c.char != ' ')
                    .map(|p| p + 1)
                    .unwrap_or(0);

                if content_end > new_cols {
                    let remainder: Vec<TerminalCell> = lines[i].cells.split_off(new_cols);
                    let was_wrapped = lines[i].wrapped;
                    lines[i].wrapped = true;

                    let new_line = TerminalLine {
                        cells: remainder,
                        wrapped: was_wrapped,
                    };

                    if i < new_cursor_row {
                        new_cursor_row += 1;
                    } else if i == new_cursor_row && new_cursor_col >= new_cols {
                        new_cursor_col -= new_cols;
                        new_cursor_row += 1;
                    }

                    lines.insert(i + 1, new_line);
                }
                lines[i].cells.resize(new_cols, TerminalCell::default());
                i += 1;
            }
        } else {
            let mut i = 0;
            while i < lines.len() {
                if lines[i].wrapped && i + 1 < lines.len() {
                    let current_content = lines[i]
                        .cells
                        .iter()
                        .rposition(|c| c.char != ' ')
                        .map(|p| p + 1)
                        .unwrap_or(0);

                    let space = new_cols.saturating_sub(current_content);
                    if space > 0 && i + 1 < lines.len() {
                        let next_line = lines.remove(i + 1).unwrap();
                        let next_content = next_line
                            .cells
                            .iter()
                            .rposition(|c| c.char != ' ')
                            .map(|p| p + 1)
                            .unwrap_or(0);
                        let pull_count = space.min(next_content);

                        lines[i].cells.truncate(current_content);
                        lines[i]
                            .cells
                            .extend_from_slice(&next_line.cells[..pull_count]);
                        lines[i].cells.resize(new_cols, TerminalCell::default());

                        let remaining = next_content.saturating_sub(pull_count);
                        if remaining > 0 {
                            let mut remaining_line = TerminalLine {
                                cells: next_line.cells[pull_count..].to_vec(),
                                wrapped: next_line.wrapped,
                            };
                            remaining_line
                                .cells
                                .resize(new_cols, TerminalCell::default());
                            lines.insert(i + 1, remaining_line);
                            lines[i].wrapped = true;
                        } else {
                            lines[i].wrapped = next_line.wrapped;
                            if i < new_cursor_row {
                                new_cursor_row = new_cursor_row.saturating_sub(1);
                            }
                        }
                        continue;
                    }
                }
                lines[i].cells.resize(new_cols, TerminalCell::default());
                i += 1;
            }
        }

        new_cursor_row = new_cursor_row.min(lines.len().saturating_sub(1));
        new_cursor_col = new_cursor_col.min(new_cols.saturating_sub(1));
        (new_cursor_row, new_cursor_col)
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        if cols == self.cols && rows == self.rows {
            return;
        }

        let old_cols = self.cols;
        let cursor_abs = self.lines.len().saturating_sub(self.rows) + self.cursor.row;

        self.cols = cols;
        self.rows = rows;

        if !self.use_alt_screen {
            let (new_cursor_abs, new_cursor_col) =
                Self::reflow_lines(&mut self.lines, old_cols, cols, cursor_abs, self.cursor.col);

            let viewport_start = if self.lines.len() >= rows {
                self.lines.len() - rows
            } else {
                0
            };

            self.cursor.row = new_cursor_abs
                .saturating_sub(viewport_start)
                .min(rows.saturating_sub(1));
            self.cursor.col = new_cursor_col;

            while self.lines.len() < viewport_start + rows {
                self.lines.push_back(TerminalLine::new(cols));
            }
        } else {
            for line in &mut self.lines {
                line.resize(cols);
            }
            self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
            self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
        }

        self.scroll_region_top = 0;
        self.scroll_region_bottom = rows.saturating_sub(1);
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());

        self.tab_stops.resize(cols, false);
        for (i, stop) in self.tab_stops.iter_mut().enumerate() {
            *stop = i % 8 == 0;
        }

        self.tabs.clear();
        for i in (0..cols).step_by(8) {
            self.tabs.push(i);
        }

        self.mark_all_dirty();

        for placement in &mut self.image_placements {
            if placement.image.display_cols > cols {
                let clamped_cols = cols.saturating_sub(placement.anchor_col).max(1);
                let ratio = clamped_cols as f32 / placement.image.display_cols as f32;
                let new_rows = (placement.image.display_rows as f32 * ratio).ceil() as usize;
                let image = Arc::make_mut(&mut placement.image);
                image.display_cols = clamped_cols;
                image.display_rows = new_rows.max(1);
            }
        }
    }

    pub fn reset(&mut self) {
        self.lines.clear();
        for _ in 0..self.rows {
            self.lines.push_back(TerminalLine::new(self.cols));
        }
        self.cursor = CursorPosition::origin();
        self.scroll_offset = 0;
        self.current_style = CellStyle::default();
        self.cursor_visible = true;
        self.cursor_style = CursorStyle::default();
        self.saved_cursor = None;
        self.title = None;
        self.scroll_region_top = 0;
        self.scroll_region_bottom = self.rows.saturating_sub(1);
        self.origin_mode = false;
        self.autowrap = true;
        self.insert_mode = false;
        self.alt_screen = None;
        self.use_alt_screen = false;
        self.bracketed_paste = false;
        self.mouse_mode = 0;
        self.sgr_mouse = false;
        self.focus_tracking = false;
        self.user_scrolled = false;
        self.g0_charset = Charset::Ascii;
        self.g1_charset = Charset::Ascii;
        self.active_charset = 0;
        self.current_hyperlink = None;
        self.image_placements.clear();
        self.sync_update_active = false;
        self.sync_update_start = None;
        self.application_keypad = false;
        self.tab_stops = vec![false; self.cols];
        for i in (0..self.cols).step_by(8) {
            self.tab_stops[i] = true;
        }
        self.mark_all_dirty();
    }

    pub fn place_image(
        &mut self,
        data: Arc<gpui::Image>,
        display_cols: usize,
        display_rows: usize,
    ) {
        let id = self.next_image_id;
        self.next_image_id = self.next_image_id.wrapping_add(1);

        let anchor_line = self.viewport_to_absolute(self.cursor.row);
        let anchor_col = self.cursor.col;

        let image = Arc::new(TerminalImage {
            data,
            display_cols,
            display_rows,
        });

        for row_offset in 0..display_rows {
            let line_idx = anchor_line + row_offset;
            while self.lines.len() <= line_idx {
                self.lines.push_back(TerminalLine::new(self.cols));
            }
            let line = &mut self.lines[line_idx];
            for col_offset in 0..display_cols {
                let col = anchor_col + col_offset;
                if col >= line.len() {
                    break;
                }
                let kind = if row_offset == 0 && col_offset == 0 {
                    ImageCellKind::Anchor(id)
                } else {
                    ImageCellKind::Continuation(id)
                };
                line.cells[col].image_cell = kind;
            }
        }

        self.image_placements.push(ImagePlacement {
            image,
            anchor_line,
            anchor_col,
        });

        const MAX_IMAGE_PLACEMENTS: usize = 200;
        while self.image_placements.len() > MAX_IMAGE_PLACEMENTS {
            self.image_placements.remove(0);
        }

        for _ in 0..display_rows {
            if self.cursor.row == self.scroll_region_bottom {
                self.scroll_up_region();
            } else if self.cursor.row + 1 < self.rows {
                self.cursor.row += 1;
            }
        }
        self.cursor.col = 0;
    }

    pub fn visible_image_placements(&self) -> Vec<&ImagePlacement> {
        let total = self.lines.len();
        let viewport_start = total.saturating_sub(self.rows + self.scroll_offset);
        let viewport_end = total.saturating_sub(self.scroll_offset);

        self.image_placements
            .iter()
            .filter(|p| {
                let img_end = p.anchor_line + p.image.display_rows;
                p.anchor_line < viewport_end && img_end > viewport_start
            })
            .collect()
    }

    pub fn viewport_start(&self) -> usize {
        let total = self.lines.len();
        total.saturating_sub(self.rows + self.scroll_offset)
    }
}
