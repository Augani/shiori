# Terminal Improvements Design

Date: 2026-02-19

## Motivation

Gap analysis comparing Shiori's terminal against Ghostty's libghostty-vt revealed 3 critical, 7 high, and 16+ medium gaps. This design addresses all critical and high severity items to bring the terminal to feature parity with modern expectations.

## Current Architecture

| File | Lines | Purpose |
|------|-------|---------|
| terminal_view.rs | 1,835 | GPUI rendering, input handling, polling |
| terminal_state.rs | 1,387 | Cell grid, cursor, modes, scroll regions |
| ansi_parser.rs | 1,684 | ANSI/VT100/Kitty/iTerm2 parser |
| pty_service.rs | 389 | PTY lifecycle, shell setup, I/O |

Data flow: `pty_service` (I/O) -> `ansi_parser` (protocol) -> `terminal_state` (logic) -> `terminal_view` (rendering).

## Changes

### Phase 1: Bug Fixes & Quick Wins

#### VPA Bug (CSI d resets column)
- `ansi_parser.rs`: `CSI d` currently emits `CursorPosition(row, 0)`. Add new variant `VerticalPositionAbsolute(row)` that preserves current column.
- `terminal_state.rs`: Add handler that sets `cursor.row` without touching `cursor.col`.

#### Device Attributes Response Fix
- DA1: Change `CSI ? 62;4 c` to `CSI ? 62;22 c` (remove false Sixel claim, keep color support).
- DA2: Keep `CSI > 1;1;0 c`.

#### Pixel Dimensions in PTY Resize
- `terminal_view.rs`: Calculate `pixel_width = cols * cell_width`, `pixel_height = rows * cell_height` and pass to `PtySize` instead of 0.

