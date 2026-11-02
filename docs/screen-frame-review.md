原生屏幕边框：实现评审与修复方案

评审范围是 `border-notification-native` 相对本地 `master` 的功能改动，覆盖 macOS 和 Windows。原评审对应 `e94085a59`，基线为 `eab27cc24`。修复实施状态如下，每项单独提交。

| 独立改动 | 状态与验证 |
| --- | --- |
| SCK 回调生命周期 | 已修复；原生回归验证确认修复前销毁提前返回，修复后等待帧和错误回调结束，忽略后续回调；Objective-C 语法检查通过。 |
| 旧 macOS 本地边框 | 已移除显示入口的 SCK 能力限制；macOS `cargo check --locked --offline --lib --features flutter` 通过，旧系统实机待验证。 |
| 精确窗口过滤 | 已删除应用级兜底；临时过滤对象验证覆盖部分匹配、空列表及未匹配 ID，修复前失败、修复后通过；Objective-C 语法检查通过。 |
| 过滤更新失败处理 | 已改为受控日志后继续取帧，每代仅尝试一次；隔离故障注入验证帧可继续返回、停止错误仍传播，修复前失败、修复后通过；macOS 编译检查通过。 |
| 窗口 ID 与版本号快照 | 已保持同一锁直到取得两者；原逻辑并发验证出现错配，修复后 499999 次并发更新中读取 560254 次快照均一致；macOS 会话边框单测通过。 |
| Windows 旧系统排除属性 | 已仅对排除 API 增加 Windows 10 2004 能力门槛；Windows 编译、会话边框单测及双屏 CM 进程验证通过，8 个窗口的排除属性均为 `0x11`；旧系统降级仍待实机验证。 |
| macOS 原生源码增量构建 | 已补充 `sck_capture.m` 的变更追踪；更新时间戳后 Cargo 重新编译并更新 `.o` / `.a`，macOS 编译检查及会话边框单测通过。 |
| macOS 后台更新排除名单 | 已改为独立串行队列；故障注入确认等待及超时期间帧仍可交付，调用方立即返回，销毁期间的延迟工作通过 AddressSanitizer 验证。 |

**完整复评后的修复（2026-11-02）。** 以下状态更新后文复评时的“尚未修改实现”，每个独立问题单独提交。

| 复评问题 | 修复与验证 |
| --- | --- |
| 会话状态提交竞态 | 已修复。macOS 在主队列读取当前会话状态再应用，Windows 持有既有读锁直到提交显示请求，不增加状态。隔离交错程序直接提取修复前后函数：两个平台分支修复前均得到“有效会话但边框隐藏”，修复后均保持显示；macOS 使用真实主队列，Windows 分支在本机验证同步逻辑，不等同于 Windows 实机测试。macOS 既有会话边框单测通过。 |
| 停止采集诊断及错误日志限流 | 已补充停止失败、10 秒停止超时及移除输出失败的诊断。通过不依赖采集 context 的原生到 Rust 日志入口调用 `hbb_common::throttled_log!`；本功能中可能反复触发的两端窗口错误、SCK 初始化回退日志也按原有级别每处 5 秒限流，Windows 先保存错误码再进入限流宏。ASan 原生故障注入覆盖正常停止、停止及移除失败、超时和迟到回调，全部通过；实际 Rust 日志入口连续接收 100 次错误只输出 1 条，间隔后输出包含 `(x100)` 的汇总。macOS 会话单测、SCK 冒烟测试及严格原生编译通过。 |

当前分支没有红色边框配置开关；符合条件的会话会自动显示边框。macOS 采集后端也没有按边框选项切换：系统支持时优先使用 ScreenCaptureKit，不支持或初始化失败才回退 CGDisplayStream。即使排除列表为空，也优先使用 SCK。本轮不新增开关或改变后端选择策略。

现有 `--screencapturekit` 编译选项对应根包 `screencapturekit = ["cpal/screencapturekit"]`，控制的是系统音频采集。它不控制 `scrap` 的视频后端；未启用该编译选项时，macOS 视频仍然优先尝试新的 SCK。当前 macOS 测试命令只启用了 `flutter`，没有启用这个音频 feature。

