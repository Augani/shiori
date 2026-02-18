# Shiori (Beta)

A lightweight, GPU-accelerated code editor built for the age of AI coding agents.

## Why Shiori?

AI coding agents like Claude Code, Aider, and Codex are writing most of the code now. You spend less time typing and more time orchestrating — launching agents across terminals, reviewing diffs, and managing sessions.

Traditional editors ship megabytes of features you no longer need — IntelliSense, refactoring wizards, build system integrations — all consuming memory while your agents do the actual work.

Shiori takes the opposite approach: a fast, minimal editor where terminals are first-class citizens. Your RAM goes to the agents, not the editor.

## Platform Support

| Platform | Status |
|----------|--------|
| macOS | Beta |
| Windows | Coming soon |
| Linux | Coming soon |

## Features

- **First-class terminals** — Full PTY with ANSI rendering, 24-bit true color, mouse support, OSC 8 hyperlinks, and image display (Kitty protocol). Manage multiple sessions from a single window.
- **Lightweight** — Single binary, ~80 MB idle memory. Handles 750K+ line files with smooth scrolling while using less memory than other editors we tested against on the same workload.
- **GPU-accelerated** — Built on [GPUI](https://github.com/Augani/adabraka-gpui). Rendering stays smooth even under heavy terminal output.
- **Git integration** — Side-by-side and unified diff views, file staging, inline review comments, and commit UI built in.
- **Syntax highlighting** — Tree-sitter powered across 22 languages: Rust, JavaScript, TypeScript, Python, Go, C, C++, Java, Ruby, Bash, CSS, HTML, JSON, TOML, Markdown, YAML, Lua, Zig, Scala, PHP, OCaml, and SQL.
- **LSP support** — Optional completions, diagnostics, hover, and go-to-definition. Pre-configured for rust-analyzer, typescript-language-server, pyright, gopls, clangd, lua-language-server, and zls. Turn it on when you want it, off when agents are driving.
- **Autocomplete** — Tree-sitter symbol extraction with prefix filtering and anchor-positioned popup.
- **File explorer** — Tree view with git status indicators.
- **Multi-tab editing** — Tabbed files with 2-second debounced autosave.
- **Find and replace** — Regex-supported search within the current file.
- **Theming** — 6 built-in themes: Island Dark, Dracula, Nord, Monokai Vivid, GitHub Dark, Cyberpunk.

## Performance

Tested against large files and high-throughput terminal workloads:

| Metric | Result |
|--------|--------|
| Idle memory | ~80 MB |
| 750K line file | Smooth scrolling, stable memory |
| Terminal 100K line burst | Peak ~235 MB, returns to baseline within 30s |
| Terminal binary throughput (5 MB xxd) | Peak ~226 MB, returns to baseline |
| Scrollback | Capped with O(1) trimming via VecDeque |
| 24-bit color | Semicolon and colon-separated SGR both supported |

## Install

### macOS (DMG)

Download the latest `.dmg` from [Releases](https://github.com/Augani/shiori/releases). Open the DMG and drag Shiori to Applications.

To use `shiori` from the terminal:

```bash
sudo ln -sf /Applications/Shiori.app/Contents/MacOS/shiori /usr/local/bin/shiori
```

### Build from source

Requires Rust nightly:

```bash
cargo +nightly build --release
```

The binary will be at `target/release/shiori`.

To create a macOS `.app` bundle:

```bash
bash macos/bundle.sh
```

## Usage

```bash
shiori                      # Open in current directory
shiori path/to/folder       # Open a folder
shiori path/to/file.rs      # Open a file
shiori file1.rs file2.rs    # Open multiple files
```

### Key bindings

| Shortcut | Action |
|----------|--------|
| `Ctrl + \`` | Toggle terminal |
| `Cmd + T` | New terminal tab |
| `Cmd + P` | Command palette |
| `Cmd + B` | Toggle sidebar |
| `Cmd + S` | Save file |
| `Cmd + F` | Find in file |
| `Cmd + G` | Toggle git panel |
| `Cmd + Shift + O` | Open folder |
| `Cmd + Shift + K` | Symbol outline |

## Architecture

Single-binary Rust application. All source in `src/`.

- **GPUI** (`adabraka-gpui`) — GPU-accelerated UI framework
- **ropey** — Rope-based text buffer for efficient large file handling
- **tree-sitter** — Syntax highlighting and symbol extraction
- **portable-pty** — Terminal PTY management
- **git2** — Git operations via libgit2
- **lsp-types** — Language Server Protocol support

## License

See [LICENSE](LICENSE).
