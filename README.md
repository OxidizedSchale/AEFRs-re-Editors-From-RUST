# GNU's Not Unix : AEFR's Eternal Freedom & Rust-rendered

> Inspired by GNU, but not an official GNU project. Applied for the FSF Free Software Directory.
> <br>受 GNU 启发而来，不是官方的 GNU 项目，已申请“自由软件基金会自由软件目录”。

## Our Rust purity is 99.7% higher than the GNU/Linux kernel!
## 我们的 Rust 纯度比 GNU/Linux 内核高 99.7%!

**GNU:AEFR is a free software project embodying the spirit of the GNU Manifesto, dedicated to liberating the creative environment of Kivotos! Distributing software without open-sourcing it is an irresponsible act!**<br>**GNU:AEFR 是一个秉承 GNU 宣言精神的自由软件项目，致力于解放基沃托斯 (Kivotos) 的创作环境！分发软件而不开源，这是没有责任的行为！**

---

### ​⚖️ License / 许可证
This project is distributed under, and only under, the **GPL-3.0** License.<br>本项目采用且只采用 **GPL-3.0** 协议发布。

---

## 🧭 The Philosophy of GNU:AEFR / 导览：GNU:AEFR 的哲学

> GNU:AEFR is not currently a complex application designed to please everyone, as it is not yet a complete release. At present, it is merely a kernel built upon 1123 lines of bare-metal Rust logic. **In the world of computing, the shortest path is always the most invincible!** If you seek current compatibility and ease of use, please use AA; if you seek ultimate freedom, extreme performance, and community-driven maintenance, welcome to GNU:AEFR.
> 
> GNU:AEFR 目前并不是一个为了讨好所有人而设计的复杂应用，因为它仍未做出完整版本。它目前只是 1123 行直达底层的 Rust 逻辑所构成的一个内核。**在计算机的世界里，最短的路径永远是最无敌的！** 如果你追求的是目前的兼容性和易用，请去用 AA；如果你追求的是极致的自由与性能以及社区维护，欢迎来到 GNU:AEFR。

*   **Unofficial & Fan-Made / 非官方粉丝制作**: A high-performance, multi-platform, multi-threaded *Blue Archive* fan-creation editor crafted entirely in pure Rust.<br>一个使用纯 Rust 打造的，性能强劲的多平台、多线程《蔚蓝档案》二次创作编辑器。
*   **No Game Engine / 无游戏引擎**: We do not rely on Unity/Unreal; we drive the graphical interface directly using the lightweight `egui` library.<br>我们不依赖 Unity/Unreal，直接使用轻量级的 `egui` 库驱动图形界面。
*   **Cross-Platform Domination / 全平台制霸**: Natively supports GNU/Linux, Android, macOS, and Windows.<br>原生支持 GNU/Linux、Android、macOS 和 Windows。

### ✨ Current Features / 现已实现
- [x] Dynamically change scene backgrounds / 动态更换场景背景
- [x] Import and render up to 5 Spine skeletal animation files simultaneously / 同时导入并渲染 5 个 Spine 骨骼动画文件
- [x] Support standard Kivotos-style dialogue box rendering / 支持标准的基沃托斯风格对话框渲染
- [x] Switch skeletal animations (e.g., expressions, actions) in real-time / 实时切换骨骼动画（如表情、动作）
- [x] Asynchronously load and play Background Music (BGM) / 异步加载并播放背景音乐 (BGM)

### 🎯 Roadmap / 未来计划
- [ ] Linear editing system (Timeline) / 线性编辑系统（时间轴）
- [ ] Smooth transition and blending of character animations / 角色动作的平滑过渡与混合
- [ ] Pop-up images within scenes (e.g., illustrations) / 场景内的图片弹出（如插画）
- [ ] Scene transition effects (fade in/out, wipe, etc.) / 场景切换特效（淡入淡出、划变等）
- [ ] Character expression bubbles / 角色头顶的表情气泡

**We welcome any visionaries to join the development of GNU:AEFR!**<br>**我们欢迎任何有志之士参与 GNU:AEFR 的开发！**

---

## 🚀 Getting Started / 开始使用

> "Release? Real hackers compile from source." ;-)