**完整复评（2026-11-02，代码截至 `11e680979`）。** 当前分支没有关联 PR，因此以本地 `master`（`eab27cc24`）到 HEAD 的完整 diff 核对归属，覆盖全部 15 个变更文件及授权、断开、原生窗口、采集后端、过滤更新和销毁的调用链。以下均来自本分支新增或修改的代码；没有把原有问题或已接受的回退行为列为缺陷。本轮发现 1 个运行时问题，以及 3 项低优先级的诊断、最小化和 CI 问题，尚未修改实现。

1. **[P2] macOS / Windows：旧会话状态可能覆盖新状态，导致正在远控却没有本地边框。** [src/ui_cm_interface.rs](../src/ui_cm_interface.rs) 第 417–423 行先在 `CLIENTS` 读锁内计算 `visible`，释放锁后才提交给平台。Flutter 授权入口和 CM IPC 清理运行在不同线程，允许以下交错：A 断开，IPC 线程算出 `false` 后暂停；用户授权等待中的 B，授权线程计算并提交 `true`；旧 IPC 调用恢复并提交 `false`。此时 B 仍然是有效桌面会话，两端平台却都保存了隐藏请求。没有后续会话状态变化时，这个结果可持续存在；反向交错也可能留下多余边框。

   已用隔离程序直接提取当前 `update_screen_frame()` 和会话判断函数，在平台提交入口控制线程调度，得到 `active desktop session: true; final requested frame visibility: false`。这证明状态提交存在竞态，不等于已在真实远控 UI 中复现。原有单测只检查会话筛选条件，无法发现这个时序。

   建议串行执行“读取当前会话状态并提交显示”，不新增版本号或重试状态：macOS 可以在现有主队列中重新读取状态并应用；Windows 可以让现有读锁覆盖只写原子状态和通知窗口线程的平台调用。macOS 不能简单把 `CLIENTS` 读锁持有到 `exec_sync` 返回，因为主线程更新窗口后还会读取 `CLIENTS` 广播 ID，等待中的写线程可能使它们相互等待。保留 CM 判断，无需迁移到服务端。

2. **[P3] macOS：停止采集失败和超时被静默丢弃。** [libs/scrap/src/quartz/sck_capture.m](../libs/scrap/src/quartz/sck_capture.m) 第 337–348 行忽略停止回调的 `NSError`、`ScrapSckWait()` 的返回值，以及移除输出的返回值和 `removeError`。因此停止操作即使等待 10 秒超时，也没有对应诊断；这也遗漏了前文修复方案中“停止超时及移除输出失败必须记录”的要求。

   当前销毁代码已先在同步保护下注销 Rust 回调和 context，本项不能据此推断存在崩溃或已释放内存访问。建议只补充受控的停止、超时和移除输出错误记录，保持现有销毁顺序；不要为了记录错误重新调用已经注销的 Rust context。

3. **[P3] macOS：旧 CGDisplayStream 路径包含与功能无关的行为修改。** [libs/scrap/src/common/quartz.rs](../libs/scrap/src/common/quartz.rs) 第 70–73、88–90 行将旧回调的 `if let Ok(mut f) = f.lock()` 改为恢复 poisoned mutex 后继续写帧。SCK 不可用的旧系统也会走这个修改后的路径。正常情况下行为相同，未观察到实际故障，但改变锁中毒策略并不是接入新采集器所需，违反保留旧路径、最小化回归面的要求。

   建议仅恢复两个 CGDisplayStream 分支原有的锁处理方式，不抽取新的共享回调，不借此重构采集接口。

4. **[P3] 新增 SCK 冒烟测试没有接入 macOS CI。** [libs/scrap/src/quartz/sck.rs](../libs/scrap/src/quartz/sck.rs) 第 218–231 行的测试属于 `scrap` 的 macOS 模块；当前 [flutter-build.yml](../.github/workflows/flutter-build.yml) 的 macOS 作业只运行根包 `cargo test --lib` 及 `-p clipboard`。根包依赖 `scrap` 不会执行其单测，Linux 的 workspace 测试也不会编译 quartz 模块，因此此测试没有持续验证入口。

   本机显式执行 `cargo test --locked --offline -p scrap quartz::sck::tests::screen_capture_kit_availability_can_be_queried -- --exact`，1 项通过。若保留该测试，建议在现有 macOS 测试步骤增加对应的 `-p scrap` 命令，并沿用构建特性；无需另建测试框架。该测试只覆盖能力查询及 FFI 链接，不能代替真实采集或并发回归验证。

