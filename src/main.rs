/*
 * Project: AEFR (AEFR's Eternal Freedom & Rust-rendered)
 * GitHub: https://github.com/OxidizedSchale/AEFR-s-Eternal-Freedom-Rust-rendered
 *
 * 版权所有 (C) 2026 黛 (Dye) & AEFR Contributors
 *
 * 本程序是自由软件：您可以自由分发和/或修改它。
 * 它遵循由自由软件基金会（Free Software Foundation）发布的
 * GNU 通用公共许可证（GNU General Public License）第 3 版。
 *本程序的 git 仓库应带有 GPL3 许可证，请自行查看
 */

//全局关闭rust的大傻逼警告
#![allow(warnings)]

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
// 1. 跨平台入口
// ============================================================================

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

#[cfg(target_os = "android")]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("AEFR_App", options, Box::new(|cc| Box::new(AefrApp::new(cc))))
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native("AEFR_App", options, Box::new(|cc| Box::new(AefrApp::new(cc))));
}

// ============================================================================
// 2. 指令系统
// ============================================================================

#[derive(Debug)]
enum AppCommand {
    Dialogue { name: String, affiliation: String, content: String },
    RequestLoad { slot_idx: usize, path: String },
    /// 加载成功：槽位 | Spine对象 | 可用动画列表
    LoadSuccess(usize, Box<SpineObject>, Vec<String>),
    LoadBackground(String),
    /// 请求播放 BGM: 路径
    PlayBgm(String),
    /// BGM 数据加载完毕: 音频数据(字节流)
    BgmReady(Vec<u8>), 
    StopBgm,
    /// 切换动作: 槽位 | 动作名 | 是否循环
    SetAnimation { slot_idx: usize, anim_name: String, loop_anim: bool },
    Log(String),
}

// ============================================================================
// 3. 音频管理器 (Audio Manager)
// ============================================================================

struct AudioManager {
    _stream: rodio::OutputStream,
    _stream_handle: rodio::OutputStreamHandle,
    sink: rodio::Sink,
}

impl AudioManager {
    fn new() -> Option<Self> {
        // 初始化音频输出流
        let (_stream, stream_handle) = rodio::OutputStream::try_default().ok()?;
        let sink = rodio::Sink::try_new(&stream_handle).ok()?;
        Some(Self {
            _stream,
            _stream_handle: stream_handle,
            sink,
        })
    }

    fn play(&self, data: Vec<u8>) {
        // 使用 Cursor 将内存中的字节流包装成 Source
        let cursor = Cursor::new(data);
        if let Ok(source) = rodio::Decoder::new(cursor) {
            self.sink.stop(); // 切歌前先停止上一首
            self.sink.append(source);
            self.sink.play();
        }
    }

    fn stop(&self) {
        self.sink.stop();
    }
}

// ============================================================================
// 4. Spine 渲染核心
// ============================================================================

pub struct SpineObject {
    skeleton: Skeleton,
    state: AnimationState,
    _texture: TextureHandle,
    texture_id: TextureId,
    pub position: Pos2,
    pub scale: f32,
    // 保存 skeleton_data 的引用以便后续查询动画名
    skeleton_data: Arc<rusty_spine::SkeletonData>, 
}

// 手动实现 Debug
impl std::fmt::Debug for SpineObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpineObject").field("pos", &self.position).finish()
    }
}

unsafe impl Send for SpineObject {}

