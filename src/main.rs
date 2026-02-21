/*
 *Project: AEFR (AEFR's Eternal Freedom & Rust-rendered)
 * GitHub: https://github.com/OxidizedSchale/AEFR-s-Eternal-Freedom-Rust-rendered
 *
 * 版权所有 (C) 2026 黛 (Dye) & AEFR Contributors
 *
 * 本程序是自由软件：您可以自由分发和/或修改它。
 * 它遵循由自由软件基金会（Free Software Foundation）发布的
 * GNU 通用公共许可证（GNU General Public License）第 3 版。
 *本程序的 git 仓库应带有 GPL3 许可证，请自行查看
 *
 * ----------------------------------------------------------------------------
 *
 * [项目架构概述 / Architecture Overview]
 *
 * AEFR 是一个基于 Rust 的高性能《蔚蓝档案》二创编辑器引擎。
 * 它采用了以下核心技术栈：
 *
 * 1. UI 框架: egui (即时模式 GUI，极低内存占用) + eframe (跨平台后端)
 * 2. 渲染核心: rusty_spine (Spine 2D 运行时 C 绑定的 Rust 封装)
 * 3. 并行计算: rayon (用于多核 CPU 并行计算 5 人同屏的骨骼变形)
 * 4. 音频系统: rodio (异步音频流播放)
 * 5. 调度系统: 自研 "Gentleman Scheduler" (防止计算线程抢占 UI 和音频线程)
 *
 * [跨平台支持 / Cross-Platform]
 * - Windows / Linux / macOS (原生桌面应用)
 * - Android Termux (X11/Wayland 环境)
 * - Android APK (原生应用打包)
 */

// 全局禁用 rust 的傻逼警告
#![allow(warnings)]

// --- 依赖导入 ---
use eframe::egui;
use egui::{
    epaint::Vertex, Color32, FontData, FontDefinitions, FontFamily, Mesh, Pos2, Rect, Shape,
    TextureHandle, TextureId, Vec2,
};
use rayon::prelude::*;
use rusty_spine::{
    AnimationState, AnimationStateData, Atlas, Skeleton, SkeletonJson, Slot, Physics,
};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::io::Cursor;
use std::sync::Arc;

// ============================================================================
// 1. 绅士调度器 (The Gentleman Scheduler)
// ============================================================================

/// **[调度器核心]**
/// 默认的 Rayon 线程池会贪婪地占用所有 CPU 核心。
/// 在移动设备上，这会导致 UI 线程（主线程）和音频线程被抢占，造成卡顿和爆音。
///
/// 本调度器采用 "N-2 策略"：永远保留至少 2 个核心给操作系统、UI 和音频。
struct AefrScheduler {
    /// 专用的计算线程池，与 UI 线程物理隔离
    pool: rayon::ThreadPool,
    /// 实际工作的计算线程数
    worker_count: usize,
}

impl AefrScheduler {
    fn new() -> Self {
        // 获取设备物理/逻辑核心数
        let logic_cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        
        // 策略逻辑：
        // - 核心 1: OS & Audio (高优先级)
        // - 核心 2: UI Render Loop (主线程)
        // - 剩余核心: Rayon Spine Calculation (计算密集型)
        let worker_count = if logic_cores > 2 { logic_cores - 2 } else { 1 };

        // 构建专用线程池
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(worker_count)
            .thread_name(|idx| format!("aefr-calc-{}", idx))
            // 增加栈大小以应对复杂的递归骨骼计算
            .stack_size(4 * 1024 * 1024) 
            .build()
            .expect("Failed to initialize AEFR Scheduler");

        println!("[Scheduler] Initialized. Cores: Total {}, Workers {}", logic_cores, worker_count);

        Self { pool, worker_count }
    }

    /// 在受限的线程池中执行闭包
    /// 任何在该闭包内调用的 `par_iter` 都会被限制在 `pool` 中，不会溢出到主线程。
    fn run_parallel<OP>(&self, op: OP)
    where
        OP: FnOnce() + Send,
    {
        self.pool.install(op);
    }
}

// ============================================================================
// 2. 智能跨平台入口 (Smart Entry Points)
// ============================================================================

