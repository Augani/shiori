# Terminal Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close all critical and high severity gaps between Shiori's terminal and modern terminal standards (using Ghostty as reference).

**Architecture:** 8 phases modifying 4 files: `ansi_parser.rs` (protocol parsing), `terminal_state.rs` (state machine), `terminal_view.rs` (rendering + input encoding), `pty_service.rs` (PTY lifecycle). Data flows pty_service → ansi_parser → terminal_state → terminal_view.

**Tech Stack:** Rust nightly, GPUI, portable-pty, unicode-width

**No test suite exists.** Verification is via `cargo +nightly clippy`, `cargo +nightly build --release`, and manual testing in the built terminal.

---

## Task 1: VPA Bug Fix (CSI d resets column)

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add ParsedSegment variant)
- Modify: `src/ansi_parser.rs:961-964` (fix VPA dispatch)
- Modify: `src/terminal_state.rs` (add handler)
- Modify: `src/terminal_view.rs:249-290` (add apply_segment arm)

**Step 1: Add `VerticalPositionAbsolute` variant to `ParsedSegment`**

In `src/ansi_parser.rs`, after `CursorPrevLine(usize)` (line 158), add:

```rust
VerticalPositionAbsolute(usize),
```

**Step 2: Fix VPA dispatch in execute_csi**

In `src/ansi_parser.rs`, replace lines 961-964:

```rust
// Before (broken):
b'd' => {
    let row = param_or(&self.params, 0, 1);
    segments.push(ParsedSegment::CursorPosition(row.saturating_sub(1), 0));
}

// After (fixed):
b'd' => {
    let row = param_or(&self.params, 0, 1);
    segments.push(ParsedSegment::VerticalPositionAbsolute(row.saturating_sub(1)));
}
```

**Step 3: Add `vertical_position_absolute` to TerminalState**

In `src/terminal_state.rs`, add method after `cursor_prev_line` (around line 815):

```rust
pub fn vertical_position_absolute(&mut self, row: usize) {
    self.cursor.row = row.min(self.rows.saturating_sub(1));
    self.mark_dirty(self.cursor.row);
}
```

**Step 4: Wire up in apply_segment**

In `src/terminal_view.rs`, after the `CursorPrevLine` arm (line 262), add:

```rust
ParsedSegment::VerticalPositionAbsolute(row) => {
    self.state.vertical_position_absolute(row)
}
```

**Step 5: Build and verify**

Run: `cargo +nightly clippy`
Expected: No errors or warnings related to VPA changes.

**Step 6: Commit**

```bash
git add src/ansi_parser.rs src/terminal_state.rs src/terminal_view.rs
git commit -m "fix: VPA (CSI d) preserves column instead of resetting to 0"
```

---

## Task 2: Device Attributes Response Fix

**Files:**
- Modify: `src/terminal_view.rs:346-351` (fix DA1 response)

**Step 1: Fix DA1 response**

In `src/terminal_view.rs`, replace lines 346-351:

```rust
// Before:
ParsedSegment::DeviceAttributes(level) => {
    if level == 0 {
        self.send_input(b"\x1b[?62;4c");
    } else {
        self.send_input(b"\x1b[>1;1;0c");
    }
}

// After:
ParsedSegment::DeviceAttributes(level) => {
    if level == 0 {
        self.send_input(b"\x1b[?62;22c");
    } else {
        self.send_input(b"\x1b[>1;1;0c");
    }
}
```

This removes the false Sixel claim (4) and replaces with color support flag (22).

**Step 2: Build and verify**

Run: `cargo +nightly clippy`

**Step 3: Commit**

```bash
git add src/terminal_view.rs
git commit -m "fix: DA1 response removes false Sixel claim, reports color support"
```

---

## Task 3: Pixel Dimensions in PTY Resize

**Files:**
- Modify: `src/pty_service.rs:75-81` (start method)
- Modify: `src/pty_service.rs:208-224` (resize method)
- Modify: `src/terminal_view.rs:455-468` (flush_pending_resize)

**Step 1: Add pixel dimensions to PtyService resize signature**

In `src/pty_service.rs`, change `resize` (line 208) to accept pixel dims:

```rust
pub fn resize(&mut self, cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Result<(), PtyError> {
    self.cols = cols;
    self.rows = rows;

    if let Some(pty_pair) = &self.pty_pair {
        pty_pair
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width,
                pixel_height,
            })
            .map_err(|e| PtyError::ResizeFailed(e.to_string()))?;
    }
    Ok(())
}
```

**Step 2: Pass pixel dimensions from flush_pending_resize**

In `src/terminal_view.rs`, update `flush_pending_resize` (line 455):

```rust
fn flush_pending_resize(&mut self) {
    if let Some((cols, rows, when)) = self.pending_pty_resize {
        let elapsed = when.elapsed().as_millis();
        if elapsed >= 500 {
            self.pending_pty_resize = None;
            if cols != self.state.cols() || rows != self.state.rows() {
                self.state.resize(cols, rows);
                if let Some(pty) = &mut self.pty {
                    let pixel_width = (cols as f32 * self.char_width) as u16;
                    let pixel_height = (rows as f32 * self.line_height) as u16;
                    let _ = pty.resize(cols as u16, rows as u16, pixel_width, pixel_height);
                }
            }
        }
    }
}
```

**Step 3: Fix all other pty.resize() call sites**

Search for any other calls to `pty.resize(` and update their signatures. The `start()` method in pty_service.rs (line 75) already uses `PtySize` directly, so only the `resize()` callers need updating.

**Step 4: Build and verify**

Run: `cargo +nightly clippy`

**Step 5: Commit**

```bash
git add src/pty_service.rs src/terminal_view.rs
git commit -m "fix: pass pixel dimensions in PTY resize for accurate TIOCGWINSZ"
```

---

## Task 4: OSC String Termination (ESC \)

**Files:**
- Modify: `src/ansi_parser.rs:134-146` (add parser state)
- Modify: `src/ansi_parser.rs:668-684` (handle_osc)
- Modify: `src/ansi_parser.rs` (add new handler + dispatch)

**Step 1: Add OscEscIntermediate state**

