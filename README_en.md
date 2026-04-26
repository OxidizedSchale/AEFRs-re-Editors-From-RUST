[简体中文](./README.md) | [日本語](./README_jp.md) | [한국어](./README_kr.md)

# GNU's Not Unix : AEFR's Eternal Freedom & Rust-rendered

> Inspired by GNU, but not an official GNU project. Applied for the FSF Free Software Directory.

## Our Rust purity is 99.7% higher than the GNU/Linux kernel!

**GNU:AEFR is a free software project embodying the spirit of the GNU Manifesto, dedicated to liberating the creative environment of Kivotos! Distributing software without open-sourcing it is an irresponsible act!**

---

### ​⚖️ License
This project is distributed under, and only under, the **GPL-3.0** License（Under version 0.8.3).

For versions 0.8.3 and later (including it), only the **AGPL-3.0** license is allowed.

---

## 🧭 The Philosophy of GNU:AEFR

> GNU:AEFR is not currently a complex application designed to please everyone, as it is not yet a complete release. At present, it is merely a kernel built upon 1123 lines of bare-metal Rust logic. **In the world of computing, the shortest path is always the most invincible!** If you seek current compatibility and ease of use, please use AA; if you seek ultimate freedom, extreme performance, and community-driven maintenance, welcome to GNU:AEFR.

*   **Unofficial & Fan-Made**: A high-performance, multi-platform, multi-threaded *Blue Archive* fan-creation editor crafted entirely in pure Rust.
*   **No Game Engine**: We do not rely on Unity/Unreal; we drive the graphical interface directly using the lightweight `egui` library.
*   **Cross-Platform Domination**: Natively supports GNU/Linux, Android, macOS, and Windows.

### ✨ Current Features
- [x] Dynamically change scene backgrounds
- [x] Import and render up to 5 Spine skeletal animation files simultaneously
- [x] Support standard Kivotos-style dialogue box rendering
- [x] Switch skeletal animations (e.g., expressions, actions) in real-time
- [x] Asynchronously load and play Background Music (BGM)

### 🎯 Roadmap
- [ ] Linear editing system (Timeline)
- [ ] Smooth transition and blending of character animations
- [ ] Pop-up images within scenes (e.g., illustrations)
- [ ] Scene transition effects (fade in/out, wipe, etc.)
- [ ] Character expression bubbles

**We welcome any visionaries to join the development of GNU:AEFR!**

---

## 🚀 Getting Started

> "Release? Real hackers compile from source." ;-)