Head to the [**Releases**](https://github.com/OxidizedSchale/GNU-AEFR/releases) page to download the source code, or grab the pre-compiled binaries for your platform.<br>前往 [**Releases**](https://github.com/OxidizedSchale/GNU-AEFR/releases) 页面下载源码，或直接获取为你的平台无指令集优化编译好的二进制文件。

GNU:AEFR utilizes an interaction model combining **Graphical User Interface (GUI)** and **Command-Driven** inputs.<br>GNU:AEFR 采用 **图形化** 与 **指令驱动 (Command-Driven)** 相结合的交互方式。

*   **Desktop / 桌面端**: Graphical interface is recommended.<br>推荐使用图形化界面。
*   **Mobile / 移动端**: Currently, file importing can only be done via commands.<br>在涉及文件导入时，目前只能使用指令。

Click the `[CMD]` button in the top-left corner of the interface to open the built-in debug console.<br>点击界面左上角的 `[CMD]` 按钮即可打开内置调试控制台（有图形化支持）。

---

## 📖 Command Reference / 指令参考手册

### 1. Visuals / 场景与视觉

*   **Load Background / 加载背景**
    *   **Command / 指令**: `BG <image_path>`
    *   **Desc / 说明**: Instantly switches the background image. Supports `.jpg`, `.png`, `.webp`.<br>瞬间切换背景图片，支持 `.jpg`, `.png`, `.webp`。
    *   **Example / 示例**: `BG C:\Assets\BlueArchive\BG_Classroom.png`

*   **Load Spine Character / 装填角色**
    *   **Command / 指令**: `LOAD <slot_ID> <.atlas_path>`
    *   **Desc / 说明**: Loads a character into slots `0`~`4`. Upon success, the console prints the available animation list.<br>将角色加载到 `0`~`4` 号共 5 个槽位，支持自动排版。加载成功后，控制台会打印出该角色可用的动作列表。
    *   **Example / 示例**: `LOAD 0 D:\Assets\Shiroko\Shiroko_Home.atlas`

### 2. Motion / 动作与演出

*   **Change Animation / 切换动作**
    *   **Command / 指令**: `ANIM <slot_ID> <animation_name> [loop: true/false]`
    *   **Desc / 说明**: `true` loops the animation, `false` plays it once. Names must match exactly.<br>`true` 为循环播放，`false` 为播放一次。动作名必须精确匹配（加载角色时控制台会列出可用动作）。
    *   **Example / 示例**:
        ```bash
        ANIM 0 Start_Idle_01 true    # 让白子开始循环待机动作
        ANIM 1 Attack_Normal false   # 让 1 号位角色攻击一次
        ```

### 3. Storytelling / 剧本与对话

*   **Show Dialogue / 发送对话**
    *   **Command / 指令**: `TALK <name>|<affiliation>|<content>`
    *   **Desc / 说明**: Renders a standard dialogue box with typewriter effects. **Parameters must be separated by a pipe `|`.**<br>渲染标准的基沃托斯风格对话框，支持打字机效果（点击对话框可瞬间跳过）。**必须使用竖线 `|` 分隔参数。**
    *   **Example / 示例**:
        ```bash
        TALK 砂狼白子|对策委员会|老师，我们要去抢银行吗？
        TALK 阿洛娜|什亭之箱|老师，请不要在工作时间摸鱼！
        ```

### 4. Audio / 音频系统

*   **Play BGM / 播放 BGM**
    *   **Command / 指令**: `BGM <audio_path>`
    *   **Desc / 说明**: Asynchronously loads and plays background music with seamless switching.<br>异步加载并播放背景音乐，支持无缝切换。
    *   **Example / 示例**: `BGM D:\Music\Unwelcome_School.mp3`

*   **Stop Music / 停止音乐**
    *   **Command / 指令**: `STOP`
    *   **Desc / 说明**: Immediately stops the currently playing BGM.<br>立即停止当前播放的 BGM。

---

### 💡 Pro Tips / 极客贴士

*   **Path Issues / 路径问题**: Windows paths can be pasted directly, AEFR handles quotes automatically; on Android/Termux, use absolute paths.<br>Windows 推荐直接复制文件路径，AEFR 会自动处理引号（如 `"C:\Path"`）；Android / Termux 请使用绝对路径，例如 `/sdcard/Download/bg.png`。
*   **Performance / 性能监控**: Thanks to the "Gentleman Scheduler," the UI thread remains silky smooth even fully loaded with 5 characters and BGM. Feel free to multitask boldly.<br>得益于“绅士调度器”，即使你填满了 5 个槽位并播放 BGM，UI 线程依然保持丝滑。请放心大胆地进行多任务操作。
*   **Logs / 日志反馈**: All operation results (success/failure/animations) are printed in real-time in the console Log area.<br>所有的操作结果（成功/失败/可用动画列表）都会实时打印在控制台的 Log 区域，请留意查看。

---

## 🤝 Contribution Guidelines / 贡献指南

<details>
<summary><strong>Click to expand: Please read these guidelines before contributing <br>点击展开：在提供帮助构建 GNU:AEFR 时，必须要遵守以下规则</strong></summary>

### Tech Stack Purity / 技术栈纯洁性
This project insists on implementing 100% of its core business logic and architecture in **Rust**.<br>本项目坚持核心业务逻辑与架构 100% 使用 **Rust** 实现。

*   **As a rule, we reject** any PRs that introduce complex FFI interactions with C++ runtimes or frameworks (e.g., Qt, Unity, Unreal).<br>**原则上拒绝** 引入任何需要与 C++ 运行时或框架（如 Qt、Unity、Unreal 等）进行复杂交互（JNI/复杂FFI）的 PR，以保持架构的纯粹性和可维护性。
*   **Exceptions**: Safe Rust wrappers around fully-functional low-level C system libraries (e.g., graphics, audio, filesystem) are allowed. In such cases:<br>**例外情况**: 允许为接入现有的、功能完整的、底层系统级 C 库（如图形、音频、文件系统基础库）而编写 Rust 安全封装层。在此情况下：
    *   Must prioritize mature community `-sys` bindings (e.g., `libc`, `openssl-sys`).<br>必须优先使用社区维护的、成熟的 `-sys` 绑定库。
    *   If writing `unsafe` FFI calls is necessary, you must strictly follow the `unsafe` code standards below and prove irreplaceability.<br>若需自行编写 `unsafe` 代码进行 FFI 调用，必须严格遵守下文的 `unsafe` 代码规范，并证明其不可替代性。
    *   The ultimate goal is to encapsulate all `unsafe` calls within safe Rust APIs, completely transparent to upper-level applications.<br>最终目标是将所有 `unsafe` 调用封装在安全的 Rust API 之内，对上层应用完全透明。

### `unsafe` Rules: The Blade of Performance Above Safety / unsafe 准则：安全之上的性能之刃
> `unsafe` is the "blade of performance" granted to developers by Rust beyond compile-time safety rules. The principle is: "Do not use unless necessary; when used, it must be foolproof."<br>`unsafe` 关键字是 Rust 赋予开发者在编译期安全规则之外，进行必要底层操作的能力。本项目视其为 “安全之上的性能之刃” ，使用原则是 “非必要不使用，使用时必须万无一失”。

**Core Principle: Proof of Necessity / 核心原则：必要性证明**

Any `unsafe` block must be based on a justified reason that cannot be achieved via safe Rust. You must provide a concise "Proof of Necessity" in the comments above the `unsafe` block, including:<br>任何 `unsafe` 代码块的存在，都必须基于一个无法通过安全 Rust 代码实现的正当理由。在提交的 PR 中，您必须在 `unsafe` 代码块上方，以注释形式提供一份简明的“必要性证明”，内容应至少包括：

1.  **Reason / 原因**: Why `unsafe` is absolutely necessary (e.g., calling specific C FFI functions, deterministic memory layout conversions).<br>明确指出为何必须使用 `unsafe`（例如：调用特定的 C FFI 函数、进行确定性的内存布局转换、实现某个自引用数据结构等）。
2.  **Irreplaceability / 不可替代性**: Argue why it cannot be achieved with safe Rust standard or community libraries.<br>论证为何无法用安全的 Rust 标准库或社区库实现相同功能。
3.  **Safety Boundaries / 安全边界**: Clearly define the invariants this `unsafe` block commits to maintaining. What do you "promise" the compiler?<br>清晰界定该 `unsafe` 块所承诺维护的不变量。即，作为开发者，您向编译器“承诺”了哪些条件必定成立，从而使得这段代码在逻辑上是安全的。

*Specific Requirements: Documented Comments / 具体要求：文档化注释*
*   Every `unsafe` function, method, or block must have preceding comments.<br>每个 `unsafe` 函数、方法或代码块都必须附有前置注释。

</details>

---

## 🏛️ Architectural Philosophy / 架构哲学

<details>
<summary><strong>Click to expand: Understand the hardcore low-level design driving GNU:AEFR <br>点击展开：了解驱动 GNU:AEFR 的硬核底层设计</strong></summary>

### The "Gentleman Scheduler": Class Segregation at the Compute Level / 绅士调度器：算力层面的阶级隔离
> In GNU:AEFR v0.8+, we introduced our proprietary "Gentleman Scheduler." We do not trust default OS schedulers, as they often sacrifice real-time rendering determinism for so-called "fairness."<br>在 GNU:AEFR v0.8+ 中，我们引入了自主研发的“绅士调度器”。我们不信任操作系统的默认调度，因为它们往往为了所谓的“公平”而牺牲了实时渲染的确定性。

*   **Core Principle: N-2 Strategy / 核心准则：N-2 策略**
    *   GNU:AEFR refuses to be a greedy CPU-devouring beast. The scheduler detects physical core count `N` and forcibly isolates `N-2` cores as a compute zone (Takes 1 on dual-core setups).<br>GNU:AEFR 拒绝做吞噬 CPU 的贪婪野兽。调度器会自动探测物理核心数 `N`，并强行隔离出 `N-2` 个核心作为计算专区（在只有两个核心/线程的情况下，AEFR会占一个）。
    *   **1 Core** reserved for UI/Render (Main Thread), ensuring absolute smoothness even during compute spikes.<br>**1 核心** 预留给 UI/Render (Main Thread)：确保在计算量暴涨时，渲染帧率依旧丝滑。
    *   **1 Core** reserved for OS/Audio (Backstage), preventing BGM popping from CPU overloads.<br>**1 核心** 预留给 OS/Audio (Backstage)：作为底层系统的“避震空间”，彻底杜绝背景音乐因 CPU 满载而产生的爆音与抖动。
    *   Remaining cores are assigned to GNU:AEFR Workers for Spine math via the `Rayon` thread pool.<br>其余核心分配给 GNU:AEFR Worker：利用 Rayon 线程池进行 Spine 骨骼蒙皮与物理计算。

*   **Why insist on a "Synchronous Blocking Model"? / 为何坚持“同步阻塞模型”？**
    > Do not pitch us the cheap "non-blocking async" concepts found in Web development. In the philosophy of GNU:AEFR, "Synchronous" means order.<br>不要向我们推销 Web 开发中那种廉价的“非阻塞异步”概念。在 GNU:AEFR 的哲学里，“同步”即是秩序。

    *   **No Screen Tearing / 拒绝画面撕裂**: The main thread synchronously awaits results during the Update phase to ensure every frame's visual elements align perfectly. (You don't want character art flying around, do you?)<br>主线程在 Update 阶段会同步等待计算结果。这是为了保证每一帧的骨骼位置、表情、物理效果在空间上完全对齐（你也不希望立绘乱飞吧）。
    *   **Work Stealing Mechanism / Work Stealing 机制**: On extreme low-core machines, the main thread actively participates in computation, squeezing out 100% of physical performance.<br>得益于 Rayon 的硬核实现，在核心较少的极端情况下（如双核机器），主线程会主动下场参与计算，确保算力利用率达到 100% 的物理极限。
    *   **Scheduler Warning / 调度器警告**: Using `std::thread::spawn` for compute-heavy tasks is forbidden. All parallel tasks must dispatch via `AefrScheduler`. Violations lead to chaotic OS scheduling and betray the Rust aesthetics of GNU:AEFR.<br>禁止在任何计算密集型任务中使用 `std::thread::spawn`。所有并行任务必须通过 `AefrScheduler` 进行分发，违者将导致系统调度陷入无序竞争，这是对 GNU:AEFR 的 Rust 美学进行背叛。

### GUI Rules / GUI 改进规则
*   UI components must strictly adhere to `egui`'s **Immediate Mode** philosophy; `React`-style proposals are not accepted.<br>UI 组件必须遵循 `egui` 的**即时模式**哲学，不接受来自 `React` 风格的提议。

</details>

---

## ✉️ Contact Us / 联系我们
*   **Author Email / 作者邮箱**: `3997101522@qq.com`
*   **QQ Group / QQ 群**: `1054276524`
