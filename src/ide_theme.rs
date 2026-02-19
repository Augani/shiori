use gpui::{Hsla, Rgba};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug)]
pub struct IdeTheme {
    pub name: &'static str,
    pub description: &'static str,
    pub editor: EditorColors,
    pub syntax: SyntaxColors,
    pub terminal: TerminalColors,
    pub chrome: ChromeColors,
}

#[derive(Clone, Debug)]
pub struct EditorColors {
    pub cursor: Hsla,
    pub selection: Hsla,
    pub line_number: Hsla,
    pub line_number_active: Hsla,
    pub gutter_bg: Hsla,
    pub search_match: Hsla,
    pub search_match_active: Hsla,
    pub current_line: Hsla,
    pub bracket_match: Hsla,
    pub word_highlight: Hsla,
    pub indent_guide: Hsla,
    pub indent_guide_active: Hsla,
    pub fold_marker: Hsla,
    pub diagnostic_error: Hsla,
    pub diagnostic_warning: Hsla,
    pub diagnostic_info: Hsla,
    pub diagnostic_hint: Hsla,
}

#[derive(Clone, Debug)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub keyword_control: Hsla,
    pub type_name: Hsla,
    pub type_builtin: Hsla,
    pub function: Hsla,
    pub function_definition: Hsla,
    pub function_method: Hsla,
    pub function_macro: Hsla,
    pub string: Hsla,
    pub string_escape: Hsla,
    pub string_regex: Hsla,
    pub number: Hsla,
    pub comment: Hsla,
    pub comment_doc: Hsla,
    pub operator: Hsla,
    pub variable: Hsla,
    pub variable_parameter: Hsla,
    pub variable_builtin: Hsla,
    pub constant: Hsla,
    pub boolean: Hsla,
    pub property: Hsla,
    pub punctuation: Hsla,
    pub punctuation_bracket: Hsla,
    pub punctuation_delimiter: Hsla,
    pub attribute: Hsla,
    pub namespace: Hsla,
    pub tag: Hsla,
    pub heading: Hsla,
    pub emphasis: Hsla,
    pub link: Hsla,
    pub literal: Hsla,
    pub embedded: Hsla,
    pub constructor: Hsla,
    pub label: Hsla,
    pub default_fg: Hsla,
}

#[derive(Clone, Debug)]
pub struct TerminalColors {
    pub palette: [Rgba; 16],
    pub fg: Rgba,
    pub bg: Rgba,
}

#[derive(Clone, Debug)]
pub struct ChromeColors {
    pub bg: Hsla,
    pub header_border: Hsla,
    pub accent: Hsla,
    pub dim: Hsla,
    pub bright: Hsla,
    pub panel_bg: Hsla,
    pub editor_bg: Hsla,
    pub text_secondary: Hsla,
    pub diff_add_bg: Hsla,
    pub diff_add_text: Hsla,
    pub diff_del_bg: Hsla,
    pub diff_del_text: Hsla,
    pub review_comment_bg: Hsla,
    pub review_comment_indicator: Hsla,
}

static IDE_THEME: LazyLock<Mutex<IdeTheme>> = LazyLock::new(|| Mutex::new(island_dark()));

pub fn use_ide_theme() -> IdeTheme {
    IDE_THEME.lock().unwrap().clone()
}

pub fn install_ide_theme(theme: IdeTheme) {
    *IDE_THEME.lock().unwrap() = theme;
}

pub fn sync_adabraka_theme_from_ide(cx: &mut gpui::App) {
    let ide = use_ide_theme();
    let chrome = &ide.chrome;

    let mut theme = adabraka_ui::theme::Theme::dark();
    theme.tokens.background = chrome.editor_bg;
    theme.tokens.foreground = chrome.bright;
    theme.tokens.card = chrome.panel_bg;
    theme.tokens.card_foreground = chrome.bright;
    theme.tokens.popover = chrome.panel_bg;
    theme.tokens.popover_foreground = chrome.bright;
    theme.tokens.muted = chrome.dim;
    theme.tokens.muted_foreground = chrome.text_secondary;
    theme.tokens.accent = hsla(0.0, 0.0, 1.0, 0.1);
    theme.tokens.accent_foreground = chrome.bright;
    theme.tokens.primary = chrome.accent;
    theme.tokens.primary_foreground = chrome.bg;
    theme.tokens.ring = chrome.accent;
    theme.tokens.secondary = chrome.dim;
    theme.tokens.secondary_foreground = chrome.bright;
    theme.tokens.destructive = chrome.diff_del_text;
    theme.tokens.destructive_foreground = chrome.bright;
    theme.tokens.border = chrome.header_border;
    theme.tokens.input = chrome.header_border;

    adabraka_ui::theme::install_theme(cx, theme);
}

pub fn all_ide_themes() -> Vec<IdeTheme> {
    vec![
        island_dark(),
        dracula(),
        nord(),
        monokai_vivid(),
        github_dark(),
        github_light(),
        cyberpunk(),
    ]
}

fn rgba_from_hex(hex: u32) -> Rgba {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    Rgba { r, g, b, a: 1.0 }
}

fn hsla_from_hex(hex: u32) -> Hsla {
    let rgba = rgba_from_hex(hex);
    rgba_to_hsla(rgba)
}