#### OSC String Termination (ESC \)
- `ansi_parser.rs`: When in OscString/DcsEntry/ApcString state and ESC received, transition to intermediate state. If next byte is `\` (0x5C), complete the string. Otherwise, process ESC normally.

#### Forward/Backward Tab (CSI I / CSI Z)
- `ansi_parser.rs`: Parse `CSI I` -> `CursorForwardTab(n)`, `CSI Z` -> `CursorBackwardTab(n)`.
- `terminal_state.rs`: `cursor_forward_tab(n)` moves cursor to nth next tab stop. `cursor_backward_tab(n)` moves to nth previous tab stop.

#### DECRQM Mode Query (CSI ? Ps $ p)
- `ansi_parser.rs`: Parse `$` intermediate + `p` final in private mode -> `RequestMode(mode_number)`.
- `terminal_view.rs`: Respond with `CSI ? {mode};{1|2|0} $ y` (1=set, 2=reset, 0=not recognized).
- `terminal_state.rs`: Add `is_mode_set(mode: u16) -> Option<bool>` method.

#### XTVERSION (CSI > 0 q)
- `ansi_parser.rs`: Parse `CSI > 0 q` -> `RequestVersion`.
- `terminal_view.rs`: Respond with `DCS >|Shiori {version} ST`.

### Phase 2: xterm Modifier Key Encoding

#### Problem
Arrow keys, Home, End, F-keys, Insert, Delete, PageUp, PageDown never include modifier information. Ctrl+Right sends the same as Right.

#### Solution
In `terminal_view.rs` `handle_key_down()`, when Shift, Ctrl, Alt, or combinations are held with special keys, encode modifiers per xterm spec:

Modifier value = 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0).

| Key | Unmodified | Modified |
|-----|-----------|----------|
| Arrow Up | `CSI A` | `CSI 1;{mod} A` |
| Arrow Down | `CSI B` | `CSI 1;{mod} B` |
| Arrow Right | `CSI C` | `CSI 1;{mod} C` |
| Arrow Left | `CSI D` | `CSI 1;{mod} D` |
| Home | `CSI H` | `CSI 1;{mod} H` |
| End | `CSI F` | `CSI 1;{mod} F` |
| F1-F4 | `ESC O P/Q/R/S` | `CSI 1;{mod} P/Q/R/S` |
| F5 | `CSI 15~` | `CSI 15;{mod} ~` |
| F6-F12 | `CSI {n}~` | `CSI {n};{mod} ~` |
| Insert | `CSI 2~` | `CSI 2;{mod} ~` |
| Delete | `CSI 3~` | `CSI 3;{mod} ~` |
| PageUp | `CSI 5~` | `CSI 5;{mod} ~` |
| PageDown | `CSI 6~` | `CSI 6;{mod} ~` |

Note: When application cursor mode (DECCKM) is active AND no modifiers are pressed, arrow keys use `ESC O A/B/C/D`. With modifiers, always use CSI format.

### Phase 3: OSC Color Query Responses

#### Problem
Programs like `bat`, `delta`, vim send OSC 10/11/4 queries and hang waiting for a response.

#### Solution
Parse and respond to color queries:

| OSC | Query | Response |
|-----|-------|----------|
| `10;?` | Foreground color | `OSC 10;rgb:{rr}/{gg}/{bb} ST` |
| `11;?` | Background color | `OSC 11;rgb:{rr}/{gg}/{bb} ST` |
| `12;?` | Cursor color | `OSC 12;rgb:{rr}/{gg}/{bb} ST` |
| `4;{idx};?` | Palette color N | `OSC 4;{idx};rgb:{rr}/{gg}/{bb} ST` |

Also handle set operations:
- `OSC 4;{idx};{color}` -> update palette entry
- `OSC 10/11/12;{color}` -> update fg/bg/cursor
- `OSC 104` -> reset palette to defaults
- `OSC 110/111/112` -> reset fg/bg/cursor to defaults

Color values use X11 `rgb:` notation with 2-digit hex per channel.

New `ParsedSegment` variants: `QueryForegroundColor`, `QueryBackgroundColor`, `QueryCursorColor`, `QueryPaletteColor(u8)`, `SetForegroundColor(r,g,b)`, `SetBackgroundColor(r,g,b)`, `SetPaletteColor(u8,r,g,b)`, `ResetPalette`, `ResetForegroundColor`, `ResetBackgroundColor`, `ResetCursorColor`.

### Phase 4: CSI t Window Operations

#### Problem
Programs querying terminal pixel size or managing titles get no response.

#### Solution
Handle CSI t subcommands:

| Param | Action | Response |
|-------|--------|----------|
| 14 | Report pixel size | `CSI 4;{height_px};{width_px} t` |
| 16 | Report cell size | `CSI 6;{cell_h};{cell_w} t` |
| 18 | Report char size | `CSI 8;{rows};{cols} t` |
| 22;0 | Push title | Push current title onto stack |
| 22;2 | Push title (icon) | Same as 22;0 |
| 23;0 | Pop title | Pop and restore title |
| 23;2 | Pop title (icon) | Same as 23;0 |

Add title stack (`Vec<String>`, capped at 10) to `TerminalState`.

### Phase 5: DCS Passthrough

#### Problem
All DCS content is discarded. Breaks XTGETTCAP, DECRQSS, tmux passthrough.

#### Solution
Replace the current discard-everything DCS handler with a proper state machine:

**XTGETTCAP** (`DCS + q {hex} ST`):
- Decode hex-encoded capability name
- Look up from hardcoded table of common terminfo caps:
  - `TN` -> `xterm-256color`
  - `Co`/`colors` -> `256`
  - `RGB` -> `8/8/8`
  - `setrgbf` -> `\033[38;2;%p1%d;%p2%d;%p3%dm`
  - `setrgbb` -> `\033[48;2;%p1%d;%p2%d;%p3%dm`
  - `Tc` -> true
  - `Su` -> true (colored underlines)
- Respond: `DCS 1 + r {hex}={hex-value} ST` or `DCS 0 + r ST` if unknown

**DECRQSS** (`DCS $ q {type} ST`):
- `m` -> respond with current SGR attributes as `DCS 1 $ r {sgr-params} m ST`
- `r` -> respond with current scroll margins as `DCS 1 $ r {top};{bottom} r ST`
- ` q` -> respond with current cursor style as `DCS 1 $ r {style} SP q ST`

**Other DCS** (tmux control mode, Sixel): Log and ignore for now.

New state in `ansi_parser.rs`: `DcsCollect` state that buffers DCS content until ST. Dispatch based on DCS intermediates (`+q`, `$q`).

### Phase 6: Grapheme-Aware Selection

#### Problem
Wide chars (CJK), emoji with skin tone modifiers, and ZWJ sequences produce broken text when selected.

#### Solution

**Skip continuation cells in copy:**
In `get_selected_text()`, skip cells where `width == 0`. The primary cell (width 1 or 2) already contains the full character.

**Snap selection to grapheme boundaries:**
When mouse-down lands on a continuation cell (width 0), walk left to find the primary cell and use that as the selection anchor.

**Word selection (double-click):**
`word_bounds_at()` treats wide char + continuation as a single unit. Walk by logical characters, skipping continuation cells.

**Selection rendering:**
Highlight both the primary cell and its continuation cell(s) as a unit. When selection start/end falls on a continuation cell, extend to include the primary cell.

### Phase 7: Kitty Keyboard Protocol

#### Problem
neovim, helix, fish 4.0, and modern TUI frameworks need `CSI u` for disambiguated key input.

#### Solution

**State:** Add `keyboard_mode_stack: Vec<u32>` (max 8 entries) to `TerminalState`. Each entry is a flags bitfield:
- Bit 0: Disambiguate escape codes
- Bit 1: Report event types (press/repeat/release)
- Bit 2: Report alternate keys
- Bit 3: Report all keys as escape codes
- Bit 4: Report associated text

**Parser changes:**
- `CSI ? u` -> `QueryKeyboardMode` (respond with current flags)
- `CSI > {flags} u` -> `PushKeyboardMode(flags)` (push onto stack)
- `CSI < {n} u` -> `PopKeyboardMode(n)` (pop n entries)
- `CSI = {flags};{mode} u` -> `SetKeyboardMode(flags, mode)` (1=set, 2=or, 3=not)
- Disambiguate `CSI u` (no intermediates, has params) as SCORC vs Kitty based on presence of `>`, `<`, `?`, `=` intermediates.

**Key encoding when active:**
Format: `CSI {keycode}[;{modifiers}[:event_type]][;{text}] u`

Key mapping table (subset):
- Enter -> 13, Tab -> 9, Backspace -> 127, Escape -> 27
- Arrow Up/Down/Right/Left -> same CSI codes but with `;{mod}` always present
- Letters: Unicode codepoint of the key
- Modifiers: same formula as xterm (1 + shift + 2*alt + 4*ctrl + 8*super)

When `disambiguate` flag is set:
- Ctrl+I sends `CSI 105;5 u` (not `\t`)
- Ctrl+M sends `CSI 109;5 u` (not `\r`)
- Ctrl+[ sends `CSI 91;5 u` (not `ESC`)

### Phase 8: Resize Reflow

#### Problem
Resizing the terminal truncates content. Wrapped lines don't unwrap when widening; long lines don't re-wrap when narrowing.

#### Solution

New method `reflow_lines(old_cols, new_cols)` in `terminal_state.rs`:

**Narrowing (new_cols < old_cols):**
1. Iterate lines from top to bottom
2. If line has more content than `new_cols`:
   a. Split at `new_cols` boundary
   b. If split falls on a wide char continuation cell, split before the wide char (insert spacer at end of first part)
   c. Mark first part as `wrapped = true`
   d. Insert remainder as new line
3. Track cursor logical position through splits

**Widening (new_cols > old_cols):**
1. Iterate lines from top to bottom
2. If line is `wrapped == true` and next line exists:
   a. Pull cells from next line into current line up to `new_cols`
   b. If next line becomes empty, remove it
   c. If next line was also wrapped and more space remains, continue pulling
   d. Update `wrapped` flag based on whether next line content remains
3. Track cursor logical position through merges

**Constraints:**
- Alt-screen does NOT reflow (truncate only, matches xterm/Ghostty)
- Reflow operates on both viewport + scrollback lines
- Cursor row/col adjusted to maintain logical position
- Performance: O(total_lines) which is bounded by max_scrollback (5000)

## Files Modified

| File | Changes |
|------|---------|
| `ansi_parser.rs` | New ParsedSegment variants, DCS state machine, OSC color parsing, CSI t/u/I/Z parsing, VPA fix, OSC ST termination fix |
| `terminal_state.rs` | Keyboard mode stack, title stack, reflow logic, tab navigation, mode query, VPA handler |
| `terminal_view.rs` | Modifier key encoding, Kitty key encoding, color query responses, DCS responses, CSI t responses, pixel dims in resize, grapheme-aware selection |
| `pty_service.rs` | Pixel dimensions in PtySize |

## Implementation Order

| Phase | Description | Depends on |
|-------|-------------|------------|
| 1 | Bug fixes & quick wins | None |
| 2 | xterm modifier key encoding | None |
| 3 | OSC color query responses | None |
| 4 | CSI t window operations | None |
| 5 | DCS passthrough | None |
| 6 | Grapheme-aware selection | None |
| 7 | Kitty keyboard protocol | Phase 2 (builds on modifier encoding) |
| 8 | Resize reflow | None |

Phases 1-6 are independent. Phase 7 depends on Phase 2. Phase 8 is independent but large.