**本轮验证与限制。** macOS 会话边框单测 1 项通过；上述 `scrap` 测试 1 项通过；当前 Objective-C 文件的 `clang -fobjc-arc -fblocks -Werror -Wno-unguarded-availability-new -fsyntax-only` 通过；`git diff --check` 通过。并发探针位于本机忽略目录 `target/screen-frame-full-review/`，未向生产代码加入测试钩子。本轮未重跑 Windows 编译或双向真实远控，也未重跑前轮的 AddressSanitizer / ThreadSanitizer 故障注入；下文既有实机记录仍是此前验证结果，未测 TODO 仍未勾选。

**回归面最小化检查。** 本轮只更新本文档，无运行时行为变化。对整个分支中被修改的既有文件及路径逐项核对如下：

| 既有文件 | 改变的路径及必要性 |
| --- | --- |
| `src/ui_cm_interface.rs` | macOS / Windows 会话增加、授权、移除及 CM 退出时驱动边框；macOS 手动授权先更新边框、再发送授权消息。复用 CM 状态是当前设计所需，但状态提交竞态见第 1 项。 |
| `src/ipc.rs`、`src/server/connection.rs` | 仅 macOS 增加边框窗口 ID 消息及采集排除列表更新；跨进程精确排除所需。没有增加采集能力握手。 |
| `src/platform/macos.rs`、`src/platform/windows.rs` | 接入各自新增的原生边框模块，属于必要的平台入口。 |
| `libs/scrap/src/common/quartz.rs` | macOS 新采集器选择、初始化失败回退、取帧时提交异步过滤更新；为排除原生窗口所需。两个旧后端回调的锁中毒处理不必要，见第 3 项。Windows 原有采集路径未改。 |
| `libs/scrap/src/quartz/mod.rs` | 接入 SCK 模块并保存窗口 ID 与一致版本快照；只供 macOS 采集使用。 |
| `libs/scrap/build.rs`、`libs/scrap/Cargo.toml`、`Cargo.lock` | 编译及链接 macOS 原生桥接、追踪源文件变更和记录 `cc` 构建依赖；属于构建所需，锁文件已同步。 |

**双向真实远控验证（2026-10-31，分支 `4e73ed3be`，最新运行时代码 `609143911`）。** macOS 14.5（Apple Silicon，一块 3840×2160、2 倍缩放屏幕）与 Windows build 26200.9711（两块 1920×1080 屏幕）通过局域网 TCP 直连，分别作为主控和被控，使用真实连接请求和 CM 手动授权。两端均重新执行 `cargo build --locked --offline --lib --features flutter`，没有发送模拟会话 IPC。

测试复用现有 Flutter Debug 外壳与资源，替换当前分支的 Rust 核心库。macOS 外壳原有绝对链接指向另一工作目录的旧库，已在临时测试包中改为包内路径并重新签名；修正前的反向连接不计入结果。最终以运行进程的库映射和 Mach-O UUID 核对 macOS 产物，以进程模块路径和 SHA-256 核对 Windows 产物，确认客户端、服务端和 CM 加载当前核心库。

| 真实会话检查 | macOS 主控 → Windows 被控 | Windows 主控 → macOS 被控 |
| --- | --- | --- |
| 视频与输入 | 测试时钟持续变化；远程点击文本框并输入 `MAC_TO_WINDOWS_PASS`，被控端记录收到；切换到第二块屏幕后正常显示其桌面，客户端建立 display #1 解码器。 | 测试时钟持续变化；远程点击文本框并输入 `WINDOWS_TO_MAC_PASS`，被控端记录收到。 |
| 本地边框 | 两块屏幕共 8 个可见原生窗口，位置覆盖四边，采集排除属性均为 `0x11`。 | 本地截图可见完整红框；4 个原生窗口处于层级 1000，厚度 3 点，即 6 个物理像素。 |
| 主控画面 | 两块屏幕的远程画面均未见红框。 | 远程画面未见红框，普通桌面窗口正常显示；日志没有 SCK 初始化回退、过滤更新失败或流错误。 |
| 断开与重连 | 边框数量 `8 → 0 → 8 → 0`，重连后重新收到视频。 | 边框数量 `4 → 0 → 4 → 0`，窗口 ID 从 `1822–1825` 换为 `1829–1832`，重连后视频及排除仍正常。 |