fn rgba_to_hsla(rgba: Rgba) -> Hsla {
    let r = rgba.r;
    let g = rgba.g;
    let b = rgba.b;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 0.001 {
        return Hsla {
            h: 0.0,
            s: 0.0,
            l,
            a: rgba.a,
        };
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < 0.001 {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h
    } else if (max - g).abs() < 0.001 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    Hsla {
        h: h / 6.0,
        s,
        l,
        a: rgba.a,
    }
}

fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h, s, l, a }
}

pub fn island_dark() -> IdeTheme {
    IdeTheme {
        name: "Island Dark",
        description: "A calm, modern dark theme",
        editor: EditorColors {
            cursor: hsla_from_hex(0x3b82f6),
            selection: hsla(0.611, 0.40, 0.35, 0.40),
            line_number: hsla_from_hex(0x4a4a4f),
            line_number_active: hsla_from_hex(0x9ca3af),
            gutter_bg: hsla_from_hex(0x18181b),
            search_match: hsla(0.114, 0.67, 0.50, 0.30),
            search_match_active: hsla(0.081, 0.54, 0.55, 0.50),
            current_line: hsla(0.0, 0.0, 1.0, 0.04),
            bracket_match: hsla(0.575, 0.82, 0.66, 0.50),
            word_highlight: hsla(0.0, 0.0, 1.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 1.0, 0.06),
            indent_guide_active: hsla(0.0, 0.0, 1.0, 0.14),
            fold_marker: hsla_from_hex(0x6b7280),
            diagnostic_error: hsla_from_hex(0xf87171),
            diagnostic_warning: hsla_from_hex(0xfbbf24),
            diagnostic_info: hsla_from_hex(0x60a5fa),
            diagnostic_hint: hsla_from_hex(0x6b7280),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0xc084fc),
            keyword_control: hsla_from_hex(0xd8b4fe),
            type_name: hsla_from_hex(0x67e8f9),
            type_builtin: hsla_from_hex(0x22d3ee),
            function: hsla_from_hex(0x60a5fa),
            function_definition: hsla_from_hex(0x93bbfc),
            function_method: hsla_from_hex(0x60a5fa),
            function_macro: hsla_from_hex(0xa78bfa),
            string: hsla_from_hex(0x86efac),
            string_escape: hsla_from_hex(0xfca5a5),
            string_regex: hsla_from_hex(0xf87171),
            number: hsla_from_hex(0xfbbf24),
            comment: hsla_from_hex(0x6b7280),
            comment_doc: hsla_from_hex(0x8b95a5),
            operator: hsla_from_hex(0x94a3b8),
            variable: hsla_from_hex(0xbdc6d4),
            variable_parameter: hsla_from_hex(0xfda4af),
            variable_builtin: hsla_from_hex(0xf87171),
            constant: hsla_from_hex(0xfbbf24),
            boolean: hsla_from_hex(0xfb923c),
            property: hsla_from_hex(0x93c5fd),
            punctuation: hsla_from_hex(0x9ca3af),
            punctuation_bracket: hsla_from_hex(0x7f8694),
            punctuation_delimiter: hsla_from_hex(0x9ca3af),
            attribute: hsla_from_hex(0xfbbf24),
            namespace: hsla_from_hex(0x67e8f9),
            tag: hsla_from_hex(0xf87171),
            heading: hsla_from_hex(0x60a5fa),
            emphasis: hsla_from_hex(0xc084fc),
            link: hsla_from_hex(0x3b82f6),
            literal: hsla_from_hex(0x86efac),
            embedded: hsla_from_hex(0x9ca3af),
            constructor: hsla_from_hex(0x67e8f9),
            label: hsla_from_hex(0xfbbf24),
            default_fg: hsla_from_hex(0xe2e8f0),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x121214),
                rgba_from_hex(0xf87171),
                rgba_from_hex(0x86efac),
                rgba_from_hex(0xfbbf24),
                rgba_from_hex(0x60a5fa),
                rgba_from_hex(0xc084fc),
                rgba_from_hex(0x67e8f9),
                rgba_from_hex(0xe2e8f0),
                rgba_from_hex(0x4a4a4f),
                rgba_from_hex(0xf87171),
                rgba_from_hex(0x86efac),
                rgba_from_hex(0xfbbf24),
                rgba_from_hex(0x60a5fa),
                rgba_from_hex(0xc084fc),
                rgba_from_hex(0x67e8f9),
                rgba_from_hex(0xffffff),
            ],
            fg: rgba_from_hex(0xe2e8f0),
            bg: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0x121214),
            header_border: hsla(0.0, 0.0, 1.0, 0.05),
            accent: hsla_from_hex(0x3b82f6),
            dim: hsla_from_hex(0x6b7280),
            bright: hsla_from_hex(0xe2e8f0),
            panel_bg: hsla_from_hex(0x1e1e20),
            editor_bg: hsla_from_hex(0x18181b),
            text_secondary: hsla_from_hex(0x9ca3af),
            diff_add_bg: hsla(0.33, 0.7, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0x86efac),
            diff_del_bg: hsla(0.0, 0.7, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xf87171),
            review_comment_bg: hsla_from_hex(0x2a2520),
            review_comment_indicator: hsla_from_hex(0xfbbf24),
        },
    }
}