// [场景 A] 桌面端 (Windows, macOS, Linux)
// 使用标准的 native 运行模式
#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("AEFR - OxidizedSchale Edition"),
        ..Default::default()
    };
    eframe::run_native("AEFR_App", options, Box::new(|cc| Box::new(AefrApp::new(cc))))
}

// [场景 B] Termux (Android Linux 环境)
// Termux 虽然内核是 Android，但用户通常希望通过 X11/Wayland 运行窗口。
// 此时没有 android_activity 上下文，按桌面模式处理。
#[cfg(target_os = "android")]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("AEFR_App", options, Box::new(|cc| Box::new(AefrApp::new(cc))))
}

// [场景 C] Android 原生 APK
// 当通过 cargo-apk 或 xbuild 打包时，系统会调用此入口。
// 必须接收 `AndroidApp` 上下文以处理生命周期事件。
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native("AEFR_App", options, Box::new(|cc| Box::new(AefrApp::new(cc))));
}

// ============================================================================
// 3. 异步指令系统 (Command System)
// ============================================================================

/// 应用程序内部的消息总线。
/// 用于解耦 UI 线程和后台加载线程，避免 IO 操作阻塞渲染循环。
#[derive(Debug)]
enum AppCommand {
    /// 设置对话框内容
    Dialogue { name: String, affiliation: String, content: String },
    
    /// 请求异步加载资源 (槽位索引, 文件路径)
    RequestLoad { slot_idx: usize, path: String },
    
    /// 资源加载完成回调 (包含构建好的 Spine 对象和可用动画列表)
    LoadSuccess(usize, Box<SpineObject>, Vec<String>),
    
    /// 异步加载背景图
    LoadBackground(String),
    
    /// 播放 BGM (路径)
    PlayBgm(String),
    
    /// BGM 数据预读完成 (二进制数据)
    BgmReady(Vec<u8>), 
    
    /// 停止播放 BGM
    StopBgm,
    
    /// 切换角色动画 (槽位, 动画名, 是否循环)
    SetAnimation { slot_idx: usize, anim_name: String, loop_anim: bool },
    
    /// 控制台日志输出
    Log(String),
}

// ============================================================================
// 4. 音频管理器 (Audio Manager)
// ============================================================================

/// 基于 rodio 的简单音频管理器
struct AudioManager {
    _stream: rodio::OutputStream,
    _stream_handle: rodio::OutputStreamHandle,
    sink: rodio::Sink,
}

impl AudioManager {
    fn new() -> Option<Self> {
        // 尝试获取默认音频输出设备
        let (_stream, stream_handle) = rodio::OutputStream::try_default().ok()?;
        let sink = rodio::Sink::try_new(&stream_handle).ok()?;
        Some(Self { _stream, _stream_handle: stream_handle, sink })
    }

    fn play(&self, data: Vec<u8>) {
        // 使用 Cursor 在内存中读取音频数据，避免持有文件句柄
        let cursor = Cursor::new(data);
        if let Ok(source) = rodio::Decoder::new(cursor) {
            self.sink.stop(); // 简单的单轨播放逻辑：切歌先停
            self.sink.append(source);
            self.sink.play();
        }
    }

    fn stop(&self) { self.sink.stop(); }
}

// ============================================================================
// 5. Spine 渲染核心 (Spine Rendering Engine)
// ============================================================================

/// 封装后的 Spine 对象。
/// 实现了 `Send` trait 以便在 Rayon 线程池中并行计算。
pub struct SpineObject {
    skeleton: Skeleton,
    state: AnimationState,
    _texture: TextureHandle, // 保持 GPU 纹理存活
    texture_id: TextureId,   // 用于 egui 绘制命令
    pub position: Pos2,      // 屏幕位置
    pub scale: f32,          // 缩放比例
    // 保留 SkeletonData 用于后续查询动画名称
    skeleton_data: Arc<rusty_spine::SkeletonData>, 
}

impl std::fmt::Debug for SpineObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpineObject").field("pos", &self.position).finish()
    }
}

// 安全声明：rusty_spine 底层是 C 指针，但在 Rust 封装层我们保证线程安全
unsafe impl Send for SpineObject {}