impl SpineObject {
    fn load_async(ctx: &egui::Context, path_str: &str) -> Option<(Self, Vec<String>)> {
        let atlas_path = std::path::Path::new(path_str);
        let atlas = Arc::new(Atlas::new_from_file(atlas_path).ok()?);
        
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

        let json_path = atlas_path.with_extension("json");
        let skeleton_json = SkeletonJson::new(atlas);
        let skeleton_data = Arc::new(skeleton_json.read_skeleton_data_file(json_path).ok()?);
        let state_data = Arc::new(AnimationStateData::new(skeleton_data.clone()));
        let mut state = AnimationState::new(state_data);

        // 获取所有可用动画名称
        let anim_names: Vec<String> = skeleton_data.animations().map(|a| a.name().to_string()).collect();

        // 默认播放第一个动画 (通常是 Idle)
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
    
    /// 切换当前播放的动画
    fn set_animation_by_name(&mut self, anim_name: &str, loop_anim: bool) -> bool {
        // 从 skeleton_data 中查找动画
        if let Some(anim) = self.skeleton_data.animations().find(|a| a.name() == anim_name) {
            let _ = self.state.set_animation(0, &anim, loop_anim);
            true
        } else {
            false
        }
    }

    fn update_parallel(&mut self, dt: f32) {
        self.state.update(dt);
        let _ = self.state.apply(&mut self.skeleton);
        self.skeleton.update_world_transform(Physics::None);
    }

    fn paint(&self, ui: &mut egui::Ui) {
        let mut mesh = Mesh::with_texture(self.texture_id);
        let mut world_vertices = Vec::with_capacity(1024);
        for slot in self.skeleton.draw_order() {
            if let Some(attachment) = slot.attachment() {
                if let Some(region) = attachment.as_region() {
                    unsafe {
                        if world_vertices.len() < 8 { world_vertices.resize(8, 0.0); }
                        region.compute_world_vertices(&*slot, &mut world_vertices, 0, 2);
                        self.push_to_mesh(&mut mesh, &world_vertices[0..8], &region.uvs(), &[0, 1, 2, 2, 3, 0], &*slot, region.color());
                    }
                } else if let Some(mesh_att) = attachment.as_mesh() {
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

    fn push_to_mesh(&self, mesh: &mut Mesh, w_v: &[f32], uvs: &[f32], tris: &[u16], slot: &Slot, att_c: rusty_spine::Color) {
        let s_c = slot.color();
        let color = Color32::from_rgba_premultiplied(
            (s_c.r * att_c.r * 255.0) as u8, (s_c.g * att_c.g * 255.0) as u8,
            (s_c.b * att_c.b * 255.0) as u8, (s_c.a * att_c.a * 255.0) as u8,
        );
        let idx_offset = mesh.vertices.len() as u32;
        let count = usize::min(uvs.len() / 2, w_v.len() / 2);
        for i in 0..count {
            let pos = Pos2::new(w_v[i*2] * self.scale + self.position.x, -w_v[i*2+1] * self.scale + self.position.y);
            mesh.vertices.push(Vertex { pos, uv: Pos2::new(uvs[i*2], uvs[i*2+1]), color });
        }
        for &idx in tris { mesh.indices.push(idx_offset + idx as u32); }
    }
}

// ============================================================================
// 5. 应用主逻辑
// ============================================================================

struct AefrApp {
    current_name: String,
    current_affiliation: String,
    
    // 打字机
    target_chars: Vec<char>,
    visible_count: usize,
    type_timer: f32,
    type_speed: f32,

    // 资源
    characters: Vec<Option<SpineObject>>,
    background: Option<TextureHandle>,
    
    // 音频
    audio_manager: Option<AudioManager>,

    tx: Sender<AppCommand>,
    rx: Receiver<AppCommand>,

    console_open: bool,
    console_input: String,
    console_logs: Vec<String>,
}

impl AefrApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let (tx, rx) = channel();
        
        // 初始化音频管理器 (如果失败，仅打印日志，不崩溃)
        let audio_manager = match AudioManager::new() {
            Some(mgr) => Some(mgr),
            None => {
                println!("Warning: Audio device not found.");
                None
            }
        };

        Self {
            current_name: "System".into(),
            current_affiliation: "AEFR".into(),
            target_chars: "AEFR v0.7 Ultimate Ready.\nAudio & Animation systems online.".chars().collect(),
            visible_count: 0, 
            type_timer: 0.0,
            type_speed: 0.03,
            characters: (0..5).map(|_| None).collect(),
            background: None,
            audio_manager,
            tx, rx,
            console_open: false,
            console_input: String::new(),
            console_logs: vec!["Type 'HELP' for new commands.".into()],
        }
    }

    fn parse_and_send_command(&mut self) {
        let input = self.console_input.trim().to_owned();
        if input.is_empty() { return; }
        self.console_logs.push(format!("> {}", input));

        let tx = self.tx.clone();
        if let Some(rest) = input.strip_prefix("LOAD ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(idx) = parts[0].parse::<usize>() {
                    tx.send(AppCommand::RequestLoad { slot_idx: idx, path: parts[1].replace("\"", "") }).ok();
                }
            }
        } else if let Some(rest) = input.strip_prefix("ANIM ") {
            // 解析: ANIM <slot> <name> [loop]
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(idx) = parts[0].parse::<usize>() {
                    let anim_name = parts[1].to_string();
                    let loop_anim = parts.get(2).map(|s| *s == "true").unwrap_or(true); // 默认循环
                    tx.send(AppCommand::SetAnimation { slot_idx: idx, anim_name, loop_anim }).ok();
                }
            } else {
                self.console_logs.push("Usage: ANIM <slot> <anim_name> [true/false]".into());
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
            self.console_logs.push("Commands:".into());
            self.console_logs.push("  ANIM <0-4> <anim_name> [true/false]".into());
            self.console_logs.push("  BGM <path_to_audio> | STOP".into());
            self.console_logs.push("  LOAD <0-4> <path> | BG <path>".into());
            self.console_logs.push("  TALK <name>|<aff>|<msg>".into());
        }
        self.console_input.clear();
    }

    fn handle_async_events(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                AppCommand::Dialogue { name, affiliation, content } => { 
                    self.current_name = name; 
                    self.current_affiliation = affiliation; 
                    self.target_chars = content.chars().collect();
                    self.visible_count = 0;
                }
                AppCommand::Log(msg) => self.console_logs.push(msg),
                AppCommand::RequestLoad { slot_idx, path } => {
                    let tx_cb = self.tx.clone();
                    let ctx_clone = ctx.clone();
                    self.console_logs.push(format!("Loading slot {}...", slot_idx));
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
                        loaded.position = Pos2::new(200.0 + idx as f32 * 220.0, 720.0);
                        *slot = Some(loaded);
                        self.console_logs.push(format!("Slot {} Loaded.", idx));
                        self.console_logs.push(format!("Avail Anims: {:?}", anims)); // 打印可用动作
                    }
                }
                AppCommand::SetAnimation { slot_idx, anim_name, loop_anim } => {
                     if let Some(Some(char)) = self.characters.get_mut(slot_idx) {
                         if char.set_animation_by_name(&anim_name, loop_anim) {
                             self.console_logs.push(format!("Slot {} anim set to '{}'", slot_idx, anim_name));
                         } else {
                             self.console_logs.push(format!("Anim '{}' not found in slot {}", anim_name, slot_idx));
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
                    self.console_logs.push(format!("Loading BGM: {}", path));
                    // 异步读取文件，避免阻塞UI
                    thread::spawn(move || {
                        if let Ok(data) = std::fs::read(&path) {
                            tx_cb.send(AppCommand::BgmReady(data)).ok();
                        } else {
                            tx_cb.send(AppCommand::Log("Failed to read audio file.".into())).ok();
                        }
                    });
                }
                AppCommand::BgmReady(data) => {
                    if let Some(mgr) = &self.audio_manager {
                        mgr.play(data);
                        self.console_logs.push("BGM Playing.".into());
                    }
                }
                AppCommand::StopBgm => {
                    if let Some(mgr) = &self.audio_manager {
                        mgr.stop();
                        self.console_logs.push("BGM Stopped.".into());
                    }
                }
            }
        }
    }
}