pub fn dracula() -> IdeTheme {
    IdeTheme {
        name: "Dracula",
        description: "Classic vampire-inspired palette",
        editor: EditorColors {
            cursor: hsla_from_hex(0xf8f8f2),
            selection: hsla(0.73, 0.50, 0.50, 0.35),
            line_number: hsla_from_hex(0x6272a4),
            line_number_active: hsla_from_hex(0xf8f8f2),
            gutter_bg: hsla_from_hex(0x282a36),
            search_match: hsla(0.114, 0.67, 0.50, 0.30),
            search_match_active: hsla(0.081, 0.54, 0.55, 0.50),
            current_line: hsla(0.0, 0.0, 1.0, 0.05),
            bracket_match: hsla_from_hex(0xbd93f9).opacity(0.5),
            word_highlight: hsla(0.0, 0.0, 1.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 1.0, 0.06),
            indent_guide_active: hsla(0.0, 0.0, 1.0, 0.14),
            fold_marker: hsla_from_hex(0x6272a4),
            diagnostic_error: hsla_from_hex(0xff5555),
            diagnostic_warning: hsla_from_hex(0xf1fa8c),
            diagnostic_info: hsla_from_hex(0x8be9fd),
            diagnostic_hint: hsla_from_hex(0x6272a4),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0xff79c6),
            keyword_control: hsla_from_hex(0xff79c6),
            type_name: hsla_from_hex(0x8be9fd),
            type_builtin: hsla_from_hex(0x8be9fd),
            function: hsla_from_hex(0x50fa7b),
            function_definition: hsla_from_hex(0x50fa7b),
            function_method: hsla_from_hex(0x50fa7b),
            function_macro: hsla_from_hex(0x50fa7b),
            string: hsla_from_hex(0xf1fa8c),
            string_escape: hsla_from_hex(0xff79c6),
            string_regex: hsla_from_hex(0xff5555),
            number: hsla_from_hex(0xbd93f9),
            comment: hsla_from_hex(0x6272a4),
            comment_doc: hsla_from_hex(0x7384b0),
            operator: hsla_from_hex(0xff79c6),
            variable: hsla_from_hex(0xf8f8f2),
            variable_parameter: hsla_from_hex(0xffb86c),
            variable_builtin: hsla_from_hex(0xbd93f9),
            constant: hsla_from_hex(0xbd93f9),
            boolean: hsla_from_hex(0xbd93f9),
            property: hsla_from_hex(0x66d9ef),
            punctuation: hsla_from_hex(0xf8f8f2),
            punctuation_bracket: hsla_from_hex(0xf8f8f2),
            punctuation_delimiter: hsla_from_hex(0xf8f8f2),
            attribute: hsla_from_hex(0x50fa7b),
            namespace: hsla_from_hex(0x8be9fd),
            tag: hsla_from_hex(0xff79c6),
            heading: hsla_from_hex(0xbd93f9),
            emphasis: hsla_from_hex(0xff79c6),
            link: hsla_from_hex(0x8be9fd),
            literal: hsla_from_hex(0xf1fa8c),
            embedded: hsla_from_hex(0xf8f8f2),
            constructor: hsla_from_hex(0x8be9fd),
            label: hsla_from_hex(0xff79c6),
            default_fg: hsla_from_hex(0xf8f8f2),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x282a36),
                rgba_from_hex(0xff5555),
                rgba_from_hex(0x50fa7b),
                rgba_from_hex(0xf1fa8c),
                rgba_from_hex(0xbd93f9),
                rgba_from_hex(0xff79c6),
                rgba_from_hex(0x8be9fd),
                rgba_from_hex(0xf8f8f2),
                rgba_from_hex(0x6272a4),
                rgba_from_hex(0xff6e6e),
                rgba_from_hex(0x69ff94),
                rgba_from_hex(0xffffa5),
                rgba_from_hex(0xd6acff),
                rgba_from_hex(0xff92df),
                rgba_from_hex(0xa4ffff),
                rgba_from_hex(0xffffff),
            ],
            fg: rgba_from_hex(0xf8f8f2),
            bg: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0x282a36),
            header_border: hsla(0.0, 0.0, 1.0, 0.05),
            accent: hsla_from_hex(0xbd93f9),
            dim: hsla_from_hex(0x6272a4),
            bright: hsla_from_hex(0xf8f8f2),
            panel_bg: hsla_from_hex(0x44475a),
            editor_bg: hsla_from_hex(0x282a36),
            text_secondary: hsla_from_hex(0x6272a4),
            diff_add_bg: hsla(0.33, 0.7, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0x50fa7b),
            diff_del_bg: hsla(0.0, 0.7, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xff5555),
            review_comment_bg: hsla_from_hex(0x3a3545),
            review_comment_indicator: hsla_from_hex(0xf1fa8c),
        },
    }
}