In `src/ansi_parser.rs`, add to `ParserState` enum (line 134):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiPrivate,
    OscString,
    OscEscIntermediate,
    DcsEntry,
    DcsCollect,
    ApcString,
}
```

**Step 2: Update handle_osc for ESC transition**

In `src/ansi_parser.rs`, update `handle_osc` (line 668):

```rust
fn handle_osc(&mut self, byte: u8, segments: &mut Vec<ParsedSegment>) {
    match byte {
        0x07 => {
            self.execute_osc(segments);
            self.state = ParserState::Ground;
        }
        0x1B => {
            self.state = ParserState::OscEscIntermediate;
        }
        _ => {
            if self.osc_string.len() < 4096 {
                self.osc_string.push(byte);
            }
        }
    }
}
```

**Step 3: Add handler for OscEscIntermediate**

Add new method and wire it into the main `parse` dispatch loop. When in `OscEscIntermediate`:
- If byte is `\\` (0x5C): execute OSC, go to Ground
- If byte is something else: execute OSC, process byte as new ESC sequence start

```rust
fn handle_osc_esc(&mut self, byte: u8, segments: &mut Vec<ParsedSegment>) {
    self.execute_osc(segments);
    if byte == b'\\' {
        self.state = ParserState::Ground;
    } else {
        self.state = ParserState::Escape;
        self.handle_escape(byte, segments);
    }
}
```

**Step 4: Wire into main parse loop**

In the main `parse()` method, add the `OscEscIntermediate` match arm alongside the other states:

```rust
ParserState::OscEscIntermediate => {
    self.handle_osc_esc(byte, &mut segments);
}
```

**Step 5: Apply same fix to DCS**

Update `handle_dcs` similarly — when ESC received, transition to a state that checks for `\`:

```rust
fn handle_dcs(&mut self, byte: u8, _segments: &mut Vec<ParsedSegment>) {
    match byte {
        0x1B => {
            self.state = ParserState::Ground;
        }
        _ => {
            if self.dcs_string.len() < 4096 {
                self.dcs_string.push(byte);
            }
        }
    }
}
```

(DCS will be fully reworked in Task 11, so minimal fix here.)

**Step 6: Build and verify**

Run: `cargo +nightly clippy`

**Step 7: Commit**

```bash
git add src/ansi_parser.rs
git commit -m "fix: handle ESC \\ as OSC/DCS string terminator"
```

---

## Task 5: Forward/Backward Tab (CSI I / CSI Z)

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add ParsedSegment variants)
- Modify: `src/ansi_parser.rs:924-1049` (execute_csi)
- Modify: `src/terminal_state.rs` (add methods)
- Modify: `src/terminal_view.rs:249-290` (apply_segment)

**Step 1: Add ParsedSegment variants**

In `src/ansi_parser.rs`, after `Tab` (line 177), add:

```rust
CursorForwardTab(usize),
CursorBackwardTab(usize),
```

**Step 2: Parse CSI I and CSI Z**

In `execute_csi` (line 924), add before the `b't'` case:

```rust
b'I' => {
    segments.push(ParsedSegment::CursorForwardTab(param_or(&self.params, 0, 1)));
}
b'Z' => {
    segments.push(ParsedSegment::CursorBackwardTab(param_or(&self.params, 0, 1)));
}
```

**Step 3: Add cursor_forward_tab and cursor_backward_tab to TerminalState**

In `src/terminal_state.rs`, after the existing `tab()` method (line 825):

```rust
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
        let prev_tab = self.tab_stops[..col]
            .iter()
            .rposition(|&s| s)
            .unwrap_or(0);
        self.cursor.col = prev_tab;
    }
}
```

**Step 4: Wire up in apply_segment**

In `src/terminal_view.rs`, after the `Tab` arm:

```rust
ParsedSegment::CursorForwardTab(n) => self.state.cursor_forward_tab(n),
ParsedSegment::CursorBackwardTab(n) => self.state.cursor_backward_tab(n),
```

**Step 5: Build and verify**

Run: `cargo +nightly clippy`

**Step 6: Commit**

```bash
git add src/ansi_parser.rs src/terminal_state.rs src/terminal_view.rs
git commit -m "feat: support CSI I (forward tab) and CSI Z (backward tab)"
```

---

## Task 6: DECRQM Mode Query (CSI ? Ps $ p)

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add variant)
- Modify: `src/ansi_parser.rs:645-666` (handle_csi_intermediate)
- Modify: `src/terminal_state.rs` (add is_mode_set)
- Modify: `src/terminal_view.rs` (respond)

**Step 1: Add ParsedSegment variant**

```rust
RequestMode(u16),
```

**Step 2: Parse $ p in CSI intermediate handler**

In `src/ansi_parser.rs`, update `handle_csi_intermediate` (line 645). When intermediate is `$` and final byte is `p`, emit `RequestMode`:

```rust
fn handle_csi_intermediate(
    &mut self,
    byte: u8,
    _text_buffer: &mut String,
    segments: &mut Vec<ParsedSegment>,
) {
    match byte {
        b' '..=b'/' => {
            self.intermediate.push(byte);
        }
        b'@'..=b'~' => {
            if !self.intermediate.is_empty() && self.intermediate[0] == b' ' && byte == b'q' {
                let style = self.params.first().copied().unwrap_or(0) as u8;
                segments.push(ParsedSegment::CursorStyle(style));
            } else if !self.intermediate.is_empty()
                && self.intermediate[0] == b'$'
                && byte == b'p'
            {
                let mode = self.params.first().copied().unwrap_or(0);
                segments.push(ParsedSegment::RequestMode(mode));
            }
            self.state = ParserState::Ground;
        }
        _ => {
            self.state = ParserState::Ground;
        }
    }
}
```

Note: This handles the non-private case. For private mode (`CSI ? Ps $ p`), the private marker is set and the intermediate handler needs to check for it. We need to also handle this in `execute_private_mode` or route it from the private CSI path. Since the private marker (`?`) routes to `execute_private_mode`, we need to handle the `$p` intermediate there too.

Actually, let's handle it differently. The `$` intermediate is collected during CsiIntermediate state. When the private marker is set AND intermediate is `$` AND final is `p`, we need to recognize DECRQM.

In `execute_private_mode`, add a check at the top for `$p`:

```rust
fn execute_private_mode(&mut self, final_byte: u8, segments: &mut Vec<ParsedSegment>) {
    if final_byte == b'p' && !self.intermediate.is_empty() && self.intermediate[0] == b'$' {
        let mode = self.params.first().copied().unwrap_or(0);
        segments.push(ParsedSegment::RequestMode(mode));
        return;
    }
    // ... rest of existing code
}
```

**Step 3: Add is_mode_set to TerminalState**

In `src/terminal_state.rs`:

```rust
pub fn is_mode_set(&self, mode: u16) -> Option<bool> {
    match mode {
        1 => Some(self.application_cursor_keys),
        6 => Some(self.origin_mode),
        7 => Some(self.autowrap),
        25 => Some(self.cursor_visible),
        47 | 1047 => Some(self.use_alt_screen),
        1000 | 1002 | 1003 => Some(self.mouse_mode == mode),
        1004 => Some(self.focus_tracking),
        1006 => Some(self.sgr_mouse),
        1049 => Some(self.use_alt_screen),
        2004 => Some(self.bracketed_paste),
        2026 => Some(self.sync_update_active),
        _ => None,
    }
}
```

**Step 4: Respond in apply_segment**

In `src/terminal_view.rs`:

```rust
ParsedSegment::RequestMode(mode) => {
    let setting = match self.state.is_mode_set(mode) {
        Some(true) => 1,
        Some(false) => 2,
        None => 0,
    };
    let response = format!("\x1b[?{};{}$y", mode, setting);
    self.send_input(response.as_bytes());
}
```

**Step 5: Build and verify**

Run: `cargo +nightly clippy`

**Step 6: Commit**

```bash
git add src/ansi_parser.rs src/terminal_state.rs src/terminal_view.rs
git commit -m "feat: support DECRQM mode query (CSI ? Ps $ p)"
```

---

## Task 7: XTVERSION (CSI > 0 q)

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add variant)
- Modify: `src/ansi_parser.rs:1051-1108` (execute_private_mode)
- Modify: `src/terminal_view.rs` (respond)

**Step 1: Add ParsedSegment variant**

```rust
RequestVersion,
```

**Step 2: Parse CSI > 0 q in execute_private_mode**

In `src/ansi_parser.rs`, in `execute_private_mode`, add handling for `>` marker with `q` final:

```rust
fn execute_private_mode(&mut self, final_byte: u8, segments: &mut Vec<ParsedSegment>) {
    // DECRQM check (from Task 6)
    if final_byte == b'p' && !self.intermediate.is_empty() && self.intermediate[0] == b'$' {
        let mode = self.params.first().copied().unwrap_or(0);
        segments.push(ParsedSegment::RequestMode(mode));
        return;
    }

    // XTVERSION: CSI > 0 q or CSI > q
    if final_byte == b'q' && self.private_marker == Some(b'>') {
        segments.push(ParsedSegment::RequestVersion);
        return;
    }

    // DA2: CSI > c
    if final_byte == b'c' {
        if self.private_marker == Some(b'>') {
            segments.push(ParsedSegment::DeviceAttributes(1));
        }
        return;
    }

    // ... rest of mode set/reset handling
}
```

**Step 3: Respond in apply_segment**

In `src/terminal_view.rs`:

```rust
ParsedSegment::RequestVersion => {
    let version = env!("CARGO_PKG_VERSION");
    let response = format!("\x1bP>|Shiori {}\x1b\\", version);
    self.send_input(response.as_bytes());
}
```

**Step 4: Build and verify**

Run: `cargo +nightly clippy`

**Step 5: Commit**

```bash
git add src/ansi_parser.rs src/terminal_view.rs
git commit -m "feat: respond to XTVERSION query with Shiori version"
```

---

## Task 8: xterm Modifier Key Encoding

**Files:**
- Modify: `src/terminal_view.rs:482-634` (handle_key_down)

**Step 1: Add modifier encoding helper**

In `src/terminal_view.rs`, add a helper method to `TerminalView`:

```rust
fn modifier_value(modifiers: &gpui::Modifiers) -> u8 {
    let mut val: u8 = 1;
    if modifiers.shift {
        val += 1;
    }
    if modifiers.alt {
        val += 2;
    }
    if modifiers.control {
        val += 4;
    }
    val
}

