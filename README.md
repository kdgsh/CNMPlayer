<h1 align="center">CNMPlayer</h1>

<p align="center">
	<a href="README.md">English</a>
	&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;
	<a href="README_zh.md">简体中文</a>
</p>

<p align="center" style="color:gray;">
	A Rust TUI client for NetEase Cloud Music, with an embedded fullscreen playback page.
</p>

<p align="center">
	<img src="https://img.shields.io/badge/Rust-2024-orange" alt="Rust">
	<img src="https://img.shields.io/badge/Platform-Linux-informational" alt="Platform">
	<img src="https://img.shields.io/badge/License-AGPL--3.0-blue" alt="License">
	<img src="https://img.shields.io/github/stars/professor-lee/CNMPlayer?style=flat&label=Stars&color=FFC700&logo=github&logoColor=white" alt="Stars">
</p>

<h2 align="center">Project Overview</h2>

CNMPlayer, short for Customized Netease Music Player, is a terminal music client for NetEase Cloud Music.
It supports QR code, email, and phone login, browses recommendations and search results, and streams music in the terminal with local caching.

When you switch into fullscreen playback, CNMPlayer uses the embedded TMPlayer playback page.

<h2 align="center">Features</h2>

- QR code, email, and phone login
- Session cookie restore on startup
- Home recommendations, playlist pages, author pages, album pages, and search pages
- Search suffix filters: `@single`, `@album`, `@list`, `@author`, and the `@artist` alias
- Stream playback with a local audio cache
- Audio quality selection with VIP-aware clamping
- Page lyrics overlay on content pages
- Theme switching, language switching, transparency, hint toggles, and configurable keybinds
- Playback visualizer with bars or oscilloscope mode
- Embedded fullscreen playback page integrated from TMPlayer
- Cache cleanup controls for downloaded audio

<h2 align="center">Tech Stack</h2>

- Rust 2024
- TUI: ratatui + crossterm
- Networking: tokio + reqwest + ncm-api-rs
- Playback: rodio + symphonia + cpal
- Metadata and artwork: lofty + image + qrcode
- Visualization: cava, with optional `bundle-cava` embedding
- Fullscreen playback integration: TMPlayer

<h2 align="center">Development Setup</h2>

### Terminal Font

The UI uses icon glyphs in several places. A Nerd Font is strongly recommended; otherwise some icons may render as missing glyph boxes.

### Requirements (Linux)

Install the build dependencies provided by your distribution. On Debian/Ubuntu, this is usually enough:

```bash
sudo apt update
sudo apt install -y pkg-config libasound2-dev libdbus-1-dev libchromaprint-dev
```

### Spectrum Visualization (`cava`)

CNMPlayer uses `cava` for the live spectrum visualizer.
If `cava` is not available, the app still runs, but the spectrum bars stay empty.

The executable lookup order is:

1. `TMPLAYER_CAVA`
2. `./cava`
3. `./third_party/cava/cava`
4. `cava` in `PATH`

If you want to bundle a `cava` binary into the build, enable:

```bash
cargo build --release --features bundle-cava
```

### Run

For development:

```bash
cargo run
```

### Release build

```bash
cargo build --release
./target/release/cnmplayer
```

On first run, the app creates its configuration files under your OS config directory.

- Linux: `~/.config/cnmplayer`

You can override the asset root with `CNMPLAYER_ASSET_DIR`. The app will still use `config/`, `themes/`, and `auth/` under that root.

Audio cache files are stored under your OS cache directory unless you set `cache.path` in `config/default.toml`.

<h2 align="center">Configuration</h2>

- `config/default.toml`: application settings, playback settings, keybinds, and cache policy
- `themes/*.toml`: theme definitions
- `auth/session.toml`: persisted login cookie
- Cache root: OS cache directory by default, or `cache.path` if you set one

Important settings in `config/default.toml`:

- Interface: `theme`, `language`, `transparent_background`, `show_hints`
- Login banner: `default_opening_title`
- Playback layout: `visualize`, `page_lyrics`, `album_border`, `kitty_graphics`, `kitty_cover_scale_percent`
- Bars mode: `super_smooth_bar`, `bars_gap`, `bar_number`, `bar_channels`, `bar_channel_reverse`
- Audio quality: `audio_quality`
- Keybinds: `keybind_search_box`, `keybind_fullscreen`, `keybind_settings`, `keybind_sidebar`, `keybind_quit`, `keybind_prev`, `keybind_next`, `keybind_toggle_play_pause`, `keybind_toggle_mode`
- Cache policy: `cache.path`, `cache.clean_strategy`, `cache.max_size_mb`, `cache.max_age_days`, `cache.clean_on_startup`

The available audio quality levels are:

- `standard`
- `higher`
- `exhigh`
- `lossless`
- `hires`
- `jyeffect`
- `sky`
- `dolby`
- `jymaster`

If the current account does not have VIP access, CNMPlayer clamps the quality to the free range.

<h2 align="center">Keyboard Shortcuts</h2>

Global shortcuts:

- `Ctrl+S`: open the search box
- `Ctrl+F`: open fullscreen playback
- `T`: open settings
- `P`: toggle the playback sidebar
- `Q`: quit the host app
- `Esc`: close overlays or go back from the current page
- `Ctrl+K`: open help
- `Alt+Space`: toggle play/pause
- `Alt+Left`: previous track
- `Alt+Right`: next track
- `Alt+M`: toggle repeat mode

Login page:

- `F1`: QR login
- `F2`: username login
- `F3`: phone login
- `Tab` / `Up` / `Down`: switch focus
- `Enter`: confirm or submit

Search box:

- `Enter`: run the search
- `Esc` / `Ctrl+S`: close the search box
- `Backspace`: delete text
- Arrow keys: move the cursor

Search, playlist, and author pages:

- `Enter`: open or play the focused item
- `Esc` or `Left`: go back
- `Tab` / `Down`: move to the next item
- `Shift+Tab` / `Up`: move to the previous item

Settings keybind page:

- `Enter`: start rebinding the selected shortcut
- `Ctrl+Alt+R`: reset keybinds to defaults
- `Esc`: return

<h2 align="center">Related Projects</h2>

- [TMPlayer](https://github.com/professor-lee/TMPlayer): fullscreen playback UI used by CNMPlayer
- [ncm-api-rs](https://github.com/imsyy/ncm-api-rs): NetEase Cloud Music API client used by CNMPlayer

<h2 align="center">License</h2>

CNMPlayer is licensed under [AGPL-3.0-only](LICENSE).

See [CITATION.cff](CITATION.cff) for the standard citation metadata and upstream references.