impl SpineObject {
    /// **[异步加载器]**
    /// 在后台线程运行，负责 IO 读取、纹理上传和 Spine 数据解析。
    /// 返回构造好的对象和该角色包含的所有动画名称列表。
    fn load_async(ctx: &egui::Context, path_str: &str) -> Option<(Self, Vec<String>)> {
        let atlas_path = std::path::Path::new(path_str);
        
        // 1. 加载 Atlas (图集)
        let atlas = Arc::new(Atlas::new_from_file(atlas_path).ok()?);
        
        // 2. 加载纹理并上传至 GPU
        // 注意：egui Context 是线程安全的，可以在后台线程 load_texture
        let (texture_handle, texture_id) = if let Some(page) = atlas.pages().next() {
            let img_path = atlas_path.parent()?.join(page.name());
            let img = image::open(&img_path).ok()?;
            let size = [img.width() as usize, img.height() as usize];
            let rgba8 = img.to_rgba8();
            let c_img = egui::ColorImage::from_rgba_unmultiplied(size, rgba8.as_raw());
            let h = ctx.load_texture(page.name(), c_img, egui::TextureOptions::LINEAR);
            let id = h.id();
            (h, id)
        } else { return None; };

        // 3. 加载 Skeleton (骨骼数据)
        let json_path = atlas_path.with_extension("json");
        let skeleton_json = SkeletonJson::new(atlas);
        let skeleton_data = Arc::new(skeleton_json.read_skeleton_data_file(json_path).ok()?);
        let state_data = Arc::new(AnimationStateData::new(skeleton_data.clone()));
        let mut state = AnimationState::new(state_data);

        // 4. 提取动画列表
        let anim_names: Vec<String> = skeleton_data.animations().map(|a| a.name().to_string()).collect();
        
        // 默认播放第一个动画
        if let Some(anim) = skeleton_data.animations().next() { 
            let _ = state.set_animation(0, &anim, true); 
        }

        let obj = Self {
            skeleton: Skeleton::new(skeleton_data.clone()),
            state,
            _texture: texture_handle,
            texture_id,
            position: Pos2::new(0.0, 0.0),
            scale: 0.5,
            skeleton_data,
        };
        Some((obj, anim_names))
    }
    
    /// 按名称切换动画
    fn set_animation_by_name(&mut self, anim_name: &str, loop_anim: bool) -> bool {
        if let Some(anim) = self.skeleton_data.animations().find(|a| a.name() == anim_name) {
            let _ = self.state.set_animation(0, &anim, loop_anim);
            true
        } else { false }
    }

    /// **[并行更新]**
    /// 计算骨骼变换。此函数将在 Rayon 线程池中运行。
    fn update_parallel(&mut self, dt: f32) {
        self.state.update(dt);
        let _ = self.state.apply(&mut self.skeleton);
        self.skeleton.update_world_transform(Physics::None); // v0.8 暂时禁用物理以提升性能
    }

    /// **[渲染管线]**
    /// 将计算好的骨骼数据转换为 egui 可识别的 Mesh (顶点+索引)。
    fn paint(&self, ui: &mut egui::Ui) {
        let mut mesh = Mesh::with_texture(self.texture_id);
        // 预分配顶点缓冲区，避免频繁内存重分配
        let mut world_vertices = Vec::with_capacity(1024);
        
        for slot in self.skeleton.draw_order() {
            if let Some(attachment) = slot.attachment() {
                // 处理 Region (静态图块)
                if let Some(region) = attachment.as_region() {
                    unsafe {
                        if world_vertices.len() < 8 { world_vertices.resize(8, 0.0); }
                        region.compute_world_vertices(&*slot, &mut world_vertices, 0, 2);
                        self.push_to_mesh(&mut mesh, &world_vertices[0..8], &region.uvs(), &[0, 1, 2, 2, 3, 0], &*slot, region.color());
                    }
                } 
                // 处理 Mesh (自由变形网格)
                else if let Some(mesh_att) = attachment.as_mesh() {
                    unsafe {
                        let len = mesh_att.world_vertices_length() as usize;
                        if world_vertices.len() < len { world_vertices.resize(len, 0.0); }
                        mesh_att.compute_world_vertices(&*slot, 0, len as i32, &mut world_vertices, 0, 2);
                        let uvs = std::slice::from_raw_parts(mesh_att.uvs(), len);
                        let tris = std::slice::from_raw_parts(mesh_att.triangles(), mesh_att.triangles_count() as usize);
                        self.push_to_mesh(&mut mesh, &world_vertices[0..len], uvs, tris, &*slot, mesh_att.color());
                    }
                }
            }
        }
        ui.painter().add(Shape::mesh(mesh));
    }

