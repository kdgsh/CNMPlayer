<h1 align="center"><img src="logo.svg"/></h1>

<p align="center">
	<a href="README.md">English</a>
	&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;
	<a href="README_zh.md">简体中文</a>
</p>

<p align="center" style="color:gray;">
	基于 Rust 的网易云音乐 TUI 客户端，内置全屏播放页。
</p>

<p align="center">
    <img src="https://img.shields.io/badge/Language-Rust-orange?logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20Windows%20%7C%20macOS-informational?logo=linux&logoColor=white" alt="Platform">
    <img src="https://img.shields.io/badge/License-AGPL--3.0-blue?logo=opensourceinitiative&logoColor=white" alt="License">
    <img src="https://img.shields.io/github/stars/professor-lee/CNMPlayer?style=flat&label=Stars&color=FFC700&logo=github&logoColor=white" alt="Stars">
    <img src="https://img.shields.io/github/forks/professor-lee/CNMPlayer?style=flat&label=Forks&color=60adff&logo=git-fork&logoColor=white" alt="Forks">
    <img src="https://img.shields.io/github/v/release/professor-lee/CNMPlayer?color=32cd32&label=Release&logo=github-actions&logoColor=white" alt="Release">
    <img src="https://img.shields.io/github/last-commit/professor-lee/CNMPlayer?color=rebeccapurple&logo=git&logoColor=white" alt="Last Commit">
	<img src="https://img.shields.io/github/commit-activity/m/professor-lee/CNMPlayer?style=flat&color=FF69B4&logo=github" alt="Commit Activity">
	<img src="https://img.shields.io/github/languages/code-size/professor-lee/CNMPlayer?style=flat&color=blueviolet" alt="Code Size">
</p>

<h2 align="center">项目概述</h2>

CNMPlayer（Customized Netease Music Player）是一个运行在终端中的网易云音乐客户端。
它支持二维码、邮箱和手机号登录，可以浏览推荐、搜索结果、歌单、作者页和专辑页，并把歌曲流式播放到终端中，同时缓存音频到本地。

切换到全屏播放时，CNMPlayer 使用内置的 TMPlayer 播放页。

<h2 align="center">已有功能</h2>

- 二维码、邮箱、手机号登录
- 启动时自动恢复上次登录会话
- 首页推荐、歌单页、作者页、专辑页和搜索页
- 搜索后缀筛选：`@single`、`@album`、`@list`、`@author`，以及 `@artist` 别名
- 流式播放，并带本地音频缓存
- 支持按 VIP 权限自动裁剪的音质选择
- 内容页页面歌词浮层
- 主题切换、语言切换、透明背景、提示开关和可配置快捷键
- 频谱条 / 示波器可视化
- 内置的全屏播放页，来源于 TMPlayer
- 下载音频的缓存清理控制

<h2 align="center">技术栈</h2>

- Rust 2024
- TUI：ratatui + crossterm
- 网络：tokio + reqwest + ncm-api-rs
- 播放：rodio + symphonia + cpal
- 元数据与封面：lofty + image + qrcode
- 可视化：外部 `cava`
- 全屏播放整合：TMPlayer

<h2 align="center">开发与运行</h2>

### 终端字体

界面中有一些图标字形，强烈建议使用 Nerd Font；如果没有这类字体，部分图标可能显示为缺字方块。

### 依赖（Linux）

请安装发行版提供的构建依赖。以 Debian/Ubuntu 为例：

```bash
sudo apt update
sudo apt install -y build-essential cmake pkg-config libasound2-dev libdbus-1-dev libchromaprint-dev
```

### 频谱可视化（`cava`）

CNMPlayer 会查找外部 `cava` 可执行文件来生成实时频谱可视化。
如果系统里没有 `cava`，程序仍然可以运行，但条形频谱会保持空白。

可执行文件的查找顺序如下：

1. `TMPLAYER_CAVA`
2. `<可执行文件目录>/cava`
3. `<可执行文件目录>/third_party/cava/cava`
4. `<当前工作目录>/third_party/cava/cava`
5. `PATH` 里的 `cava`

### 运行

开发环境运行：

```bash
cargo run
```

### Release 构建

```bash
cargo build --release
./target/release/cnmplayer
```

首次运行时，程序会在系统配置目录下创建配置文件。

- Linux：`~/.config/cnmplayer`

你可以使用 `CNMPLAYER_ASSET_DIR` 覆盖资源根目录。程序仍然会在这个根目录下使用 `config/`、`themes/` 和 `auth/` 子目录。

音频缓存默认保存在系统缓存目录中；如果你在 `config/default.toml` 里设置了 `cache.path`，则会改用该目录。

<h2 align="center">配置</h2>