fn has_modifiers(modifiers: &gpui::Modifiers) -> bool {
    modifiers.shift || modifiers.alt || modifiers.control
}
```

**Step 2: Replace special key handling with modifier-aware encoding**

Replace the arrow key, home, end, page, insert, F-key, and delete handling in `handle_key_down` (lines 500-633) to use modifier encoding.

For arrow keys (example for "up"):

```rust
"up" => {
    let mods = &event.keystroke.modifiers;
    if Self::has_modifiers(mods) {
        let m = Self::modifier_value(mods);
        let seq = format!("\x1b[1;{}A", m);
        self.send_input(seq.as_bytes());
    } else if app_cursor {
        self.send_input(b"\x1bOA");
    } else {
        self.send_input(key_codes::UP);
    }
    true
}
```

For tilde-style keys (delete, insert, pageup, pagedown, F5-F12):

```rust
"delete" => {
    let mods = &event.keystroke.modifiers;
    if Self::has_modifiers(mods) {
        let m = Self::modifier_value(mods);
        let seq = format!("\x1b[3;{}~", m);
        self.send_input(seq.as_bytes());
    } else {
        self.send_input(key_codes::DELETE);
    }
    true
}
```

For F1-F4 (SS3 style unmodified, CSI style with modifiers):

```rust
"f1" => {
    let mods = &event.keystroke.modifiers;
    if Self::has_modifiers(mods) {
        let m = Self::modifier_value(mods);
        let seq = format!("\x1b[1;{}P", m);
        self.send_input(seq.as_bytes());
    } else {
        self.send_input(b"\x1bOP");
    }
    true
}
```

Apply the same pattern to all special keys:
- `up/down/left/right`: CSI 1;{mod} A/B/C/D
- `home/end`: CSI 1;{mod} H/F
- `insert`: CSI 2;{mod} ~
- `delete`: CSI 3;{mod} ~
- `pageup`: CSI 5;{mod} ~
- `pagedown`: CSI 6;{mod} ~
- `f1-f4`: CSI 1;{mod} P/Q/R/S
- `f5`: CSI 15;{mod} ~
- `f6`: CSI 17;{mod} ~
- `f7`: CSI 18;{mod} ~
- `f8`: CSI 19;{mod} ~
- `f9`: CSI 20;{mod} ~
- `f10`: CSI 21;{mod} ~
- `f11`: CSI 23;{mod} ~
- `f12`: CSI 24;{mod} ~

**Step 3: Build and verify**

Run: `cargo +nightly clippy`

**Step 4: Commit**

```bash
git add src/terminal_view.rs
git commit -m "feat: xterm modifier key encoding for arrows, F-keys, and special keys"
```

---

## Task 9: OSC Color Query Responses

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add variants)
- Modify: `src/ansi_parser.rs:859-921` (execute_osc)
- Modify: `src/terminal_view.rs` (respond)

**Step 1: Add ParsedSegment variants**

In `src/ansi_parser.rs`, add to the enum:

```rust
QueryForegroundColor,
QueryBackgroundColor,
QueryCursorColor,
QueryPaletteColor(u8),
SetForegroundColor(u8, u8, u8),
SetBackgroundColor(u8, u8, u8),
SetCursorColor(u8, u8, u8),
SetPaletteColor(u8, u8, u8, u8),
ResetPalette,
ResetForegroundColor,
ResetBackgroundColor,
ResetCursorColor,
```

**Step 2: Parse OSC 10/11/12/4 queries and set commands**

In `src/ansi_parser.rs`, update `execute_osc` to handle these. Add cases in the match on `cmd`:

```rust
"10" => {
    if arg == "?" {
        segments.push(ParsedSegment::QueryForegroundColor);
    } else if let Some((r, g, b)) = parse_x11_color(arg) {
        segments.push(ParsedSegment::SetForegroundColor(r, g, b));
    }
}
"11" => {
    if arg == "?" {
        segments.push(ParsedSegment::QueryBackgroundColor);
    } else if let Some((r, g, b)) = parse_x11_color(arg) {
        segments.push(ParsedSegment::SetBackgroundColor(r, g, b));
    }
}
"12" => {
    if arg == "?" {
        segments.push(ParsedSegment::QueryCursorColor);
    } else if let Some((r, g, b)) = parse_x11_color(arg) {
        segments.push(ParsedSegment::SetCursorColor(r, g, b));
    }
}
"4" => {
    if let Some(semi) = arg.find(';') {
        let idx_str = &arg[..semi];
        let color_str = &arg[semi + 1..];
        if let Ok(idx) = idx_str.parse::<u8>() {
            if color_str == "?" {
                segments.push(ParsedSegment::QueryPaletteColor(idx));
            } else if let Some((r, g, b)) = parse_x11_color(color_str) {
                segments.push(ParsedSegment::SetPaletteColor(idx, r, g, b));
            }
        }
    }
}
"104" => {
    segments.push(ParsedSegment::ResetPalette);
}
"110" => {
    segments.push(ParsedSegment::ResetForegroundColor);
}
"111" => {
    segments.push(ParsedSegment::ResetBackgroundColor);
}
"112" => {
    segments.push(ParsedSegment::ResetCursorColor);
}
```

Also handle cases where OSC 10/11/12/104/110/111/112 come without a semicolon (no arg). Add before the `if let Some(idx) = osc.find(';')` block:

```rust
fn execute_osc(&mut self, segments: &mut Vec<ParsedSegment>) {
    let osc = String::from_utf8_lossy(&self.osc_string).into_owned();

    // Handle no-arg OSC commands
    match osc.as_str() {
        "104" => {
            segments.push(ParsedSegment::ResetPalette);
            self.osc_string.clear();
            return;
        }
        "110" => {
            segments.push(ParsedSegment::ResetForegroundColor);
            self.osc_string.clear();
            return;
        }
        "111" => {
            segments.push(ParsedSegment::ResetBackgroundColor);
            self.osc_string.clear();
            return;
        }
        "112" => {
            segments.push(ParsedSegment::ResetCursorColor);
            self.osc_string.clear();
            return;
        }
        _ => {}
    }

    if let Some(idx) = osc.find(';') {
        // ... existing code with new cases added
    }
    self.osc_string.clear();
}
```

**Step 3: Add parse_x11_color helper**

In `src/ansi_parser.rs`:

```rust
fn parse_x11_color(s: &str) -> Option<(u8, u8, u8)> {
    if let Some(hex) = s.strip_prefix("rgb:") {
        let parts: Vec<&str> = hex.split('/').collect();
        if parts.len() == 3 {
            let r = u8::from_str_radix(&parts[0][..2.min(parts[0].len())], 16).ok()?;
            let g = u8::from_str_radix(&parts[1][..2.min(parts[1].len())], 16).ok()?;
            let b = u8::from_str_radix(&parts[2][..2.min(parts[2].len())], 16).ok()?;
            return Some((r, g, b));
        }
    }
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some((r, g, b));
        }
    }
    None
}
```

**Step 4: Respond to queries in terminal_view**

In `src/terminal_view.rs`, add handling in `apply_segment`. Use the parser's stored palette/fg/bg colors:

```rust
ParsedSegment::QueryForegroundColor => {
    let fg = self.parser.foreground_color();
    let response = format!(
        "\x1b]10;rgb:{:02x}/{:02x}/{:02x}\x1b\\",
        (fg.r * 255.0) as u8,
        (fg.g * 255.0) as u8,
        (fg.b * 255.0) as u8,
    );
    self.send_input(response.as_bytes());
}
ParsedSegment::QueryBackgroundColor => {
    let bg = self.parser.background_color();
    let response = format!(
        "\x1b]11;rgb:{:02x}/{:02x}/{:02x}\x1b\\",
        (bg.r * 255.0) as u8,
        (bg.g * 255.0) as u8,
        (bg.b * 255.0) as u8,
    );
    self.send_input(response.as_bytes());
}
ParsedSegment::QueryCursorColor => {
    let fg = self.parser.foreground_color();
    let response = format!(
        "\x1b]12;rgb:{:02x}/{:02x}/{:02x}\x1b\\",
        (fg.r * 255.0) as u8,
        (fg.g * 255.0) as u8,
        (fg.b * 255.0) as u8,
    );
    self.send_input(response.as_bytes());
}
ParsedSegment::QueryPaletteColor(idx) => {
    let color = self.parser.palette_color(idx);
    let response = format!(
        "\x1b]4;{};rgb:{:02x}/{:02x}/{:02x}\x1b\\",
        idx,
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
    );
    self.send_input(response.as_bytes());
}
```

For the set/reset variants, handle them as no-ops or update the parser's color state. The important part is the query responses — set/reset can store the values for when queries happen again:

```rust
ParsedSegment::SetForegroundColor(_r, _g, _b) => {}
ParsedSegment::SetBackgroundColor(_r, _g, _b) => {}
ParsedSegment::SetCursorColor(_r, _g, _b) => {}
ParsedSegment::SetPaletteColor(_idx, _r, _g, _b) => {}
ParsedSegment::ResetPalette => {}
ParsedSegment::ResetForegroundColor => {}
ParsedSegment::ResetBackgroundColor => {}
ParsedSegment::ResetCursorColor => {}
```

**Step 5: Add accessor methods to AnsiParser**

In `src/ansi_parser.rs`, add public methods to get the current foreground, background, and palette colors. These are already stored in the parser's `palette`, `fg_color`, `bg_color` fields.

```rust
pub fn foreground_color(&self) -> Rgba {
    self.fg_color
}