pub fn nord() -> IdeTheme {
    IdeTheme {
        name: "Nord",
        description: "Arctic, north-bluish palette",
        editor: EditorColors {
            cursor: hsla_from_hex(0xd8dee9),
            selection: hsla(0.55, 0.30, 0.40, 0.35),
            line_number: hsla_from_hex(0x4c566a),
            line_number_active: hsla_from_hex(0xd8dee9),
            gutter_bg: hsla_from_hex(0x2e3440),
            search_match: hsla(0.114, 0.67, 0.50, 0.30),
            search_match_active: hsla(0.081, 0.54, 0.55, 0.50),
            current_line: hsla(0.0, 0.0, 1.0, 0.04),
            bracket_match: hsla_from_hex(0x88c0d0).opacity(0.5),
            word_highlight: hsla(0.0, 0.0, 1.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 1.0, 0.06),
            indent_guide_active: hsla(0.0, 0.0, 1.0, 0.14),
            fold_marker: hsla_from_hex(0x4c566a),
            diagnostic_error: hsla_from_hex(0xbf616a),
            diagnostic_warning: hsla_from_hex(0xebcb8b),
            diagnostic_info: hsla_from_hex(0x81a1c1),
            diagnostic_hint: hsla_from_hex(0x4c566a),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0x81a1c1),
            keyword_control: hsla_from_hex(0x5e81ac),
            type_name: hsla_from_hex(0x8fbcbb),
            type_builtin: hsla_from_hex(0x8fbcbb),
            function: hsla_from_hex(0x88c0d0),
            function_definition: hsla_from_hex(0x88c0d0),
            function_method: hsla_from_hex(0x88c0d0),
            function_macro: hsla_from_hex(0x5e81ac),
            string: hsla_from_hex(0xa3be8c),
            string_escape: hsla_from_hex(0xebcb8b),
            string_regex: hsla_from_hex(0xebcb8b),
            number: hsla_from_hex(0xb48ead),
            comment: hsla_from_hex(0x616e88),
            comment_doc: hsla_from_hex(0x7b8da0),
            operator: hsla_from_hex(0x81a1c1),
            variable: hsla_from_hex(0xd8dee9),
            variable_parameter: hsla_from_hex(0xd08770),
            variable_builtin: hsla_from_hex(0xbf616a),
            constant: hsla_from_hex(0xb48ead),
            boolean: hsla_from_hex(0xd08770),
            property: hsla_from_hex(0x88c0d0),
            punctuation: hsla_from_hex(0xeceff4),
            punctuation_bracket: hsla_from_hex(0xd8dee9),
            punctuation_delimiter: hsla_from_hex(0xeceff4),
            attribute: hsla_from_hex(0x8fbcbb),
            namespace: hsla_from_hex(0x8fbcbb),
            tag: hsla_from_hex(0x81a1c1),
            heading: hsla_from_hex(0x88c0d0),
            emphasis: hsla_from_hex(0x81a1c1),
            link: hsla_from_hex(0x88c0d0),
            literal: hsla_from_hex(0xa3be8c),
            embedded: hsla_from_hex(0xd8dee9),
            constructor: hsla_from_hex(0x8fbcbb),
            label: hsla_from_hex(0xd08770),
            default_fg: hsla_from_hex(0xd8dee9),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x3b4252),
                rgba_from_hex(0xbf616a),
                rgba_from_hex(0xa3be8c),
                rgba_from_hex(0xebcb8b),
                rgba_from_hex(0x81a1c1),
                rgba_from_hex(0xb48ead),
                rgba_from_hex(0x88c0d0),
                rgba_from_hex(0xe5e9f0),
                rgba_from_hex(0x4c566a),
                rgba_from_hex(0xbf616a),
                rgba_from_hex(0xa3be8c),
                rgba_from_hex(0xebcb8b),
                rgba_from_hex(0x81a1c1),
                rgba_from_hex(0xb48ead),
                rgba_from_hex(0x8fbcbb),
                rgba_from_hex(0xeceff4),
            ],
            fg: rgba_from_hex(0xd8dee9),
            bg: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0x2e3440),
            header_border: hsla(0.0, 0.0, 1.0, 0.05),
            accent: hsla_from_hex(0x88c0d0),
            dim: hsla_from_hex(0x4c566a),
            bright: hsla_from_hex(0xd8dee9),
            panel_bg: hsla_from_hex(0x3b4252),
            editor_bg: hsla_from_hex(0x2e3440),
            text_secondary: hsla_from_hex(0x616e88),
            diff_add_bg: hsla(0.33, 0.7, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0xa3be8c),
            diff_del_bg: hsla(0.0, 0.7, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xbf616a),
            review_comment_bg: hsla_from_hex(0x3b3a40),
            review_comment_indicator: hsla_from_hex(0xebcb8b),
        },
    }
}

