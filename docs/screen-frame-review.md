原生屏幕边框：实现评审与修复方案

评审范围是 `border-notification-native` 相对本地 `master` 的功能改动，覆盖 macOS 和 Windows。原评审对应 `e94085a59`，基线为 `eab27cc24`。修复实施状态如下，每项单独提交。

| 独立改动 | 状态与验证 |
| --- | --- |
| SCK 回调生命周期 | 已修复；原生回归验证确认修复前销毁提前返回，修复后等待帧和错误回调结束，忽略后续回调；Objective-C 语法检查通过。 |
| 旧 macOS 本地边框 | 已移除显示入口的 SCK 能力限制；macOS `cargo check --locked --offline --lib --features flutter` 通过，旧系统实机待验证。 |
| 精确窗口过滤 | 已删除应用级兜底；临时过滤对象验证覆盖部分匹配、空列表及未匹配 ID，修复前失败、修复后通过；Objective-C 语法检查通过。 |
| 过滤更新失败处理 | 已改为受控日志后继续取帧，每代仅尝试一次；隔离故障注入验证帧可继续返回、停止错误仍传播，修复前失败、修复后通过；macOS 编译检查通过。 |
| 窗口 ID 与版本号快照 | 待实施。 |
| Windows 旧系统排除属性 | 待实施。 |

以下保留实施前的评审与方案记录，当前完成状态以上表为准。

`Cargo.lock` 中 `scrap` 对 `cc` 的构建依赖记录已在上一轮补齐，与 `libs/scrap/Cargo.toml` 保持一致，没有升级依赖版本。本轮只更新评审文档。

**产品要求：本地边框优先，采集排除尽力而为。** 存在符合条件的桌面会话时，被控端应显示边框。旧系统、后端回退或排除失败时继续显示，允许主控看到红框；不能为了隐藏远端红框而隐藏本地提示、排除整个应用或中断正常采集。不要求首次显示和重建过程中远端完全看不到任何一帧红框。

**当前功能**是在被控端显示红色屏幕边框，并尝试将边框从远端画面中排除。每个显示器由四个窄条原生窗口组成，颜色为 `#EF4438`，约 75% 不透明度，厚度为 6 像素。

共用判断位于 [ui_cm_interface.rs](../src/ui_cm_interface.rs) 的 `should_show_screen_frame()`：至少有一个已授权、未断开的桌面会话时显示；文件传输、摄像头、终端和端口转发会话不触发。新增连接、授权、断开、删除连接时重新计算，最后一个符合条件的会话结束后隐藏，CM 退出时清理。

当前会给所有显示器加框，没有根据远端观看的显示器筛选，也没有增加配置开关。CM 中的 `authorized` 是界面侧状态，不能等同于服务端所有授权检查已经完成，更不能代表实际已经输出视频帧。

| 实现部分 | Windows | macOS |
| --- | --- | --- |
| 窗口 | 每个显示器四个窄条 `HWND` | 每个显示器四个窄条 `NSPanel` |
| 执行线程 | 独立 `screen-frame` 线程和 Win32 消息循环 | 调度到 AppKit 主线程 |
| 输入行为 | 置顶、鼠标穿透、不激活窗口 | 高层级、忽略鼠标，配置跨 Space 和全屏显示 |
| 缩放 | 使用显示器坐标计算 6 像素边条 | 根据 `backingScaleFactor` 换算 6 像素厚度 |
| 显示器变化 | 响应 `WM_DISPLAYCHANGE`、`WM_DPICHANGED` 后重建 | 注册显示器配置变化回调后重建 |
| 排除采集 | `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` | 把边框窗口 ID 加入 ScreenCaptureKit 内容过滤器 |
| 不支持的情况 | 排除 API 失败后仍显示，符合新要求；旧系统的属性兼容性需要小修 | 系统不支持 SCK 时目前不显示，需要去掉这个限制；SCK 初始化失败回退后仍显示，符合新要求 |

Windows 的实现集中在 [windows/screen_frame.rs](../src/platform/windows/screen_frame.rs)。创建窗口后先设置透明度和采集排除属性，再调用 `SetWindowPos` 显示。隐藏时销毁窗口；工作线程保留消息循环供下一次显示使用。原有采集后端没有修改，不需要向采集进程传递这些窗口的 ID。