impl eframe::App for AefrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_async_events(ctx);
        let dt = ctx.input(|i| i.stable_dt);

        if self.visible_count < self.target_chars.len() {
            self.type_timer += dt;
            if self.type_timer > self.type_speed {
                self.visible_count += 1;
                self.type_timer = 0.0;
                ctx.request_repaint();
            }
        }

        self.characters.par_iter_mut().for_each(|slot| {
            if let Some(char) = slot { char.update_parallel(dt); }
        });
        if self.characters.iter().any(|c| c.is_some()) { ctx.request_repaint(); }

        egui::CentralPanel::default().show(ctx, |ui| {
            let screen_rect = ui.max_rect();
            if let Some(bg) = &self.background {
                ui.painter().image(bg.id(), screen_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            for char in self.characters.iter().flatten() { char.paint(ui); }
            
            let current_text: String = self.target_chars.iter().take(self.visible_count).collect();
            if draw_dialogue_ui(ui, screen_rect, &self.current_name, &self.current_affiliation, &current_text) {
                if self.visible_count < self.target_chars.len() {
                    self.visible_count = self.target_chars.len();
                }
            }

            let cmd_rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(60.0, 40.0));
            if ui.put(cmd_rect, egui::Button::new("CMD")).clicked() { self.console_open = !self.console_open; }
            if self.console_open { draw_console_window(ctx, self); }
        });
    }
}

fn draw_dialogue_ui(ui: &mut egui::Ui, screen: Rect, name: &str, affiliation: &str, content: &str) -> bool {
    let box_h = 160.0;
    let box_rect = Rect::from_min_max(Pos2::new(0.0, screen.bottom() - box_h), screen.max);
    ui.painter().rect_filled(box_rect, 5.0, Color32::from_black_alpha(180));
    let response = ui.allocate_rect(box_rect, egui::Sense::click());
    
    if !name.is_empty() {
        let name_pos = box_rect.left_top() + Vec2::new(100.0, 20.0);
        ui.painter().text(name_pos, egui::Align2::LEFT_TOP, format!("{} [{}]", name, affiliation), egui::FontId::proportional(22.0), Color32::WHITE);
    }
    ui.painter().text(box_rect.left_top() + Vec2::new(100.0, 50.0), egui::Align2::LEFT_TOP, content, egui::FontId::proportional(26.0), Color32::WHITE);
    response.clicked()
}

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