pub fn monokai_vivid() -> IdeTheme {
    IdeTheme {
        name: "Monokai Vivid",
        description: "Bold and vivid syntax colors",
        editor: EditorColors {
            cursor: hsla_from_hex(0xf8f8f0),
            selection: hsla(0.15, 0.40, 0.30, 0.40),
            line_number: hsla_from_hex(0x90908a),
            line_number_active: hsla_from_hex(0xf8f8f0),
            gutter_bg: hsla_from_hex(0x272822),
            search_match: hsla(0.114, 0.67, 0.50, 0.30),
            search_match_active: hsla(0.081, 0.54, 0.55, 0.50),
            current_line: hsla(0.0, 0.0, 1.0, 0.04),
            bracket_match: hsla_from_hex(0xf92672).opacity(0.4),
            word_highlight: hsla(0.0, 0.0, 1.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 1.0, 0.06),
            indent_guide_active: hsla(0.0, 0.0, 1.0, 0.14),
            fold_marker: hsla_from_hex(0x75715e),
            diagnostic_error: hsla_from_hex(0xf92672),
            diagnostic_warning: hsla_from_hex(0xe6db74),
            diagnostic_info: hsla_from_hex(0x66d9ef),
            diagnostic_hint: hsla_from_hex(0x75715e),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0xf92672),
            keyword_control: hsla_from_hex(0xf92672),
            type_name: hsla_from_hex(0x66d9ef),
            type_builtin: hsla_from_hex(0x66d9ef),
            function: hsla_from_hex(0xa6e22e),
            function_definition: hsla_from_hex(0xa6e22e),
            function_method: hsla_from_hex(0xa6e22e),
            function_macro: hsla_from_hex(0xa6e22e),
            string: hsla_from_hex(0xe6db74),
            string_escape: hsla_from_hex(0xae81ff),
            string_regex: hsla_from_hex(0xf92672),
            number: hsla_from_hex(0xae81ff),
            comment: hsla_from_hex(0x75715e),
            comment_doc: hsla_from_hex(0x8a856e),
            operator: hsla_from_hex(0xf92672),
            variable: hsla_from_hex(0xf8f8f0),
            variable_parameter: hsla_from_hex(0xfd971f),
            variable_builtin: hsla_from_hex(0xfd971f),
            constant: hsla_from_hex(0xae81ff),
            boolean: hsla_from_hex(0xae81ff),
            property: hsla_from_hex(0x66d9ef),
            punctuation: hsla_from_hex(0xf8f8f2),
            punctuation_bracket: hsla_from_hex(0xf8f8f2),
            punctuation_delimiter: hsla_from_hex(0xf8f8f2),
            attribute: hsla_from_hex(0xa6e22e),
            namespace: hsla_from_hex(0x66d9ef),
            tag: hsla_from_hex(0xf92672),
            heading: hsla_from_hex(0xa6e22e),
            emphasis: hsla_from_hex(0xf92672),
            link: hsla_from_hex(0x66d9ef),
            literal: hsla_from_hex(0xe6db74),
            embedded: hsla_from_hex(0xf8f8f2),
            constructor: hsla_from_hex(0x66d9ef),
            label: hsla_from_hex(0xfd971f),
            default_fg: hsla_from_hex(0xf8f8f0),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x272822),
                rgba_from_hex(0xf92672),
                rgba_from_hex(0xa6e22e),
                rgba_from_hex(0xe6db74),
                rgba_from_hex(0x66d9ef),
                rgba_from_hex(0xae81ff),
                rgba_from_hex(0xa1efe4),
                rgba_from_hex(0xf8f8f2),
                rgba_from_hex(0x75715e),
                rgba_from_hex(0xf92672),
                rgba_from_hex(0xa6e22e),
                rgba_from_hex(0xe6db74),
                rgba_from_hex(0x66d9ef),
                rgba_from_hex(0xae81ff),
                rgba_from_hex(0xa1efe4),
                rgba_from_hex(0xf9f8f5),
            ],
            fg: rgba_from_hex(0xf8f8f2),
            bg: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0x272822),
            header_border: hsla(0.0, 0.0, 1.0, 0.05),
            accent: hsla_from_hex(0xf92672),
            dim: hsla_from_hex(0x75715e),
            bright: hsla_from_hex(0xf8f8f0),
            panel_bg: hsla_from_hex(0x3e3d32),
            editor_bg: hsla_from_hex(0x272822),
            text_secondary: hsla_from_hex(0x90908a),
            diff_add_bg: hsla(0.33, 0.7, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0xa6e22e),
            diff_del_bg: hsla(0.0, 0.7, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xf92672),
            review_comment_bg: hsla_from_hex(0x33301e),
            review_comment_indicator: hsla_from_hex(0xe6db74),
        },
    }
}