    /// 辅助函数：将顶点压入 Mesh 并执行坐标系转换 (Spine -> Screen)
    fn push_to_mesh(&self, mesh: &mut Mesh, w_v: &[f32], uvs: &[f32], tris: &[u16], slot: &Slot, att_c: rusty_spine::Color) {
        let s_c = slot.color();
        // 计算预乘 Alpha 颜色 (Premultiplied Alpha)
        let color = Color32::from_rgba_premultiplied(
            (s_c.r * att_c.r * 255.0) as u8, (s_c.g * att_c.g * 255.0) as u8,
            (s_c.b * att_c.b * 255.0) as u8, (s_c.a * att_c.a * 255.0) as u8,
        );
        let idx_offset = mesh.vertices.len() as u32;
        let count = usize::min(uvs.len() / 2, w_v.len() / 2);
        
        for i in 0..count {
            // 坐标变换：Y 轴翻转 + 缩放 + 平移
            let pos = Pos2::new(
                w_v[i*2] * self.scale + self.position.x,
                -w_v[i*2+1] * self.scale + self.position.y
            );
            mesh.vertices.push(Vertex { pos, uv: Pos2::new(uvs[i*2], uvs[i*2+1]), color });
        }
        for &idx in tris { mesh.indices.push(idx_offset + idx as u32); }
    }
}

// ============================================================================
// 6. 应用主程序 (Main Application)
// ============================================================================

struct AefrApp {
    scheduler: AefrScheduler,

    // 剧情状态
    current_name: String,
    current_affiliation: String,
    
    // 打字机效果 (Typewriter Effect)
    target_chars: Vec<char>, // 目标完整文本
    visible_count: usize,    // 当前显示字数
    type_timer: f32,
    type_speed: f32,         // 打字速度 (秒/字)

    // 资源槽位 (0-4)
    characters: Vec<Option<SpineObject>>,
    background: Option<TextureHandle>,
    
    // 系统模块
    audio_manager: Option<AudioManager>,
    tx: Sender<AppCommand>,
    rx: Receiver<AppCommand>,

    // 调试控制台
    console_open: bool,
    console_input: String,
    console_logs: Vec<String>,
}