macOS 的实现分为原生窗口和采集两部分。[macos/screen_frame.rs](../src/platform/macos/screen_frame.rs) 在主线程创建边框，取得每个 `NSPanel` 的 `windowNumber`，通过现有 CM IPC 连接广播给服务端：

```text
CM 创建边框 NSPanel
  → 读取窗口 ID
  → ScreenFrameWindowIds IPC
  → 连接进程调用 scrap::set_excluded_window_ids
  → 更新排除列表及版本号
  → 采集器在取帧前发现版本变化
  → 更新 SCContentFilter
```

macOS 手动授权时，CM 先更新边框、发送窗口 ID，再发送 `Authorize`，目的是让采集开始前获得排除列表；Windows 发送 `Authorize` 后更新边框。显示器变化导致 macOS 窗口重建后，也会发送新 ID。当前没有“过滤器已经生效”的确认；按新要求不需要增加此握手。

[common/quartz.rs](../libs/scrap/src/common/quartz.rs) 在 macOS 12.3 及以上尝试使用 ScreenCaptureKit；通过 [sck.rs](../libs/scrap/src/quartz/sck.rs) 的 FFI 调用 [sck_capture.m](../libs/scrap/src/quartz/sck_capture.m)。后者动态加载系统框架，创建 `SCStream`，配置 BGRA、三帧队列和不包含鼠标光标的画面，通过 `IOSurface` 回到原有 Rust 帧处理和编码链路。不可用或初始化失败时回退到 `CGDisplayStream`。运行中收到流错误则返回采集错误，并不是在同一个采集器内自动切换后端。

**关于为什么在 CM 判断，需要先区分三类进程职责。** “IPC server 所在进程”不唯一，不能仅按 IPC 的监听端名称决定功能归属。

| 组件 | 当前职责与代码依据 |
| --- | --- |
| CM（`--cm`） | 创建 GUI、持有展示用 `CLIENTS` 列表；`ui_cm_interface::start_ipc()` 调用 `new_listener("_cm")`，所以它本身就是 `_cm` IPC 的监听端。 |
| 连接和采集进程（典型为 `--server`） | 持有真实 `Connection`、执行授权、管理采集；`server/connection.rs` 的 `start_ipc()` 主动连接 `_cm`，必要时启动 CM。 |
| 系统服务 | 启动或管理后台进程，另有服务 IPC；Windows 的服务会通过 `launch_server(session_id, ...)` 在目标会话启动 `--server`。服务进程与 `--server` 不是同一个概念。 |

相关入口是 [flutter.rs](../src/flutter.rs) 的 `cm_init()` / `start_listen_ipc()`、[ui_cm_interface.rs](../src/ui_cm_interface.rs) 的 `start_ipc()`、[server/connection.rs](../src/server/connection.rs) 的 `start_ipc()`，以及 [platform/windows.rs](../src/platform/windows.rs) 的 `run_service()` / `launch_server()`。

当前放在 CM 的工程理由是：现成的连接列表可以复用，连接增删和授权点击都有钩子；CM 又有图形界面生命周期，适合拥有边框窗口。macOS 尤其需要 AppKit 主线程；后台路径不保证具备相同的 GUI 条件。Windows 虽然可以单独创建窗口线程，也仍然需要考虑交互会话和桌面。这个选择减少了初版的跨进程协议改动。

**当前会话信息已经由连接进程传给 CM。** 这里的连接进程是被控端本机负责会话和采集的 RustDesk 进程，不是 `hbbs` / `hbbr` 中继服务。已有流程如下：

```text
连接进程 → Login{会话类型, authorized, ...} → CM 更新 CLIENTS
CM 点击允许 → 本地 authorized = true → Authorize → 连接进程继续授权
连接进程 → Close / Disconnected → CM 删除连接或标记断开
CM 根据 CLIENTS 判断是否需要本地边框
macOS CM → 边框窗口 ID → 连接/采集进程更新排除列表
```

密码登录或会话恢复等路径会在服务端处理授权后，通过 `try_start_cm(..., self.authorized)` 把状态传给 CM。手动允许路径则是 CM 先更新本地状态再请求服务端处理，因此边框可能先于真正采集开始出现。实际授权仍由 `Connection::send_logon_response_and_keep_alive()` 检查；画框不会绕过授权。断开状态通过现有 `Close` / `Disconnected` 及 IPC 任务退出清理传播。