pub fn github_dark() -> IdeTheme {
    IdeTheme {
        name: "GitHub Dark",
        description: "GitHub's official dark theme",
        editor: EditorColors {
            cursor: hsla_from_hex(0xc9d1d9),
            selection: hsla(0.60, 0.50, 0.40, 0.30),
            line_number: hsla_from_hex(0x484f58),
            line_number_active: hsla_from_hex(0xc9d1d9),
            gutter_bg: hsla_from_hex(0x0d1117),
            search_match: hsla(0.114, 0.67, 0.50, 0.30),
            search_match_active: hsla(0.081, 0.54, 0.55, 0.50),
            current_line: hsla(0.0, 0.0, 1.0, 0.04),
            bracket_match: hsla_from_hex(0x58a6ff).opacity(0.4),
            word_highlight: hsla(0.0, 0.0, 1.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 1.0, 0.06),
            indent_guide_active: hsla(0.0, 0.0, 1.0, 0.14),
            fold_marker: hsla_from_hex(0x484f58),
            diagnostic_error: hsla_from_hex(0xff7b72),
            diagnostic_warning: hsla_from_hex(0xe3b341),
            diagnostic_info: hsla_from_hex(0x58a6ff),
            diagnostic_hint: hsla_from_hex(0x484f58),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0xff7b72),
            keyword_control: hsla_from_hex(0xff7b72),
            type_name: hsla_from_hex(0x79c0ff),
            type_builtin: hsla_from_hex(0x79c0ff),
            function: hsla_from_hex(0xd2a8ff),
            function_definition: hsla_from_hex(0xd2a8ff),
            function_method: hsla_from_hex(0xd2a8ff),
            function_macro: hsla_from_hex(0xd2a8ff),
            string: hsla_from_hex(0xa5d6ff),
            string_escape: hsla_from_hex(0x79c0ff),
            string_regex: hsla_from_hex(0xff7b72),
            number: hsla_from_hex(0x79c0ff),
            comment: hsla_from_hex(0x8b949e),
            comment_doc: hsla_from_hex(0x9ba5ae),
            operator: hsla_from_hex(0xff7b72),
            variable: hsla_from_hex(0xffa657),
            variable_parameter: hsla_from_hex(0xffa657),
            variable_builtin: hsla_from_hex(0xff7b72),
            constant: hsla_from_hex(0x79c0ff),
            boolean: hsla_from_hex(0x79c0ff),
            property: hsla_from_hex(0x7ee787),
            punctuation: hsla_from_hex(0xc9d1d9),
            punctuation_bracket: hsla_from_hex(0xc9d1d9),
            punctuation_delimiter: hsla_from_hex(0xc9d1d9),
            attribute: hsla_from_hex(0x7ee787),
            namespace: hsla_from_hex(0xffa657),
            tag: hsla_from_hex(0x7ee787),
            heading: hsla_from_hex(0x58a6ff),
            emphasis: hsla_from_hex(0xff7b72),
            link: hsla_from_hex(0x58a6ff),
            literal: hsla_from_hex(0xa5d6ff),
            embedded: hsla_from_hex(0xc9d1d9),
            constructor: hsla_from_hex(0x79c0ff),
            label: hsla_from_hex(0x7ee787),
            default_fg: hsla_from_hex(0xc9d1d9),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x0d1117),
                rgba_from_hex(0xff7b72),
                rgba_from_hex(0x7ee787),
                rgba_from_hex(0xe3b341),
                rgba_from_hex(0x58a6ff),
                rgba_from_hex(0xd2a8ff),
                rgba_from_hex(0x79c0ff),
                rgba_from_hex(0xc9d1d9),
                rgba_from_hex(0x484f58),
                rgba_from_hex(0xff7b72),
                rgba_from_hex(0x7ee787),
                rgba_from_hex(0xe3b341),
                rgba_from_hex(0x58a6ff),
                rgba_from_hex(0xd2a8ff),
                rgba_from_hex(0x79c0ff),
                rgba_from_hex(0xf0f6fc),
            ],
            fg: rgba_from_hex(0xc9d1d9),
            bg: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0x0d1117),
            header_border: hsla(0.0, 0.0, 1.0, 0.05),
            accent: hsla_from_hex(0x58a6ff),
            dim: hsla_from_hex(0x484f58),
            bright: hsla_from_hex(0xc9d1d9),
            panel_bg: hsla_from_hex(0x161b22),
            editor_bg: hsla_from_hex(0x0d1117),
            text_secondary: hsla_from_hex(0x8b949e),
            diff_add_bg: hsla(0.33, 0.7, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0x7ee787),
            diff_del_bg: hsla(0.0, 0.7, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xff7b72),
            review_comment_bg: hsla_from_hex(0x1c1e24),
            review_comment_indicator: hsla_from_hex(0xe3b341),
        },
    }
}