pub fn background_color(&self) -> Rgba {
    self.bg_color
}

pub fn palette_color(&self, idx: u8) -> Rgba {
    self.palette.get(idx as usize).copied().unwrap_or(self.fg_color)
}
```

**Step 6: Build and verify**

Run: `cargo +nightly clippy`

**Step 7: Commit**

```bash
git add src/ansi_parser.rs src/terminal_view.rs
git commit -m "feat: OSC 10/11/12/4 color query responses and set/reset"
```

---

## Task 10: CSI t Window Operations

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add variants)
- Modify: `src/ansi_parser.rs:1045` (parse CSI t)
- Modify: `src/terminal_state.rs:341-389` (add title_stack)
- Modify: `src/terminal_view.rs` (respond + apply)

**Step 1: Add ParsedSegment variants**

```rust
ReportPixelSize,
ReportCellSize,
ReportCharSize,
PushTitle,
PopTitle,
```

**Step 2: Parse CSI t subcommands**

In `src/ansi_parser.rs`, replace the empty `b't' => {}` (line 1045):

```rust
b't' => {
    let param = param_or(&self.params, 0, 0);
    match param {
        14 => segments.push(ParsedSegment::ReportPixelSize),
        16 => segments.push(ParsedSegment::ReportCellSize),
        18 => segments.push(ParsedSegment::ReportCharSize),
        22 => segments.push(ParsedSegment::PushTitle),
        23 => segments.push(ParsedSegment::PopTitle),
        _ => {}
    }
}
```

**Step 3: Add title stack to TerminalState**

In `src/terminal_state.rs`, add field to `TerminalState` struct:

```rust
title_stack: Vec<String>,
```

Initialize in `new()`:

```rust
title_stack: Vec::new(),
```

Add methods:

```rust
pub fn push_title(&mut self) {
    if self.title_stack.len() < 10 {
        self.title_stack.push(self.title.clone().unwrap_or_default());
    }
}

