# 踩坑记录（必读）

本项目实战踩过的坑，升级依赖或新增功能前先读这里。
所有结论均已在 egui 0.36.1 / rodio 0.22.2 / lofty 0.25.1 源码中核实。

## egui 0.36（相比网上大量旧教程的破坏性变化）

### App trait：`update(ctx)` 已删除

```rust
// 旧（≤0.35）                          // 新（0.36）
fn update(&mut self, ctx, frame)    →   fn logic(&mut self, ctx: &Context, frame)
                                     →   fn ui(&mut self, ui: &mut Ui, frame)
```

- `logic()`：每帧逻辑，窗口隐藏时也会调用（配合 request_repaint 链）——
  自动切歌、重绘调度放这里
- `ui()`：绘制，拿到的是 `&mut Ui` 而非 Context（`ui.ctx()` 获取）

### Panel API 变化

```rust
// 旧：egui::TopBottomPanel::top("id").show(ctx, |ui| ...)
// 新：
egui::Panel::top("id").show(ui, |ui| ...)
egui::CentralPanel::default().show(ui, |ui| ...)
```

### Slider 无视 add_sized 宽度（导致总时长距进度条 336px 的元凶）

Slider 用 `ui.spacing().slider_width`（默认 100px）决定宽度，`add_sized`
分配的区域只推进光标不拉伸滑块：

```rust
// 错：ui.add_sized([w, h], Slider::new(...))
// 对：
ui.spacing_mut().slider_width = w;
ui.add(egui::Slider::new(...));
```

### 点击测试：后注册者在顶层

给整行加透明交互层（`ui.interact(rect, id, sense)`）会**遮挡之前绘制的**
行内按钮的点击。解决：hit 区域排除按钮区，或先注册 hit 层再画按钮。

### 其他 API 变化

| 旧 | 新（0.36） |
|---|---|
| `FontData` 直接存入 map | `Arc<FontData>`（`Arc::new(FontData::from_owned(...))`) |
| `Frame::new()` | 不存在，用 `Frame::NONE` 或 `Frame::default()` |
| `RichText::strong(bool)` | `strong()` 无参 |
| `Painter::rect(...)` | 新增第 5 参 `StrokeKind::Inside` |
| `Response::drag_released()` | `drag_stopped()` |
| `add_enabled_sized()` | 不存在，用 `allocate_ui` + `set_enabled` 不行（Ui 无此方法），换 `add_enabled` 或跳过 |
| `TopBottomPanel` | `egui::Panel::top/bottom/left/right` |
| 弹窗 | `egui::Popup`（`from_response` / `from_toggle_button_response` / `context_menu`），关闭用 `ui.close()` |

### egui 无 CJK 字体

默认字体无中文字形，必须启动时从系统路径探测加载
（Noto CJK / 文泉驿 / DroidSansFallback），否则中文全是方块。

### `ui.horizontal` 垂直居中但按钮框高随内容变

不同字号图标的 Button 框高不同，视觉上错位——统一 `Button::min_size` 解决。

## rodio 0.22

- `OutputStream` / `Sink` 已删除：`DeviceSinkBuilder::open_default_sink()`
  → `MixerDeviceSink`（必须保活）→ `Player::connect_new(&device.mixer())`
- `Decoder::try_from(File)` 是新入口（构造时即探测格式，可提前报错）
- `Player` 内建 `get_pos / try_seek / empty / set_volume / stop / clear`
  （`stop` 只清队列、`clear` 清队列且暂停）
- `append()` 与 `empty()` 生效间有竞态：append 后约 300ms 内不要用 `empty()`
  判定播完
- `set_volume` 写入共享 controls，跨曲目持久，无需每曲重设
- `stop()` 后 `append()` 自动恢复播放（player.rs append 内部处理）

## lofty 0.25

trait 方法需显式导入作用域，否则报 "no method named"：

```rust
use lofty::file::{AudioFile, TaggedFileExt};  // primary_tag/first_tag/properties
use lofty::tag::Accessor;                     // title/artist/album
```

## GNOME Wayland 标题栏（标题乱码 + 按钮错位根因）

winit(w) 在 GNOME Wayland 下写入的 `WM_NAME`（Latin-1）存在双重编码，
中文标题在标题栏显示乱码；客户端标题栏按钮渲染也不可靠。
实测（xprop）：`_NET_WM_NAME`(UTF8) 正确、`WM_NAME` 乱码。

**解决**：main.rs 移除 `WAYLAND_DISPLAY` 强制走 X11/XWayland——
Mutter 的 X11 服务端装饰成熟可靠（按 EWMH 优先读 `_NET_WM_NAME`）。

注意：winit 0.29+ 已移除 `WINIT_UNIX_BACKEND` 环境变量，
后端选择只看 `WAYLAND_DISPLAY` / `DISPLAY` 是否存在。

## rfd 0.17（xdg-portal 后端）

构建期无需 GTK，但**运行期**依赖 D-Bus 会话 + xdg-desktop-portal
（主流桌面默认有）；极简 WM 环境下文件对话框可能无响应。

## 构建系统依赖（Linux）

```bash
sudo apt install libasound2-dev          # cpal/ALSA（rodio 必需）
# 以下 eframe/winit 需要（本机多数已有）
sudo apt install libxcb-render0-dev libxcb-shape0-dev \
     libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```
