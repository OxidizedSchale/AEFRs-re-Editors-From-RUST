# GNU's Not Unix : AEFR's Eternal Freedom & Rust-rendered

English Version | [中文版](./README.md)

> Inspired by GNU, but not an official GNU project. Applied for the FSF Free Software Directory.

## Our Rust purity is 99.7% higher than the GNU/Linux kernel!

**GNU:AEFR is a free software project embodying the spirit of the GNU Manifesto, dedicated to liberating the creative environment of Kivotos! Distributing software without open-sourcing it is an irresponsible act!**

---

### ​⚖️ License
This project is distributed under, and only under, the **GPL-3.0** License (Before version 0.8.3).

For versions 0.8.3 and later, only the **AGPL-3.0** license is allowed.

---

## 🧭 The Philosophy of GNU:AEFR

> GNU:AEFR is not currently a complex application designed to please everyone. It is merely a kernel built upon 1123 lines of bare-metal Rust logic. **In the world of computing, the shortest path is always the most invincible!** If you seek current compatibility and ease of use, please use AA; if you seek ultimate freedom, extreme performance, and community-driven maintenance, welcome to GNU:AEFR.

*   **Unofficial & Fan-Made**: A high-performance, multi-platform, multi-threaded *Blue Archive* fan-creation editor crafted entirely in pure Rust.
*   **No Game Engine**: We do not rely on Unity/Unreal; we drive the graphical interface directly using the lightweight `egui` library.
*   **Cross-Platform Domination**: Natively supports GNU/Linux, Android, macOS, and Windows.

### ✨ Current Features
- [x] Dynamically change scene backgrounds
- [x] Import and render up to 5 Spine skeletal animation files simultaneously
- [x] Support standard Kivotos-style dialogue box rendering
- [x] Switch skeletal animations in real-time
- [x] Asynchronously load and play Background Music (BGM)

### 🎯 Roadmap
- [ ] Linear editing system (Timeline)
- [ ] Smooth transition and blending of character animations
- [ ] Pop-up images within scenes (e.g., illustrations)
- [ ] Scene transition effects (fade, wipe, etc.)
- [ ] Character expression bubbles

**We welcome any visionaries to join the development of GNU:AEFR!**

---

## 🚀 Getting Started

> "Release? Real hackers compile from source." ;-)

Head to the [**Releases**](https://github.com/OxidizedSchale/GNU-AEFR/releases) page to download the source code or pre-compiled binaries.

GNU:AEFR utilizes an interaction model combining **Graphical User Interface (GUI)** and **Command-Driven** inputs.

*   **Desktop**: Graphical interface is recommended.
*   **Mobile**: Currently, file importing is done via commands.

Click the `[CMD]` button in the top-left corner to open the built-in debug console.

---

## 📖 Command Reference

### 1. Visuals
*   **Load Background**: `BG <image_path>`
    *   Instantly switches the background image. Supports `.jpg`, `.png`, `.webp`.
    *   Example: `BG C:\Assets\BlueArchive\BG_Classroom.png`
*   **Load Spine Character**: `LOAD <slot_ID> <.atlas_path>`
    *   Loads a character into slots `0` to `4`. Prints available animations upon success.
    *   Example: `LOAD 0 D:\Assets\Shiroko\Shiroko_Home.atlas`

### 2. Motion
*   **Change Animation**: `ANIM <slot_ID> <animation_name> [loop: true/false]`
    *   `true` loops the animation, `false` plays it once.
    *   Example: `ANIM 0 Start_Idle_01 true`

### 3. Storytelling
*   **Show Dialogue**: `TALK <name>|<affiliation>|<content>`
    *   Renders a standard dialogue box. **Parameters must be separated by a pipe `|`.**
    *   Example: `TALK Shiroko|Foreclosure Task Force|Sensei, are we robbing a bank?`

### 4. Audio
*   **Play BGM**: `BGM <audio_path>`
    *   Asynchronously loads and plays background music.
*   **Stop Music**: `STOP`
    *   Immediately stops the currently playing BGM.

---

### 💡 Pro Tips
*   **Path Issues**: Windows paths can be pasted directly; on Android/Termux, use absolute paths.
*   **Performance**: Thanks to the "Gentleman Scheduler," the UI thread remains silky smooth even fully loaded.
*   **Logs**: All operation results are printed in real-time in the console Log area.

---

## 🤝 Contribution Guidelines
<details>
<summary><strong>Click to expand: Please read these guidelines before contributing</strong></summary>

### Tech Stack Purity
This project insists on implementing 100% of its core logic in **Rust**.
*   **We reject** PRs introducing complex FFI with C++ runtimes (e.g., Qt, Unity, Unreal).
*   **Exceptions**: Safe Rust wrappers around low-level C system libraries are allowed.

### `unsafe` Rules
"Do not use unless necessary; when used, it must be foolproof." Every `unsafe` block must include a "Proof of Necessity."
</details>

---

## 🏛️ Architectural Philosophy
<details>
<summary><strong>Click to expand: Understand the hardcore low-level design</strong></summary>

### The "Gentleman Scheduler"
We do not trust default OS schedulers. Our scheduler isolates CPU cores:
*   **1 Core** reserved for UI/Render (Main Thread).
*   **1 Core** reserved for OS/Audio to prevent BGM popping.
*   **Remaining Cores** assigned to Workers for Spine calculations via `Rayon`.

### GUI Rules
UI components must strictly adhere to `egui`'s **Immediate Mode** philosophy.
</details>

---

## ✉️ Contact Us
*   **Author Email**: `3997101522@qq.com`
*   **QQ Group**: `1054276524`