pub fn pop_title(&mut self) -> Option<String> {
    self.title_stack.pop()
}
```

**Step 4: Handle in apply_segment**

In `src/terminal_view.rs`:

```rust
ParsedSegment::ReportPixelSize => {
    let height = (self.state.rows() as f32 * self.line_height) as usize;
    let width = (self.state.cols() as f32 * self.char_width) as usize;
    let response = format!("\x1b[4;{};{}t", height, width);
    self.send_input(response.as_bytes());
}
ParsedSegment::ReportCellSize => {
    let cell_h = self.line_height as usize;
    let cell_w = self.char_width as usize;
    let response = format!("\x1b[6;{};{}t", cell_h, cell_w);
    self.send_input(response.as_bytes());
}
ParsedSegment::ReportCharSize => {
    let response = format!("\x1b[8;{};{}t", self.state.rows(), self.state.cols());
    self.send_input(response.as_bytes());
}
ParsedSegment::PushTitle => {
    self.state.push_title();
}
ParsedSegment::PopTitle => {
    if let Some(title) = self.state.pop_title() {
        self.state.set_title(Some(title));
    }
}
```

**Step 5: Build and verify**

Run: `cargo +nightly clippy`

**Step 6: Commit**

```bash
git add src/ansi_parser.rs src/terminal_state.rs src/terminal_view.rs
git commit -m "feat: CSI t window operations (pixel/cell/char size, title push/pop)"
```

---

## Task 11: DCS Passthrough (XTGETTCAP + DECRQSS)

**Files:**
- Modify: `src/ansi_parser.rs:134-146` (add DcsCollect state)
- Modify: `src/ansi_parser.rs:148-209` (add variants)
- Modify: `src/ansi_parser.rs:686-690` (rewrite handle_dcs)
- Modify: `src/terminal_view.rs` (respond)
- Modify: `src/terminal_state.rs` (add sgr_string method)

**Step 1: Add DcsCollect state and buffer field**

The `DcsCollect` state was added in Task 4's `ParserState` enum. Add a `dcs_string: Vec<u8>` buffer to the AnsiParser struct (alongside the existing `osc_string` and `apc_string`), and `dcs_intermediate: Vec<u8>` field.

**Step 2: Add ParsedSegment variants**

```rust
XtGetTcap(String),
DecrqssRequest(String),
```

**Step 3: Rewrite DCS handling**

In `src/ansi_parser.rs`, when the parser enters DCS (on receiving `P` after ESC):
- Collect DCS intermediates during `DcsEntry` (the current state handles first byte)
- Transition to `DcsCollect` to buffer content
- On `ESC \` or `0x07` (ST), dispatch

Replace `handle_dcs`:

```rust
fn handle_dcs_entry(&mut self, byte: u8) {
    match byte {
        0x1B => {
            self.state = ParserState::Ground;
        }
        b'0'..=b'9' | b';' => {
            self.dcs_string.push(byte);
        }
        _ => {
            self.dcs_string.push(byte);
            self.state = ParserState::DcsCollect;
        }
    }
}