之前建议的“服务端提供会话和采集能力状态”，包含进一步确认授权以及新增“当前实际使用 SCK 还是 CGDisplayStream、是否能够排除边框”的通知，让 CM 决定是否显示。后半部分目前不存在，CM 调用 `uses_screen_capture_kit()` 只是在自己的进程检查系统是否具备 SCK，不能知道另一进程实际选了什么后端。

**按新要求撤回采集能力通知和显示握手建议，保留 CM 判断。** 本地边框是否显示不依赖采集是否能排除它，因此无需新增 IPC、连接计数器或后端能力汇总。CM 继续复用现成会话状态并管理窗口，采集进程继续尽力排除窗口即可。为消除短暂提前显示而迁移判断也没有必要；以后若明确要求“首帧真正发出才画框”，再单独讨论该语义。

系统登录前没有 CM、锁屏或安全桌面切换等仍需明确支持范围；本次没有承诺扩展这些场景，也不因为采集排除需求引入新的 GUI 宿主进程。

原五项问题重新评估如下：

| 原问题 | 新结论 | 建议范围 |
| --- | --- | --- |
| 1. macOS 回调与销毁竞争 | 保留，优先修 | SCK FFI 回调和资源释放路径 |
| 2. macOS 回退旧后端后仍显示边框 | 撤销原风险，属于预期行为 | 另外去掉旧 macOS 无 SCK 就不画框的限制 |
| 3. macOS 误排除整个应用 | 保留，修法可以简化 | 删除应用级兜底；只排除找到的边框，失败不终止正常采集 |
| 4. macOS ID 与版本号快照不一致 | 保留低成本修复，优先级降低 | 持有现有互斥锁直到取得完整快照 |
| 5. Windows 排除失败仍显示 | 这部分撤销；只保留旧系统兼容处理 | 旧系统跳过排除 API，仍显示可被采集的正常红框 |

1. **[P1] macOS 回调与销毁竞争。** `scrap_sck_destroy()` 在停止采集前清空 `frameCallback` 和 `errorCallback`；回调执行处分别读取指针进行判空和调用。线程交错时存在空指针调用风险。停止超时被忽略后仍释放 Rust `CallbackState`，也缺少明确的回调生命周期保证。这是静态检查发现的风险，尚未实机复现崩溃。

   建议让帧回调、错误回调和 context 注销使用同一个局部同步机制。回调在受保护的区域检查 context 并完成 Rust 调用；销毁在同一保护下注销 context，确保已进入的调用结束、后来的调用直接返回，再停止采集和移除输出。不能仅把函数指针改成原子变量，因为它不能保护 context 的生命周期。等待系统停止时不能持有这把锁，否则可能与回调互相等待。正常销毁和初始化失败要走同一套清理，停止超时及移除输出失败必须记录，释放 Rust context 前必须保证任何后续回调都不能再访问它。

   验证重点是断开、切换显示器、启动失败和关闭超时路径下没有崩溃、死锁或已释放内存访问；使用实机反复启停及可用的内存检查工具，不为此引入一套庞大的测试框架。

2. **撤销“回退后仍显示边框”的风险。** SCK 初始化失败后使用 CGDisplayStream，边框进入主控画面，正是允许的降级结果。保留回退，不通知 CM 隐藏边框，不因为边框排除失败暂停发帧。

   另有一个与新要求相反的现有条件需要改：[macos/screen_frame.rs](../src/platform/macos/screen_frame.rs) 的 `set_visible()` 先执行 `visible && scrap::uses_screen_capture_kit()`，导致系统完全不支持 SCK 时连本地边框也不显示。建议只去掉这里的能力限制；采集后端选择仍检查 SCK 是否可用。这样旧 macOS 也正常画框，主控可以看到它，无需增加跨进程状态。