Head to the [**Releases**](https://github.com/OxidizedSchale/GNU-AEFR/releases) page to download the source code, or grab the pre-compiled binaries for your platform.

GNU:AEFR utilizes an interaction model combining **Graphical User Interface (GUI)** and **Command-Driven** inputs.

*   **Desktop**: Graphical interface is recommended.
*   **Mobile**: Currently, file importing can only be done via commands.

Click the `[CMD]` button in the top-left corner of the interface to open the built-in debug console.

---

## 📖 Command Reference

### 1. Visuals

*   **Load Background**
    *   **Command**: `BG <image_path>`
    *   **Desc**: Instantly switches the background image. Supports `.jpg`, `.png`, `.webp`.
    *   **Example**: `BG C:\Assets\BlueArchive\BG_Classroom.png`

*   **Load Spine Character**
    *   **Command**: `LOAD <slot_ID> <.atlas_path>`
    *   **Desc**: Loads a character into slots `0` to `4`. Upon success, the console prints the available animation list.
    *   **Example**: `LOAD 0 D:\Assets\Shiroko\Shiroko_Home.atlas`

### 2. Motion

*   **Change Animation**
    *   **Command**: `ANIM <slot_ID> <animation_name> [loop: true/false]`
    *   **Desc**: `true` loops the animation, `false` plays it once. Names must match exactly.
    *   **Example**:
        ```bash
        ANIM 0 Start_Idle_01 true    # Let Shiroko start her looping idle animation
        ANIM 1 Attack_Normal false   # Have the character in slot 1 attack once
        ```

### 3. Storytelling

*   **Show Dialogue**
    *   **Command**: `TALK <name>|<affiliation>|<content>`
    *   **Desc**: Renders a standard dialogue box with typewriter effects. **Parameters must be separated by a pipe `|`.**
    *   **Example**:
        ```bash
        TALK Sunaookami Shiroko|Countermeasures Council|Sensei, should we go rob a bank?
        TALK Arona|Schale|Sensei, please don't slack off during work hours!
        ```

### 4. Audio

*   **Play BGM**
    *   **Command**: `BGM <audio_path>`
    *   **Desc**: Asynchronously loads and plays background music with seamless switching.
    *   **Example**: `BGM D:\Music\Unwelcome_School.mp3`

*   **Stop Music**
    *   **Command**: `STOP`
    *   **Desc**: Immediately stops the currently playing BGM.

---

### 💡 Pro Tips

*   **Path Issues**: Windows paths can be pasted directly, AEFR handles quotes automatically; on Android/Termux, use absolute paths.
*   **Performance**: Thanks to the "Gentleman Scheduler," the UI thread remains silky smooth even fully loaded with 5 characters and BGM. Feel free to multitask boldly.
*   **Logs**: All operation results (success/failure/animations) are printed in real-time in the console Log area.

---

## 🤝 Contribution Guidelines

<details>
<summary><strong>Click to expand: Please read these guidelines before contributing</strong></summary>

### Tech Stack Purity
This project insists on implementing 100% of its core business logic and architecture in **Rust**.

*   **As a rule, we reject** any PRs that introduce complex FFI interactions with C++ runtimes or frameworks (e.g., Qt, Unity, Unreal).
*   **Exceptions**: Safe Rust wrappers around fully-functional low-level C system libraries (e.g., graphics, audio, filesystem) are allowed. In such cases:
    *   Must prioritize mature community `-sys` bindings (e.g., `libc`, `openssl-sys`).
    *   If writing `unsafe` FFI calls is necessary, you must strictly follow the `unsafe` code standards below and prove irreplaceability.
    *   The ultimate goal is to encapsulate all `unsafe` calls within safe Rust APIs, completely transparent to upper-level applications.

### `unsafe` Rules: The Blade of Performance Above Safety
> `unsafe` is the "blade of performance" granted to developers by Rust beyond compile-time safety rules. The principle is: "Do not use unless necessary; when used, it must be foolproof."

**Core Principle: Proof of Necessity**

Any `unsafe` block must be based on a justified reason that cannot be achieved via safe Rust. You must provide a concise "Proof of Necessity" in the comments above the `unsafe` block, including:

1.  **Reason**: Why `unsafe` is absolutely necessary (e.g., calling specific C FFI functions, deterministic memory layout conversions).
2.  **Irreplaceability**: Argue why it cannot be achieved with safe Rust standard or community libraries.
3.  **Safety Boundaries**: Clearly define the invariants this `unsafe` block commits to maintaining. What do you "promise" the compiler?

*Specific Requirements: Documented Comments*
*   Every `unsafe` function, method, or block must have preceding comments.

</details>

---

## 🏛️ Architectural Philosophy

<details>
<summary><strong>Click to expand: Understand the hardcore low-level design driving GNU:AEFR</strong></summary>

### The "Gentleman Scheduler": Class Segregation at the Compute Level
> In GNU:AEFR v0.8+, we introduced our proprietary "Gentleman Scheduler." We do not trust default OS schedulers, as they often sacrifice real-time rendering determinism for so-called "fairness."

*   **Core Principle: N-2 Strategy**
    *   GNU:AEFR refuses to be a greedy CPU-devouring beast. The scheduler detects physical core count `N` and forcibly isolates `N-2` cores as a compute zone (Takes 1 on dual-core setups).
    *   **1 Core** reserved for UI/Render (Main Thread), ensuring absolute smoothness even during compute spikes.
    *   **1 Core** reserved for OS/Audio (Backstage), preventing BGM popping from CPU overloads.
    *   Remaining cores are assigned to GNU:AEFR Workers for Spine math via the `Rayon` thread pool.

*   **Why insist on a "Synchronous Blocking Model"?**
    > Do not pitch us the cheap "non-blocking async" concepts found in Web development. In the philosophy of GNU:AEFR, "Synchronous" means order.

    *   **No Screen Tearing**: The main thread synchronously awaits results during the Update phase to ensure every frame's visual elements align perfectly. (You don't want character art flying around, do you?)
    *   **Work Stealing Mechanism**: On extreme low-core machines, the main thread actively participates in computation, squeezing out 100% of physical performance.
    *   **Scheduler Warning**: Using `std::thread::spawn` for compute-heavy tasks is forbidden. All parallel tasks must dispatch via `AefrScheduler`. Violations lead to chaotic OS scheduling and betray the Rust aesthetics of GNU:AEFR.

### GUI Rules
*   UI components must strictly adhere to `egui`'s **Immediate Mode** philosophy; `React`-style proposals are not accepted.

</details>

---

## ✉️ Contact Us
*   **Author Email**: `3997101522@qq.com`
*   **QQ Group**: `1054276524`