fn handle_dcs_collect(&mut self, byte: u8, segments: &mut Vec<ParsedSegment>) {
    match byte {
        0x1B => {
            self.execute_dcs(segments);
            self.state = ParserState::Ground;
        }
        0x07 => {
            self.execute_dcs(segments);
            self.state = ParserState::Ground;
        }
        _ => {
            if self.dcs_string.len() < 4096 {
                self.dcs_string.push(byte);
            }
        }
    }
}

fn execute_dcs(&mut self, segments: &mut Vec<ParsedSegment>) {
    let dcs = std::mem::take(&mut self.dcs_string);
    let dcs_str = String::from_utf8_lossy(&dcs);

    if dcs_str.starts_with("+q") {
        let hex_names = &dcs_str[2..];
        for hex_name in hex_names.split(';') {
            if let Some(name) = hex_decode_string(hex_name.trim()) {
                segments.push(ParsedSegment::XtGetTcap(name));
            }
        }
    } else if dcs_str.starts_with("$q") {
        let request_type = dcs_str[2..].to_string();
        segments.push(ParsedSegment::DecrqssRequest(request_type));
    }
}
```

**Step 4: Add hex decode helper**

```rust
fn hex_decode_string(hex: &str) -> Option<String> {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .filter_map(|i| {
            if i + 2 <= hex.len() {
                u8::from_str_radix(&hex[i..i + 2], 16).ok()
            } else {
                None
            }
        })
        .collect();
    String::from_utf8(bytes).ok()
}

