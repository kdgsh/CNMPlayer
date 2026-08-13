# main → develop 功能迁移清单

将 main 分支（835eabe）的改进逐步迁移到 develop。每项完成后勾选并同步提交到 fork。
对应 main 提交可用 `git show <hash>` 查看具体改动。

## 1. 多音源支持（基础，先迁移）✅
- [x] 02afc3c `feat: add multi-source support with LX Music compatible custom source`（087ec68，已适配 cyper）
- [x] 96fe949 `fix: use X-Request-Key header and /lyric endpoint for custom source`（4fa792f，已适配 cyper）
- [x] 90b5b6b `feat: unlock lossless/hires audio quality with custom source`（2bddfb9）

## 2. 主题跟随系统 ✅
- [x] f590a57 `feat: dynamic system theme adapting to DMS accent color and dark/light mode`（f882cb2）
- [x] aca32f5 `feat: system theme follows Noctalia wallpaper accent`（1b5b5bc）

## 3. 顶栏导航重构 ✅
- [x] e947f4b `feat: replace sidebar with topbar tabs for user playlists/toplists/recommend`
- [x] 1428696 `fix: open daily recommendations from topbar recommend tab`
- [ ] c66e3c4 `refactor: remove in-app volume control`（伴随重构，按需）
- [ ] 90b1f97 `refactor: unify progress bar style with volume bar style`（伴随重构，按需）

## 4. 后台加载 + 封面磁盘缓存（核心性能）✅
- [x] 835eabe `feat: background topbar/playlist loads, cover disk cache, search suggest state, lyric merge, like toggle in main app`（大提交，按功能拆开迁移）
  - 顶栏 tab / 推荐分类后台加载（消除切换延迟）
  - 主页歌单→歌单页先进入（占位 + 后台加载）
  - 方向键导航到顶栏（tab_focus）
  - 封面磁盘缓存接入显示路径 + URL 归一化 + 内存图注入
  - 用户歌单加载并行化 + 启动并行（load_startup_tabs）
  - 缓存 sidecar（lrc/cover）清理
  - 红心状态字段（like 相关状态）
- [x] af0306a `feat: show loading and empty states for background playlist loads`（依赖上面的状态字段）

## 5. 搜索联想 ✅
- [x] 434b419 `feat: add live search suggestions with debounce in search box`（suggest/web 接口 + 300ms 防抖）
- 附加：Home 顶栏右侧常驻搜索栏（main 之外的自定义功能，搜索框常驻右上角，激活后联想下拉）

## 6. 歌词翻译与过滤
- [x] b9cb186 `feat: request netease tlyric translation with lrc in one call`
- [x] 263bfec `feat: show lyric translations and filter credit lines in fullscreen player`
- [x] 7d9fc04 `feat: add lyric translation setting and page rendering in main app`
- [x] 后续修复：所有音源统一走 api.lyric()（含翻译），自定义源不再跳过

## 7. 全屏增强
- [x] a24d756 `feat: show lyrics on top with spectrum at bottom in fullscreen panel`
- [ ] 645cefd `refactor: remove fullscreen header theme/mode status display`（按需，跳过）
- [x] 全屏歌词换行（wrap_text，超长歌词自动换行，随第 6 项完成）

## 8. 红心点击
- [ ] 2845559 `feat: click heart icon on player bar to toggle like`（播放条 + 全屏 meta 行）

## 9. 音源设置 UI ✅
- [x] f11a55d `feat: add custom source settings with availability test`（cd431c3，设置页填写 + 自动测试；UI 部分取自 835eabe）

## 备注
- develop 是 compio/cyper 生态（无 tokio），后台加载模式（shot_and_share/tokio::spawn）迁移时需适配
- 提交信息保持英文语义化（feat:/fix:/refactor:），与仓库风格一致