3. **[P2] macOS 可能误排除整个 RustDesk 应用。** `ScrapSckBuildFilter()` 在初始无 ID 或没有枚举齐指定窗口时，按 bundle ID / 进程名排除整个应用。主界面、设置等普通窗口也会消失。若返回成功，采集器便记录该版本已处理；窗口随后可枚举并不会自动触发重试，因此可能持续到下次 ID 变化。

   建议删除 `ScrapSckFindOwnApplications()` 及应用级过滤分支，始终使用 `excludingWindows`：找到哪些请求的边框窗口就排除哪些，没有找到就不排除。无需强求枚举齐全，也不必为短暂显示红框增加重试定时器或握手；不匹配的边框留在主控画面是允许的降级，普通 RustDesk 窗口不能因此消失。

   同时处理已有错误传播：[common/quartz.rs](../libs/scrap/src/common/quartz.rs) 的 `frame()` 对 `update_excluded_window_ids()` 使用 `?`，过滤更新失败会变成取帧错误，[video_service.rs](../src/server/video_service.rs) 会退出当前采集循环。这不符合排除尽力而为的要求。建议对排除更新失败记录受控日志，保留仍正常工作的流和现有精确过滤器，继续取帧；用现有版本号记录这一代已尝试，避免每帧重复查询或等待，下一代变化或采集器重建再尝试。真实的流停止错误仍应走原有错误处理，不能一并吞掉。

   验证普通 RustDesk 窗口保持可见；精确匹配时排除边框，部分匹配或更新失败时本地仍有完整红框、远端允许看到未排除的边框，正常采集不能因此终止或持续反复阻塞。

4. **macOS 排除列表和版本号不是一致快照，保留局部修复。** `excluded_window_ids()` 克隆列表后释放锁，再读取原子版本号。写线程可以在两步之间更新，导致读到“旧 ID + 新版本号”，采集器因版本相同而漏更新。已用可控线程交错的独立验证程序复现这一组合。远端允许看到边框后，此问题的影响降低，但修复很小，没有理由增加复杂状态来绕开它。

   最小修法是显式保留现有 `MutexGuard`，在锁内读取版本号、克隆 ID，再一起返回。写侧本来就在持锁期间修改列表和递增版本号，无需增加状态、换锁或设计新缓存。验证更新后的快照始终对应同一代数据；不向生产代码加入测试专用同步钩子。

5. **Windows 排除失败仍显示符合要求，只处理旧系统的兼容行为。** API 失败后记日志并继续显示的现有路径应保留。需要小修的是 `WDA_EXCLUDEFROMCAPTURE` 在 Windows 10 2004 以前会按 `WDA_MONITOR` 处理，采集结果可能是没有内容的边条，而不是允许显示的正常红框。依据是 [Microsoft SetWindowDisplayAffinity 文档](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)。

   建议复用 `base::platform::windows::is_windows_version_or_greater(10, 0, 19041, 0, 0)`，仅在支持的系统上尝试设置排除属性；旧系统直接跳过此调用，以默认的可采集属性显示红框。受支持系统上调用失败也照常显示，不销毁窗口。窗口创建、透明度设置或显示本身失败仍应保持原有资源清理；不要把这些失败和“无法排除采集”混为一谈。

   验证 DXGI / GDI 实际路径下，本地始终显示边框；系统支持且排除成功时远端看不到边框，不支持或排除失败时远端可以看到正常红框。能力判断不应控制边框的创建或显示。

建议修复 1、3、4，另做两项很小的兼容调整：macOS 去掉绘图对 SCK 可用性的依赖；Windows 只对排除 API 做版本检查。保留现有 CM 会话判断和 IPC 结构，取消新增采集能力通知、隐藏确认及过滤器就绪握手。以上是修订后的方案，本次没有实施运行时改动。

**验证记录与边界。** 上一轮 review 在 macOS 上执行了 `cargo test --locked --offline --lib --features flutter ui_cm_interface::tests::screen_frame_tracks_only_active_screen_sessions -- --exact`，1 项测试通过；Objective-C 的 `clang -fsyntax-only` 检查通过；窗口 ID / 版本号竞态已用独立程序复现。这些不等于已完成图形界面端到端验证。尚未测试 Windows 实机、多屏、全屏、锁屏及双端断线重连场景。

本轮只修订本文档，`Cargo.lock` 的修复继续保留，不改变既有运行时路径。后续修复预计只涉及 SCK 回调清理、macOS 精确窗口过滤及更新错误处理、排除列表快照、macOS 边框显示条件和 Windows 排除 API 的调用条件。无需修改连接授权、通用 IPC、公共采集接口或增加后端能力通知。