fn hex_encode_string(s: &str) -> String {
    s.bytes().map(|b| format!("{:02X}", b)).collect()
}
```

**Step 5: Handle XTGETTCAP responses in terminal_view**

In `src/terminal_view.rs`:

```rust
ParsedSegment::XtGetTcap(name) => {
    let (found, value) = match name.as_str() {
        "TN" => (true, "xterm-256color"),
        "Co" | "colors" => (true, "256"),
        "RGB" => (true, "8/8/8"),
        "Tc" => (true, "true"),
        "Su" => (true, "true"),
        "setrgbf" => (true, "\\E[38;2;%p1%d;%p2%d;%p3%dm"),
        "setrgbb" => (true, "\\E[48;2;%p1%d;%p2%d;%p3%dm"),
        _ => (false, ""),
    };
    if found {
        let hex_name = crate::ansi_parser::hex_encode_string(&name);
        let hex_val = crate::ansi_parser::hex_encode_string(value);
        let response = format!("\x1bP1+r{}={}\x1b\\", hex_name, hex_val);
        self.send_input(response.as_bytes());
    } else {
        let hex_name = crate::ansi_parser::hex_encode_string(&name);
        let response = format!("\x1bP0+r{}\x1b\\", hex_name);
        self.send_input(response.as_bytes());
    }
}
```

**Step 6: Handle DECRQSS responses**

```rust
ParsedSegment::DecrqssRequest(request_type) => {
    let response = match request_type.as_str() {
        "m" => {
            let sgr = self.state.current_sgr_string();
            format!("\x1bP1$r{}m\x1b\\", sgr)
        }
        "r" => {
            let (top, bottom) = self.state.scroll_region();
            format!("\x1bP1$r{};{}r\x1b\\", top + 1, bottom + 1)
        }
        " q" => {
            let style = self.state.cursor_style_code();
            format!("\x1bP1$r{} q\x1b\\", style)
        }
        _ => format!("\x1bP0$r\x1b\\"),
    };
    self.send_input(response.as_bytes());
}
```

**Step 7: Add helper methods to TerminalState**

In `src/terminal_state.rs`:

```rust
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
```

**Step 8: Update DCS state transitions in parse loop**

In the main `parse()` method, update the state dispatch:
- `DcsEntry` → calls `handle_dcs_entry`
- `DcsCollect` → calls `handle_dcs_collect`

**Step 9: Make hex_encode_string public**

Mark `hex_encode_string` as `pub` so terminal_view can use it.

**Step 10: Build and verify**

Run: `cargo +nightly clippy`

**Step 11: Commit**

```bash
git add src/ansi_parser.rs src/terminal_state.rs src/terminal_view.rs
git commit -m "feat: DCS passthrough for XTGETTCAP and DECRQSS"
```

---

## Task 12: Grapheme-Aware Selection

**Files:**
- Modify: `src/terminal_view.rs:1005-1051` (get_selected_text)
- Modify: `src/terminal_view.rs:841-882` (word_bounds_at)
- Modify: `src/terminal_view.rs` (selection rendering + mouse snap)

**Step 1: Fix get_selected_text to skip continuation cells**

In `src/terminal_view.rs`, update `get_selected_text` (line 1005):

```rust
pub fn get_selected_text(&self) -> Option<String> {
    let (start, end) = (self.selection_start?, self.selection_end?);

    let (start_line, start_col, end_line, end_col) =
        if start.0 > end.0 || (start.0 == end.0 && start.1 > end.1) {
            (end.0, end.1, start.0, start.1)
        } else {
            (start.0, start.1, end.0, end.1)
        };

    if start_line == end_line && start_col == end_col {
        return None;
    }

    let mut result = String::new();

    for line_idx in start_line..=end_line {
        if let Some(line) = self.state.line(line_idx) {
            let col_start = if line_idx == start_line { start_col } else { 0 };
            let col_end = if line_idx == end_line {
                end_col.min(line.cells.len())
            } else {
                line.cells.len()
            };

            for col in col_start..col_end {
                if let Some(cell) = line.cells.get(col) {
                    if cell.width == 0 {
                        continue;
                    }
                    if cell.char != ' ' || col < col_end.saturating_sub(1) {
                        result.push(cell.char);
                    }
                }
            }

            let trimmed_len = result.trim_end().len();
            result.truncate(trimmed_len);

            if line_idx < end_line && !line.wrapped {
                result.push('\n');
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
```

**Step 2: Fix word_bounds_at to handle wide chars**

Update `word_bounds_at` (line 841) to skip continuation cells:

```rust
fn word_bounds_at(&self, line_idx: usize, col: usize) -> (usize, usize) {
    let line = match self.state.line(line_idx) {
        Some(l) => l,
        None => return (col, col),
    };

    if col >= line.cells.len() {
        return (col, col);
    }

    let actual_col = if line.cells[col].width == 0 {
        (0..col).rev().find(|&c| line.cells[c].width != 0).unwrap_or(col)
    } else {
        col
    };

    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    let target_char = line.cells[actual_col].char;
    let target_is_word = is_word_char(target_char);

    let mut start = actual_col;
    while start > 0 {
        let prev = start - 1;
        if line.cells[prev].width == 0 {
            start = prev;
            continue;
        }
        let prev_char = line.cells[prev].char;
        if target_is_word {
            if !is_word_char(prev_char) {
                break;
            }
        } else if prev_char.is_whitespace() != target_char.is_whitespace() {
            break;
        }
        start = prev;
    }

    let mut end = actual_col;
    while end < line.cells.len() {
        let cell = &line.cells[end];
        if cell.width == 0 {
            end += 1;
            continue;
        }
        if target_is_word {
            if !is_word_char(cell.char) {
                break;
            }
        } else if cell.char.is_whitespace() != target_char.is_whitespace() {
            break;
        }
        end += 1;
    }

    (start, end)
}
```

**Step 3: Snap mouse selection to grapheme boundaries**

When setting `selection_start` or `selection_end` in mouse handlers, check if the column lands on a continuation cell (width == 0) and walk left to the primary cell. Add helper:

```rust
fn snap_to_primary_cell(&self, line_idx: usize, col: usize) -> usize {
    if let Some(line) = self.state.line(line_idx) {
        if col < line.cells.len() && line.cells[col].width == 0 {
            return (0..col)
                .rev()
                .find(|&c| line.cells[c].width != 0)
                .unwrap_or(col);
        }
    }
    col
}
```

Apply this in mouse_down and mouse_move where selection coordinates are set.

**Step 4: Build and verify**

Run: `cargo +nightly clippy`

**Step 5: Commit**

```bash
git add src/terminal_view.rs
git commit -m "feat: grapheme-aware selection (skip continuations, snap to primary)"
```

---

## Task 13: Kitty Keyboard Protocol

**Depends on:** Task 8 (modifier encoding)

**Files:**
- Modify: `src/ansi_parser.rs:148-209` (add variants)
- Modify: `src/ansi_parser.rs:1012-1013` (disambiguate CSI u)
- Modify: `src/terminal_state.rs:341-389` (add keyboard_mode_stack)
- Modify: `src/terminal_view.rs:482-707` (key encoding)

**Step 1: Add ParsedSegment variants**

```rust
QueryKeyboardMode,
PushKeyboardMode(u32),
PopKeyboardMode(u32),
SetKeyboardMode(u32, u8),
```

**Step 2: Parse Kitty keyboard CSI sequences**

In `src/ansi_parser.rs`, the current `b'u'` case (line 1012) unconditionally emits `CursorRestore`. We need to disambiguate based on intermediates/private markers:

- No intermediate, no private → `CursorRestore` (SCORC)
- `?` private marker → `QueryKeyboardMode`
- `>` private marker → `PushKeyboardMode`
- `<` private marker → `PopKeyboardMode`
- `=` private marker → `SetKeyboardMode`

The `>` and `<` markers are currently only checked in `execute_private_mode`. We need to route `u` with these markers.

Update `execute_private_mode` to handle the `u` final byte:

```rust
// In execute_private_mode, add before the mode set/reset handling:
if final_byte == b'u' {
    match self.private_marker {
        Some(b'?') => {
            segments.push(ParsedSegment::QueryKeyboardMode);
            return;
        }
        Some(b'>') => {
            let flags = self.params.first().copied().unwrap_or(0) as u32;
            segments.push(ParsedSegment::PushKeyboardMode(flags));
            return;
        }
        Some(b'<') => {
            let n = self.params.first().copied().unwrap_or(1) as u32;
            segments.push(ParsedSegment::PopKeyboardMode(n));
            return;
        }
        Some(b'=') => {
            let flags = self.params.first().copied().unwrap_or(0) as u32;
            let mode = self.params.get(1).copied().unwrap_or(1) as u8;
            segments.push(ParsedSegment::SetKeyboardMode(flags, mode));
            return;
        }
        _ => return,
    }
}
```

Keep `b'u'` in `execute_csi` as `CursorRestore` for the non-private case.

**Step 3: Add keyboard mode stack to TerminalState**

In `src/terminal_state.rs`, add field:

```rust
keyboard_mode_stack: Vec<u32>,
```

Initialize in `new()`:

```rust
keyboard_mode_stack: Vec::new(),
```

Add methods:

```rust
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
```

**Step 4: Handle in apply_segment**

```rust
ParsedSegment::QueryKeyboardMode => {
    let flags = self.state.keyboard_mode_flags();
    let response = format!("\x1b[?{}u", flags);
    self.send_input(response.as_bytes());
}
ParsedSegment::PushKeyboardMode(flags) => {
    self.state.push_keyboard_mode(flags);
}
ParsedSegment::PopKeyboardMode(n) => {
    self.state.pop_keyboard_mode(n);
}
ParsedSegment::SetKeyboardMode(flags, mode) => {
    self.state.set_keyboard_mode(flags, mode);
}
```

**Step 5: Encode keys using Kitty protocol when active**

In `src/terminal_view.rs`, update `handle_key_down` to check `self.state.keyboard_mode_flags()`. When bit 0 (disambiguate) is set, encode ambiguous keys:

Add a method `encode_key_kitty`:

```rust
fn encode_key_kitty(&mut self, key: &str, event: &KeyDownEvent) -> bool {
    let flags = self.state.keyboard_mode_flags();
    if flags == 0 {
        return false;
    }

    let disambiguate = flags & 1 != 0;
    if !disambiguate {
        return false;
    }

    let mods = &event.keystroke.modifiers;
    let mod_val = Self::modifier_value(mods);
    let has_mods = Self::has_modifiers(mods);

    let codepoint = match key {
        "enter" => Some(13u32),
        "tab" => Some(9),
        "backspace" => Some(127),
        "escape" => Some(27),
        _ => None,
    };

    if let Some(cp) = codepoint {
        if has_mods || disambiguate {
            let seq = if has_mods {
                format!("\x1b[{};{}u", cp, mod_val)
            } else {
                format!("\x1b[{}u", cp)
            };
            self.send_input(seq.as_bytes());
            return true;
        }
    }

    if let Some(key_char) = &event.keystroke.key_char {
        if let Some(c) = key_char.chars().next() {
            if c.is_ascii_alphabetic() && mods.control {
                let cp = c.to_ascii_lowercase() as u32;
                let seq = format!("\x1b[{};{}u", cp, mod_val);
                self.send_input(seq.as_bytes());
                return true;
            }
        }
    }

    false
}
```

Call this at the top of `handle_key_down`, before the existing match:

```rust
if self.encode_key_kitty(key, event) {
    self.reset_cursor_blink();
    return;
}
```

**Step 6: Build and verify**

Run: `cargo +nightly clippy`

**Step 7: Commit**

```bash
git add src/ansi_parser.rs src/terminal_state.rs src/terminal_view.rs
git commit -m "feat: Kitty keyboard protocol (CSI u) with mode stack"
```

---

## Task 14: Resize Reflow

**Files:**
- Modify: `src/terminal_state.rs:1213-1269` (resize method)

**Step 1: Add reflow_lines method**

In `src/terminal_state.rs`, add before `resize`:

```rust
fn reflow_lines(lines: &mut VecDeque<TerminalLine>, old_cols: usize, new_cols: usize, cursor_row: usize, cursor_col: usize) -> (usize, usize) {
    if old_cols == new_cols {
        return (cursor_row, cursor_col);
    }

    let abs_cursor = cursor_row;
    let mut new_cursor_row = cursor_row;
    let mut new_cursor_col = cursor_col;

    if new_cols < old_cols {
        let mut i = 0;
        let mut cursor_offset = 0;
        while i < lines.len() {
            let line_len = lines[i].cells.iter().rposition(|c| c.char != ' ').map(|p| p + 1).unwrap_or(0);
            if line_len > new_cols {
                let mut remainder_cells: Vec<TerminalCell> = lines[i].cells.split_off(new_cols);
                let was_wrapped = lines[i].wrapped;
                lines[i].wrapped = true;

                let mut new_line = TerminalLine {
                    cells: remainder_cells,
                    wrapped: was_wrapped,
                };
                new_line.resize(old_cols);

                if i < abs_cursor + cursor_offset {
                    cursor_offset += 1;
                } else if i == abs_cursor + cursor_offset && new_cursor_col >= new_cols {
                    new_cursor_col -= new_cols;
                    cursor_offset += 1;
                }

                lines.insert(i + 1, new_line);
            }
            lines[i].resize(new_cols);
            i += 1;
        }
        new_cursor_row = abs_cursor + cursor_offset;
    } else {
        let mut i = 0;
        let mut cursor_offset: isize = 0;
        while i < lines.len() {
            if lines[i].wrapped && i + 1 < lines.len() {
                let current_content = lines[i].cells.iter().rposition(|c| c.char != ' ').map(|p| p + 1).unwrap_or(0);
                let space = new_cols.saturating_sub(current_content);

                if space > 0 {
                    let next_line = lines.remove(i + 1).unwrap_or_else(|| TerminalLine::new(new_cols));
                    let pull_count = space.min(next_line.cells.len());
                    let pulled: Vec<TerminalCell> = next_line.cells[..pull_count].to_vec();

                    lines[i].cells.truncate(current_content);
                    lines[i].cells.extend(pulled);
                    lines[i].resize(new_cols);

                    let remaining_content = next_line.cells[pull_count..].iter().rposition(|c| c.char != ' ').map(|p| p + 1).unwrap_or(0);
                    if remaining_content > 0 {
                        let mut remaining_line = TerminalLine {
                            cells: next_line.cells[pull_count..].to_vec(),
                            wrapped: next_line.wrapped,
                        };
                        remaining_line.resize(new_cols);
                        lines.insert(i + 1, remaining_line);
                        lines[i].wrapped = true;
                    } else {
                        lines[i].wrapped = next_line.wrapped;
                        if (i + 1) <= (abs_cursor as isize + cursor_offset) as usize {
                            cursor_offset -= 1;
                        }
                    }
                    continue;
                }
            }
            lines[i].resize(new_cols);
            i += 1;
        }
        new_cursor_row = (abs_cursor as isize + cursor_offset).max(0) as usize;
    }

    new_cursor_row = new_cursor_row.min(lines.len().saturating_sub(1));
    new_cursor_col = new_cursor_col.min(new_cols.saturating_sub(1));
    (new_cursor_row, new_cursor_col)
}
```

**Step 2: Update resize to call reflow_lines**

Replace the `resize` method (line 1213):

```rust
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

        self.cursor.row = new_cursor_abs.saturating_sub(viewport_start).min(rows.saturating_sub(1));
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
```

**Step 3: Add TerminalLine resize method check**

Verify the existing `TerminalLine::resize` method works correctly:

```rust
pub fn resize(&mut self, cols: usize) {
    self.cells.resize(cols, TerminalCell::default());
}
```

This truncates or extends — good enough for the reflow.

**Step 4: Build and verify**

Run: `cargo +nightly clippy`

**Step 5: Commit**

```bash
git add src/terminal_state.rs
git commit -m "feat: resize reflow - rewrap lines on terminal width change"
```

---

## Task 15: Final Build Verification

**Step 1: Full clippy check**

Run: `cargo +nightly clippy`
Expected: No errors. Warnings acceptable only for unused variables in set/reset color handlers.

**Step 2: Release build**

Run: `cargo +nightly build --release`
Expected: Successful build.

**Step 3: Commit any remaining fixups**

If clippy or build revealed issues, fix and commit them.

---

## Implementation Order

| Task | Description | Depends on |
|------|-------------|------------|
| 1 | VPA bug fix | None |
| 2 | DA response fix | None |
| 3 | Pixel dims in PTY resize | None |
| 4 | OSC string termination | None |
| 5 | Forward/backward tab | None |
| 6 | DECRQM mode query | None |
| 7 | XTVERSION | None |
| 8 | xterm modifier key encoding | None |
| 9 | OSC color queries | None |
| 10 | CSI t window ops | None |
| 11 | DCS passthrough | None |
| 12 | Grapheme-aware selection | None |
| 13 | Kitty keyboard protocol | Task 8 |
| 14 | Resize reflow | None |
| 15 | Final verification | All |

Tasks 1-12 are independent and can be parallelized. Task 13 depends on Task 8. Task 14 is independent. Task 15 is the final gate.