最终截图、窗口枚举及输入结果保存在本机忽略目录 `target/screen-frame-e2e/`，汇总为 `result.json`；Windows 同名目录保留操作与恢复记录。本轮没有发现需要新增运行时代码修复的问题，只补充本文档，既有运行时路径未改动。

测试结束后两端 `RustDesk2.toml` 已从备份恢复并核对一致，测试进程、边框和临时计划任务已清理。Windows 原安装版服务（Automatic / Running）、`--server`、主界面和托盘均已恢复，没有卸载或替换安装目录文件；远端原有工作区改动继续保留。

验证范围是正常交互桌面的默认采集路径及断开重连；未覆盖旧系统、锁屏、安全桌面、显示器热插拔、全屏 Space、强制 GDI / CGDisplayStream 回退或长时间稳定性。上一轮 10–20 秒等待场景仍由故障注入验证，本轮未向真实远控注入系统超时，不能据此声称这些边界已全部通过。以下较早记录中的“未完成真实远控”由本段更新，其他未测边界继续保留。

**待验证 TODO：真实远控边界场景。** 以下项目均待执行；完成后记录系统版本、代码提交、实际采集后端、操作次数、截图或日志及结果，再勾选。测试中同时观察被控端边框和主控端画面，使用动态时钟与专用输入框验证视频、鼠标和键盘；结束后恢复临时设置及原进程状态。

- [ ] **旧 Windows。** 在客户端仍支持、且低于 build 19041 的系统上建立远控，检查跳过排除 API 后本地仍显示正常红框、远端没有因该属性产生的黑边，视频及输入正常；断开后清理，重连后恢复。允许主控看到红框。需要对应系统的测试机或虚拟机。
- [ ] **旧 macOS。** 在客户端仍支持、且低于 12.3 的系统上建立远控，确认使用 CGDisplayStream，本地仍显示边框，视频、输入及断开重连正常。允许主控看到红框。需要对应系统的测试机或虚拟机。
- [ ] **Windows 热插拔。** 保持远控，接入第二块屏幕、拔掉正在观看的屏幕，再插回，重复多轮并记录次数。检查边框按实际屏幕数量及位置重建、旧窗口清理、主控能继续观看和切换屏幕，记录最长停帧时间。物理拔插需要现场操作，软件禁用显示器只能作为补测。
- [ ] **macOS 热插拔。** 执行同样的接入、拔出、插回操作，额外核对边框窗口 ID 更新和采集排除效果。区分采集器重建造成的中断与排除名单更新造成的等待；不能因后者拖住正常出帧。物理拔插需要现场操作。
- [ ] **Windows 锁屏及安全桌面。** 使用当前版本的服务模式，测试连接中锁屏再解锁、锁屏时断开后解锁重连，以及触发 UAC 后返回普通桌面。解锁或返回后视频、输入和边框应恢复，断开后无边框残留。安全桌面本身能否显示边框单独记录并与 master 对照；普通用户启动的测试进程不能代替服务模式验证。
- [ ] **macOS 锁屏。** 测试连接中锁屏再解锁，以及锁屏时断开、解锁后重连。确认解锁后视频、输入和边框正常，断开后无残留；锁屏期间的表现单独记录，并与 master 对照。
- [ ] **Windows 强制 GDI。** 关闭现有“使用 DirectX 捕获屏幕”选项，重新连接，通过日志确认 `gdi: true`，再验证视频、输入、边框及断开重连。GDI 不必然采集到红框；无法排除时允许出现正常红框，不能因此隐藏本地提示或停止采集。
- [ ] **macOS：SCK 不可用。** 使用隔离的临时测试构建模拟 SCK 能力检查返回不可用，确认进入 CGDisplayStream。保留屏幕录制权限，验证本地边框、视频、输入及断开重连；远端允许看到红框。当前没有强制切换配置项，临时注入不纳入生产提交。
- [ ] **macOS：SCK 初始化失败。** 使用临时测试构建保留能力检查成功、让 SCK 初始化返回失败，以回退日志及实际画面确认进入 CGDisplayStream。通过标准同上一项，结束后还原临时注入。
- [ ] **Windows 排除 API 失败。** 在支持该 API 的系统上，通过临时测试构建注入 `SetWindowDisplayAffinity` 失败，验证本地边框仍完整显示，远端允许看到红框，正常视频和输入不受影响，断开后清理。
- [ ] **macOS 排除更新失败。** 在已开始出帧的真实会话中注入过滤更新失败并触发窗口 ID 变化。检查本地边框保留、普通应用窗口没有被整体排除、视频及输入继续工作，日志受控且不会每帧重复尝试同一代名单；远端允许看到尚未排除的红框。
- [ ] **Windows 全屏。** 在被控端让应用进入、退出全屏，检查边框覆盖正确屏幕、不抢焦点、不挡点击，退出全屏及断开后没有重复或残留窗口；记录实际应用及其全屏模式。
- [ ] **macOS 全屏及 Space。** 在被控端切换普通桌面与全屏 Space，并进入、退出应用全屏。检查边框覆盖正确屏幕、不抢焦点、不挡点击，返回桌面及断开后没有重复或残留窗口。
- [ ] **双向长时间稳定性。** 两个方向各先运行 1～2 小时动态画面，期间反复连接、断开及切换屏幕。记录实际时长、操作次数、停帧及错误，比较多轮操作后稳定状态的内存、线程和窗口数量，检查是否持续增长或出现崩溃、持续黑屏、边框残留。
- [ ] **macOS 后台更新延迟：真实会话。** 在已经出帧的会话中，分别延迟内容枚举和过滤更新回调，覆盖 10～20 秒等待、超时及更新完成场景，再触发排除名单变化。等待期间主控时钟应继续更新、远程输入应继续响应；超时后仍能传画面，等待期间断开也不能崩溃。只延迟排除名单更新，不把初始建流等待算入本项；记录最长停帧时间及日志。此前隔离故障注入已通过，此项用于补齐真实会话验证。