impl AefrApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let (tx, rx) = channel();
        
        // 启动调度器
        let scheduler = AefrScheduler::new();

        // 尝试初始化音频设备，失败不崩溃
        let audio_manager = match AudioManager::new() {
            Some(mgr) => Some(mgr),
            None => { println!("Audio init failed, running in silent mode."); None }
        };

        Self {
            scheduler,
            current_name: "System".into(),
            current_affiliation: "AEFR".into(),
            target_chars: "AEFR v0.8 Scheduler Online.\nReady for orders.".chars().collect(),
            visible_count: 0, 
            type_timer: 0.0,
            type_speed: 0.03, // 30ms 一个字
            characters: (0..5).map(|_| None).collect(),
            background: None,
            audio_manager,
            tx, rx,
            console_open: false,
            console_input: String::new(),
            console_logs: vec!["Scheduler ready.".into()],
        }
    }

    /// 解析控制台输入并派发指令
    fn parse_and_send_command(&mut self) {
        let input = self.console_input.trim().to_owned();
        if input.is_empty() { return; }
        self.console_logs.push(format!("> {}", input));

        let tx = self.tx.clone();
        
        // --- 简单指令解析器 ---
        if let Some(rest) = input.strip_prefix("LOAD ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(idx) = parts[0].parse::<usize>() {
                    tx.send(AppCommand::RequestLoad { slot_idx: idx, path: parts[1].replace("\"", "") }).ok();
                }
            }
        } else if let Some(rest) = input.strip_prefix("ANIM ") {
            // 格式: ANIM <slot> <name> [loop=true]
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(idx) = parts[0].parse::<usize>() {
                    let anim_name = parts[1].to_string();
                    let loop_anim = parts.get(2).map(|s| *s == "true").unwrap_or(true);
                    tx.send(AppCommand::SetAnimation { slot_idx: idx, anim_name, loop_anim }).ok();
                }
            }
        } else if let Some(path) = input.strip_prefix("BGM ") {
             tx.send(AppCommand::PlayBgm(path.replace("\"", ""))).ok();
        } else if input.eq_ignore_ascii_case("STOP") {
             tx.send(AppCommand::StopBgm).ok();
        } else if let Some(rest) = input.strip_prefix("TALK ") {
            let p: Vec<&str> = rest.split('|').collect();
            if p.len() == 3 {
                tx.send(AppCommand::Dialogue { name: p[0].to_owned(), affiliation: p[1].to_owned(), content: p[2].to_owned() }).ok();
            }
        } else if let Some(path) = input.strip_prefix("BG ") {
            tx.send(AppCommand::LoadBackground(path.replace("\"", ""))).ok();
        } else if input.eq_ignore_ascii_case("HELP") {
            self.console_logs.push("Commands: LOAD, ANIM, BGM, BG, TALK".into());
        }
        self.console_input.clear();
    }

    /// 处理异步事件回调
    fn handle_async_events(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                AppCommand::Dialogue { name, affiliation, content } => { 
                    self.current_name = name; 
                    self.current_affiliation = affiliation; 
                    self.target_chars = content.chars().collect();
                    self.visible_count = 0; // 重置打字机
                }
                AppCommand::Log(msg) => self.console_logs.push(msg),
                AppCommand::RequestLoad { slot_idx, path } => {
                    let tx_cb = self.tx.clone();
                    let ctx_clone = ctx.clone();
                    self.console_logs.push(format!("Loading slot {}...", slot_idx));
                    
                    // 派发到后台线程
                    thread::spawn(move || {
                        if let Some((obj, anims)) = SpineObject::load_async(&ctx_clone, &path) {
                            tx_cb.send(AppCommand::LoadSuccess(slot_idx, Box::new(obj), anims)).ok();
                        } else {
                            tx_cb.send(AppCommand::Log(format!("Load failed: {}", path))).ok();
                        }
                    });
                }
                AppCommand::LoadSuccess(idx, obj, anims) => {
                    if let Some(slot) = self.characters.get_mut(idx) {
                        let mut loaded = *obj;
                        // 简单的自动排版逻辑
                        loaded.position = Pos2::new(200.0 + idx as f32 * 220.0, 720.0);
                        *slot = Some(loaded);
                        self.console_logs.push(format!("Slot {} Loaded. Anims: {}", idx, anims.len()));
                    }
                }
                AppCommand::SetAnimation { slot_idx, anim_name, loop_anim } => {
                     if let Some(Some(char)) = self.characters.get_mut(slot_idx) {
                         if char.set_animation_by_name(&anim_name, loop_anim) {
                             self.console_logs.push(format!("Slot {} -> {}", slot_idx, anim_name));
                         } else {
                             self.console_logs.push(format!("Anim not found: {}", anim_name));
                         }
                     }
                }
                AppCommand::LoadBackground(path) => {
                    if let Ok(img) = image::open(&path) {
                        let rgba = img.to_rgba8();
                        let c_img = egui::ColorImage::from_rgba_unmultiplied([img.width() as _, img.height() as _], rgba.as_raw());
                        self.background = Some(ctx.load_texture(&path, c_img, egui::TextureOptions::LINEAR));
                        self.console_logs.push("BG Loaded.".into());
                    }
                }
                AppCommand::PlayBgm(path) => {
                    let tx_cb = self.tx.clone();
                    // 异步读取音频文件
                    thread::spawn(move || {
                        if let Ok(data) = std::fs::read(&path) {
                            tx_cb.send(AppCommand::BgmReady(data)).ok();
                        } else {
                            tx_cb.send(AppCommand::Log("Audio read failed.".into())).ok();
                        }
                    });
                }
                AppCommand::BgmReady(data) => {
                    if let Some(mgr) = &self.audio_manager {
                        mgr.play(data);
                        self.console_logs.push("Playing BGM.".into());
                    }
                }
                AppCommand::StopBgm => {
                    if let Some(mgr) = &self.audio_manager { mgr.stop(); }
                }
            }
        }
    }
}