pub fn cyberpunk() -> IdeTheme {
    IdeTheme {
        name: "Cyberpunk",
        description: "Neon-lit digital frontier",
        editor: EditorColors {
            cursor: hsla_from_hex(0xfcee0a),
            selection: hsla(0.16, 0.80, 0.40, 0.30),
            line_number: hsla_from_hex(0x1e3a5f),
            line_number_active: hsla_from_hex(0x8badc9),
            gutter_bg: hsla_from_hex(0x000b1e),
            search_match: hsla(0.16, 0.80, 0.50, 0.30),
            search_match_active: hsla(0.16, 0.90, 0.60, 0.50),
            current_line: hsla(0.0, 0.0, 1.0, 0.04),
            bracket_match: hsla_from_hex(0xfcee0a).opacity(0.4),
            word_highlight: hsla(0.0, 0.0, 1.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 1.0, 0.06),
            indent_guide_active: hsla(0.0, 0.0, 1.0, 0.14),
            fold_marker: hsla_from_hex(0x1e3a5f),
            diagnostic_error: hsla_from_hex(0xff2e97),
            diagnostic_warning: hsla_from_hex(0xfcee0a),
            diagnostic_info: hsla_from_hex(0x00d4ff),
            diagnostic_hint: hsla_from_hex(0x1e3a5f),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0xfcee0a),
            keyword_control: hsla_from_hex(0xfcee0a),
            type_name: hsla_from_hex(0x00d4ff),
            type_builtin: hsla_from_hex(0x00d4ff),
            function: hsla_from_hex(0xff2e97),
            function_definition: hsla_from_hex(0xff2e97),
            function_method: hsla_from_hex(0xff2e97),
            function_macro: hsla_from_hex(0xd557ff),
            string: hsla_from_hex(0x00ff9f),
            string_escape: hsla_from_hex(0xfcee0a),
            string_regex: hsla_from_hex(0xff2e97),
            number: hsla_from_hex(0xfcee0a),
            comment: hsla_from_hex(0x1e3a5f),
            comment_doc: hsla_from_hex(0x2d5580),
            operator: hsla_from_hex(0xff2e97),
            variable: hsla_from_hex(0x8badc9),
            variable_parameter: hsla_from_hex(0xd557ff),
            variable_builtin: hsla_from_hex(0xff2e97),
            constant: hsla_from_hex(0xfcee0a),
            boolean: hsla_from_hex(0xff2e97),
            property: hsla_from_hex(0x00d4ff),
            punctuation: hsla_from_hex(0x8badc9),
            punctuation_bracket: hsla_from_hex(0x5e8ba0),
            punctuation_delimiter: hsla_from_hex(0x8badc9),
            attribute: hsla_from_hex(0x00ff9f),
            namespace: hsla_from_hex(0x00d4ff),
            tag: hsla_from_hex(0xff2e97),
            heading: hsla_from_hex(0xfcee0a),
            emphasis: hsla_from_hex(0xff2e97),
            link: hsla_from_hex(0x00d4ff),
            literal: hsla_from_hex(0x00ff9f),
            embedded: hsla_from_hex(0x8badc9),
            constructor: hsla_from_hex(0x00d4ff),
            label: hsla_from_hex(0xfcee0a),
            default_fg: hsla_from_hex(0x8badc9),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x000b1e),
                rgba_from_hex(0xff2e97),
                rgba_from_hex(0x00ff9f),
                rgba_from_hex(0xfcee0a),
                rgba_from_hex(0x00d4ff),
                rgba_from_hex(0xd557ff),
                rgba_from_hex(0x00d4ff),
                rgba_from_hex(0x8badc9),
                rgba_from_hex(0x1e3a5f),
                rgba_from_hex(0xff2e97),
                rgba_from_hex(0x00ff9f),
                rgba_from_hex(0xfcee0a),
                rgba_from_hex(0x00d4ff),
                rgba_from_hex(0xd557ff),
                rgba_from_hex(0x00d4ff),
                rgba_from_hex(0xffffff),
            ],
            fg: rgba_from_hex(0x8badc9),
            bg: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0x000b1e),
            header_border: hsla(0.0, 0.0, 1.0, 0.05),
            accent: hsla_from_hex(0xfcee0a),
            dim: hsla_from_hex(0x1e3a5f),
            bright: hsla_from_hex(0x8badc9),
            panel_bg: hsla_from_hex(0x05162a),
            editor_bg: hsla_from_hex(0x000b1e),
            text_secondary: hsla_from_hex(0x1e3a5f),
            diff_add_bg: hsla(0.33, 0.7, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0x00ff9f),
            diff_del_bg: hsla(0.0, 0.7, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xff2e97),
            review_comment_bg: hsla_from_hex(0x0f1a2e),
            review_comment_indicator: hsla_from_hex(0xfcee0a),
        },
    }
}

pub fn github_light() -> IdeTheme {
    IdeTheme {
        name: "GitHub Light",
        description: "GitHub's clean light theme",
        editor: EditorColors {
            cursor: hsla_from_hex(0x0969da),
            selection: hsla(0.583, 0.60, 0.60, 0.25),
            line_number: hsla_from_hex(0x8c959f),
            line_number_active: hsla_from_hex(0x24292f),
            gutter_bg: hsla_from_hex(0xffffff),
            search_match: hsla(0.114, 0.80, 0.55, 0.30),
            search_match_active: hsla(0.114, 0.80, 0.55, 0.50),
            current_line: hsla(0.15, 0.15, 0.93, 1.0),
            bracket_match: hsla(0.583, 0.50, 0.70, 0.50),
            word_highlight: hsla(0.0, 0.0, 0.0, 0.07),
            indent_guide: hsla(0.0, 0.0, 0.0, 0.08),
            indent_guide_active: hsla(0.0, 0.0, 0.0, 0.18),
            fold_marker: hsla_from_hex(0x8c959f),
            diagnostic_error: hsla_from_hex(0xcf222e),
            diagnostic_warning: hsla_from_hex(0x9a6700),
            diagnostic_info: hsla_from_hex(0x0969da),
            diagnostic_hint: hsla_from_hex(0x8c959f),
        },
        syntax: SyntaxColors {
            keyword: hsla_from_hex(0xcf222e),
            keyword_control: hsla_from_hex(0xcf222e),
            type_name: hsla_from_hex(0x0550ae),
            type_builtin: hsla_from_hex(0x0550ae),
            function: hsla_from_hex(0x8250df),
            function_definition: hsla_from_hex(0x8250df),
            function_method: hsla_from_hex(0x8250df),
            function_macro: hsla_from_hex(0x8250df),
            string: hsla_from_hex(0x0a3069),
            string_escape: hsla_from_hex(0x0550ae),
            string_regex: hsla_from_hex(0x0a3069),
            number: hsla_from_hex(0x0550ae),
            comment: hsla_from_hex(0x6e7781),
            comment_doc: hsla_from_hex(0x57606a),
            operator: hsla_from_hex(0x24292f),
            variable: hsla_from_hex(0x24292f),
            variable_parameter: hsla_from_hex(0x953800),
            variable_builtin: hsla_from_hex(0x0550ae),
            constant: hsla_from_hex(0x0550ae),
            boolean: hsla_from_hex(0x0550ae),
            property: hsla_from_hex(0x0550ae),
            punctuation: hsla_from_hex(0x24292f),
            punctuation_bracket: hsla_from_hex(0x24292f),
            punctuation_delimiter: hsla_from_hex(0x24292f),
            attribute: hsla_from_hex(0x8250df),
            namespace: hsla_from_hex(0xcf222e),
            tag: hsla_from_hex(0x116329),
            heading: hsla_from_hex(0x0550ae),
            emphasis: hsla_from_hex(0x24292f),
            link: hsla_from_hex(0x0969da),
            literal: hsla_from_hex(0x0550ae),
            embedded: hsla_from_hex(0x24292f),
            constructor: hsla_from_hex(0x8250df),
            label: hsla_from_hex(0x0550ae),
            default_fg: hsla_from_hex(0x24292f),
        },
        terminal: TerminalColors {
            palette: [
                rgba_from_hex(0x24292f),
                rgba_from_hex(0xcf222e),
                rgba_from_hex(0x116329),
                rgba_from_hex(0x9a6700),
                rgba_from_hex(0x0969da),
                rgba_from_hex(0x8250df),
                rgba_from_hex(0x1b7c83),
                rgba_from_hex(0x6e7781),
                rgba_from_hex(0x57606a),
                rgba_from_hex(0xa40e26),
                rgba_from_hex(0x1a7f37),
                rgba_from_hex(0x7d4e00),
                rgba_from_hex(0x218bff),
                rgba_from_hex(0x8250df),
                rgba_from_hex(0x3192aa),
                rgba_from_hex(0x8c959f),
            ],
            fg: rgba_from_hex(0x24292f),
            bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            },
        },
        chrome: ChromeColors {
            bg: hsla_from_hex(0xf6f8fa),
            header_border: hsla(0.0, 0.0, 0.0, 0.10),
            accent: hsla_from_hex(0x0969da),
            dim: hsla_from_hex(0xd0d7de),
            bright: hsla_from_hex(0x24292f),
            panel_bg: hsla_from_hex(0xf6f8fa),
            editor_bg: hsla_from_hex(0xffffff),
            text_secondary: hsla_from_hex(0x57606a),
            diff_add_bg: hsla(0.33, 0.6, 0.5, 0.15),
            diff_add_text: hsla_from_hex(0x116329),
            diff_del_bg: hsla(0.0, 0.6, 0.5, 0.15),
            diff_del_text: hsla_from_hex(0xcf222e),
            review_comment_bg: hsla_from_hex(0xfff8c5),
            review_comment_indicator: hsla_from_hex(0x9a6700),
        },
    }
}