**继续评审（2026-10-31，代码截至 `a3479fb33`）。** 相对 `master` 核对新增代码及上述六个修复，发现一项需要补齐的构建问题。

**[P2] 原生源码修改不会触发增量重编译。** [libs/scrap/build.rs](../libs/scrap/build.rs) 第 236–242 行使用 `cc` 编译 `src/quartz/sck_capture.m`，但没有输出对应的 `cargo:rerun-if-changed`。当前构建指纹只包含环境变量追踪，`cc` 不会替调用者注册这个源文件。因此修改 Objective-C 实现后，增量构建仍可能链接旧的原生库，使修复没有进入本地产物；干净构建不受影响。

已在当前工作区复现：先完成 `cargo check --locked --offline --lib --features flutter -vv`，再仅更新时间戳并重复检查，Cargo 输出 `Fresh scrap`，所有 `sck_capture.o` / `libscrap_sck_capture.a` 的修改时间保持不变。验证后已恢复源文件时间戳，未修改源码内容。本轮检查成功不能作为最新 Objective-C 源码已经参与 Cargo 构建的证据。

建议在 macOS 编译分支增加一行：

```rust
println!("cargo:rerun-if-changed=src/quartz/sck_capture.m");
```

此项已按上述一行改动修复。重复增量验证后，Cargo 明确报告 `sck_capture.m` 变化，并重新生成原生 `.o` / `.a`；`cargo check --locked --offline --lib --features flutter` 及会话边框单测通过。影响范围仅为 macOS 构建脚本的重跑条件，不改变运行时行为。变更追踪规则参见 [Cargo Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html#change-detection)。

**排除名单更新造成的等待已修复。** 原先 [common/quartz.rs](../libs/scrap/src/common/quartz.rs) 的 `frame()` 同步调用过滤更新，内容枚举和过滤更新各可能等待 10 秒。现在 [sck_capture.m](../libs/scrap/src/quartz/sck_capture.m) 复制窗口 ID 并投递到独立串行队列后立即返回，等待只发生在后台，不占用取帧线程或帧回调队列。更新期间继续采集，允许主控看到尚未排除的边框；失败保留现有过滤器，每代仍只尝试一次。

后台更新失败通过 [sck.rs](../libs/scrap/src/quartz/sck.rs) 的非致命错误分支输出受控日志；只有真实的流停止错误才设置 `stream_error()`。销毁时先在现有同步保护下断开回调、清空活动流，再停止流；已排队工作持有原生对象及自己的 ID 副本，不能再访问已释放的 Rust context，也不会在停止后提交新的过滤器。代码注释说明了不能阻塞出帧、异步参数所有权和回调生命周期的原因。

临时原生回归程序直接编译当前 Objective-C 源码并注入系统回调延迟：修复前调用方阻塞 10.006 秒，修复后约 15 微秒返回；内容枚举等待、过滤更新等待及 10 秒超时之后均能继续交付帧，调用方复用 ID 缓冲区不影响后台请求。分别在枚举和过滤更新未完成时销毁，AddressSanitizer 检查通过，延迟完成不访问释放后的 context。另用包含实际 `sck.rs` 的隔离验证确认非致命错误后 `stream_error()` 仍为空，真实流错误仍可读取。macOS `cargo check --locked --offline --lib --features flutter` 和会话边框单测通过。

本项生产代码仅修改 `sck_capture.m` 的更新调度及相关销毁同步、`sck.rs` 的错误回调分类。初始建流仍走原来的同步初始化，未改变 CGDisplayStream 回退、Windows 路径、会话判断或 IPC。尚未进行真实 macOS 远控会话的显示器热插拔长稳测试。

**后台更新复查（代码 `609143911`）。** 核对了参数所有权、回调 ABI、流访问同步和销毁顺序，未发现新的必须修复问题。临时原生验证补充连续提交多个名单、空名单清除，以及销毁时另有 8 个请求排队的场景；已覆盖场景的 AddressSanitizer 和 ThreadSanitizer 检查均通过，后台等待及超时期间帧仍能交付，停止后排队请求不再提交过滤器。以上仍是受控故障注入，真实 macOS 远控及显示器热插拔验证的缺口保留。本次仅追加评审记录。

本轮 Windows 静态复查未发现新的明确问题；当前环境没有 Windows 编译目标，也未完成 Windows 实机、多屏和全屏验证。macOS 编译检查通过，以上临时原生验证直接编译当前 Objective-C 文件，不依赖 Cargo 缓存。本轮只更新评审文档，不改变运行时路径。

**Windows 进程验证（2026-10-31，运行时代码 `9de9d4782`）。** 已通过 SSH 将 `192.168.10.117` 的 `E:/rustdesk/rustdesk` 切换至 `border-notification-native`，保留原有未跟踪文件并对齐子模块。使用已安装的 Rust 1.93 工具链完成 `cargo test --locked --lib --features flutter ui_cm_interface::tests::screen_frame_tracks_only_active_screen_sessions -- --exact`（1 项通过）及 `cargo build --locked --offline --lib --features flutter`。

该机忽略的 `src/bridge_generated.rs` 仍按旧接口包装返回值；备份后手工对齐了 5 处已经返回 `Result` 的调用，未运行 codegen，也未将这些生成文件纳入提交。进程验证复用该机现有 Flutter Debug runner 和资源，加载本分支新构建的 `librustdesk.dll`，复制前后 SHA-256 一致。该机现有本地依赖覆盖还使 `flutter/pubspec.lock` 的 `desktop_multi_window` 指向 `../../multi-window-pr38`；此环境差异保留，未纳入提交。

在 `sun` 的交互桌面会话中启动同一测试目录的 `--server` 和 `--cm`，核对 CM 命名管道的进程 ID 后发送模拟会话 IPC。实际有两台 1920×1080 显示器，验证结果如下：

| 场景 | 边框窗口数 |
| --- | --- |
| 未授权桌面、仅已授权文件传输 | 均为 0 |
| 一个或两个已授权桌面会话 | 均为 8 |
| 关闭一个桌面会话，另一个仍在 | 8 |
| 关闭最后一个桌面会话 | 0 |
| 重新连接桌面会话，再断开 | 8 → 0 |

8 个窗口均可见，位置覆盖两台显示器的四条边，厚度为 6 像素；透明度为 191，具有置顶、不激活、分层及鼠标穿透样式，`WM_NCHITTEST` 返回 `HTTRANSPARENT`，`WM_MOUSEACTIVATE` 返回 `MA_NOACTIVATE`，采集排除属性均为 `WDA_EXCLUDEFROMCAPTURE (0x11)`。

测试目录与安装目录不同，现有主 IPC 路径校验会拒绝混用两者，因此验证时短暂停止原服务，结束后已恢复安装版 `RustDesk` 服务及 `--server`；测试进程和临时计划任务已清理，没有卸载或替换安装版文件。报告保存在 Windows 的 `target/screen-frame-review/process-result.json`，测试用程序保存在同目录的 `app/`。本次覆盖真实 CM 进程及窗口行为，未建立真实远控视频会话，尚未验证 DXGI/GDI 的最终远端画面、旧 Windows、全屏切换或显示器热插拔。

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