impl eframe::App for AefrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. 事件处理
        self.handle_async_events(ctx);
        let dt = ctx.input(|i| i.stable_dt);

        // 2. 打字机更新
        if self.visible_count < self.target_chars.len() {
            self.type_timer += dt;
            if self.type_timer > self.type_speed {
                self.visible_count += 1;
                self.type_timer = 0.0;
                ctx.request_repaint(); // 请求刷新以显示新字
            }
        }

        // 3. Spine 并行计算 (由 Gentleman Scheduler 托管)
        self.scheduler.run_parallel(|| {
            self.characters.par_iter_mut().for_each(|slot| {
                if let Some(char) = slot { 
                    // 计算骨骼变形
                    char.update_parallel(dt); 
                }
            });
        });

        // 如果有角色，持续刷新以播放动画
        if self.characters.iter().any(|c| c.is_some()) { ctx.request_repaint(); }

        // 4. UI 绘制
        egui::CentralPanel::default().show(ctx, |ui| {
            let screen_rect = ui.max_rect();
            
            // 背景层
            if let Some(bg) = &self.background {
                ui.painter().image(bg.id(), screen_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }

            // 角色层
            for char in self.characters.iter().flatten() { char.paint(ui); }
            
            // 对话框层
            let current_text: String = self.target_chars.iter().take(self.visible_count).collect();
            // 如果点击了对话框，瞬间显示全部
            if draw_dialogue_ui(ui, screen_rect, &self.current_name, &self.current_affiliation, &current_text) {
                self.visible_count = self.target_chars.len();
            }

            // 控制台按钮
            let cmd_rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(60.0, 40.0));
            if ui.put(cmd_rect, egui::Button::new("CMD")).clicked() { self.console_open = !self.console_open; }
            
            // 控制台窗口
            if self.console_open { draw_console_window(ctx, self); }
        });
    }
}

// ============================================================================
// 7. UI 组件函数
// ============================================================================

/// 绘制对话框，返回是否被点击
fn draw_dialogue_ui(ui: &mut egui::Ui, screen: Rect, name: &str, affiliation: &str, content: &str) -> bool {
    let box_h = 160.0;
    let box_rect = Rect::from_min_max(Pos2::new(0.0, screen.bottom() - box_h), screen.max);
    
    // 半透明黑底
    ui.painter().rect_filled(box_rect, 5.0, Color32::from_black_alpha(180));
    
    // 透明按钮覆盖，用于检测点击
    let response = ui.allocate_rect(box_rect, egui::Sense::click());
    
    // 名字标签
    if !name.is_empty() {
        let name_pos = box_rect.left_top() + Vec2::new(100.0, 20.0);
        ui.painter().text(name_pos, egui::Align2::LEFT_TOP, format!("{} [{}]", name, affiliation), egui::FontId::proportional(22.0), Color32::WHITE);
    }
    
    // 内容文本
    ui.painter().text(box_rect.left_top() + Vec2::new(100.0, 50.0), egui::Align2::LEFT_TOP, content, egui::FontId::proportional(26.0), Color32::WHITE);
    
    response.clicked()
}

/// 绘制调试控制台
fn draw_console_window(ctx: &egui::Context, app: &mut AefrApp) {
    egui::Window::new("AEFR CONSOLE").default_size([600.0, 400.0]).show(ctx, |ui| {
        egui::ScrollArea::vertical().stick_to_bottom(true).max_height(300.0).show(ui, |ui| {
            for log in &app.console_logs { ui.monospace(log); }
        });
        ui.horizontal(|ui| {
            if ui.text_edit_singleline(&mut app.console_input).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                app.parse_and_send_command();
            }
        });
    });
}

/// 跨平台字体加载
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let paths = vec!["/system/fonts/NotoSansCJK-Regular.ttc", "C:\\Windows\\Fonts\\msyh.ttc"];
    for p in paths {
        if let Ok(d) = std::fs::read(p) {
            fonts.font_data.insert("sys".into(), FontData::from_owned(d));
            fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "sys".into());
            ctx.set_fonts(fonts);
            return;
        }
    }
}