impl SyntaxColors {
    pub fn color_for_capture(&self, capture_name: &str) -> Hsla {
        match capture_name {
            "keyword"
            | "keyword.operator"
            | "keyword.function"
            | "keyword.return"
            | "keyword.directive"
            | "keyword.modifier"
            | "keyword.type"
            | "keyword.coroutine"
            | "keyword.storage.type"
            | "keyword.storage.modifier"
            | "keyword.import"
            | "keyword.conditional"
            | "keyword.exception"
            | "keyword.repeat" => self.keyword,

            "keyword.control"
            | "keyword.control.repeat"
            | "keyword.control.conditional"
            | "keyword.control.import"
            | "keyword.control.exception"
            | "conditional"
            | "repeat"
            | "include"
            | "exception"
            | "preproc" => self.keyword_control,

            "type" | "type.definition" | "type.qualifier" | "storageclass" | "structure"
            | "type.interface" => self.type_name,

            "type.builtin" => self.type_builtin,

            "function" | "function.call" => self.function,
            "function.definition" | "function.special.definition" => self.function_definition,
            "function.method" | "function.method.builtin" | "method" | "method.call" => {
                self.function_method
            }
            "function.macro" | "function.builtin" | "function.special" => self.function_macro,
            "constructor" => self.constructor,

            "string"
            | "string.special"
            | "string.special.url"
            | "string.special.path"
            | "string.special.key"
            | "string.special.symbol"
            | "string.special.regex"
            | "character" => self.string,
            "string.escape" | "character.special" | "escape" | "constant.character" => {
                self.string_escape
            }
            "string.regex" => self.string_regex,

            "number" | "number.float" | "float" | "constant.numeric" => self.number,

            "comment" | "comment.line" | "comment.block" | "comment.unused" => self.comment,
            "comment.documentation" | "comment.doc" => self.comment_doc,

            "operator" | "delimiter" => self.operator,

            "variable" | "variable.member" | "field" => self.variable,
            "variable.parameter" | "parameter" => self.variable_parameter,
            "variable.builtin" => self.variable_builtin,

            "constant" | "constant.builtin" | "constant.macro" | "define" | "symbol" => {
                self.constant
            }
            "boolean" => self.boolean,

            "property" | "property.definition" => self.property,

            "punctuation" | "punctuation.special" => self.punctuation,
            "punctuation.bracket" => self.punctuation_bracket,
            "punctuation.delimiter" => self.punctuation_delimiter,

            "attribute" | "annotation" | "decorator" => self.attribute,
            "label" => self.label,

            "namespace" | "module" | "module.builtin" => self.namespace,

            "tag" | "tag.builtin" | "tag.delimiter" | "tag.attribute" | "tag.error" => self.tag,

            "text.title" | "markup.heading" | "text.strong" | "markup.bold" => self.heading,

            "text.emphasis" | "markup.italic" => self.emphasis,

            "text.uri" | "markup.link.url" | "markup.link" | "text.reference" => self.link,

            "text.literal" | "markup.raw" => self.literal,

            "embedded" | "injection.content" => self.embedded,

            _ => self.default_fg,
        }
    }
}