- `config/default.toml`：程序配置、播放配置、快捷键和缓存策略
- `themes/*.toml`：主题定义
- `auth/session.toml`：持久化登录 cookie
- 缓存根目录：默认使用系统缓存目录，也可以通过 `cache.path` 指定

`config/default.toml` 里比较重要的配置项：

- 运行参数：`ui_fps`、`spectrum_hz`、`mpris_poll_ms`
- 界面：`theme`、`language`、`transparent_background`、`show_hints`、`home_more_recommend`
- 登录标题：`default_opening_title`
- 播放布局：`visualize`、`page_lyrics`、`album_border`、`kitty_graphics`、`kitty_cover_scale_percent`
- Bars 模式：`super_smooth_bar`、`bars_gap`、`bar_number`、`bar_channels`、`bar_channel_reverse`
- 播放行为：`audio_quality`、`audio_preload`、`playback_memory`、`resume_last_position`
- 歌词与识别：`lyrics_cover_fetch`、`lyrics_cover_download`、`audio_fingerprint`、`acoustid_api_key`
- 全局快捷键：`keybind_search_box`、`keybind_fullscreen`、`keybind_settings`、`keybind_sidebar`、`keybind_quit`、`keybind_prev`、`keybind_next`、`keybind_toggle_play_pause`、`keybind_toggle_mode`
- 全屏快捷键：`keybind_fullscreen_prev`、`keybind_fullscreen_next`、`keybind_fullscreen_toggle_play_pause`、`keybind_fullscreen_toggle_mode`、`keybind_fullscreen_eq`、`keybind_fullscreen_eq_reset`、`keybind_toggle_like_fullscreen`
- 折叠栏快捷键：`keybind_toggle_like_collapsed`
- 缓存策略：`cache.path`、`cache.clean_strategy`、`cache.max_size_mb`、`cache.max_age_days`、`cache.clean_on_startup`

可用的音质档位：

- `standard`
- `higher`
- `exhigh`
- `lossless`
- `hires`
- `jyeffect`
- `sky`
- `dolby`
- `jymaster`

如果当前账号没有 VIP 权限，程序会把音质限制到免费档位。

<h2 align="center">快捷键</h2>

可配置快捷键（默认值）：

- `Ctrl+S`：打开搜索框
- `Ctrl+F`：打开全屏播放页
- `T`：打开设置
- `P`：切换侧边栏
- `Q`：退出主程序
- `Esc`：关闭浮层或返回当前页面
- `Alt+Space`：播放 / 暂停
- `Alt+Left`：上一首
- `Alt+Right`：下一首
- `Alt+M`：切换循环模式
- `Left`：全屏上一首
- `Right`：全屏下一首
- `Space`：全屏播放 / 暂停
- `M`：切换全屏播放模式
- `E`：切换全屏 EQ
- `Alt+R`：重置全屏 EQ
- `L`：在全屏页切换收藏状态
- `Alt+L`：在折叠播放器栏切换收藏状态

额外的固定快捷键：

- `Ctrl+Up` / `Ctrl+Down`：在侧边栏展开时切换歌单分区（用户创建 / 用户收藏）
- `Ctrl+K`：打开帮助

登录页：

- `F1`：二维码登录
- `F2`：用户名登录
- `F3`：手机号登录
- `Q`：退出程序
- `Tab` / `↑` / `↓`：切换焦点
- `Enter`：确认或提交

搜索框：

- `Enter`：执行搜索
- `Esc` / `Ctrl+S`：关闭搜索框
- `Backspace`：删除文本
- 方向键：移动光标

搜索页、歌单页、作者页：

- `Enter`：打开或播放当前项
- `Esc` 或 `Left`：返回
- `Tab` / `Down`：切到下一项
- `Shift+Tab` / `Up`：切到上一项

设置页的按键绑定：

- `Enter`：开始重绑当前快捷键
- `Ctrl+Alt+R`：恢复默认快捷键
- `Esc`：返回

<h2 align="center">相关项目</h2>

- [TMPlayer](https://github.com/professor-lee/TMPlayer)：CNMPlayer 使用的全屏播放页实现
- [ncm-api-rs](https://github.com/imsyy/ncm-api-rs)：CNMPlayer 使用的网易云音乐 API 客户端

<h2 align="center">许可证</h2>

CNMPlayer 采用 [AGPL-3.0-only](LICENSE) 许可证。

仓库内 vendored 代码的第三方归属与许可证声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

标准引用元数据和上游依赖请查看 [CITATION.cff](CITATION.cff)。

---
## Star History

[![Star History Chart](https://api.star-history.com/image?repos=professor-lee/CNMPlayer&type=date&legend=top-left)](https://www.star-history.com/?repos=professor-lee%2FCNMPlayer&type=date&legend=top-left)