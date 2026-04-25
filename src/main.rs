/*
 * GNU:AEFR (GNU's Not Unix:AEFR's Eternal Freedom & Rust-rendered)
 * Copyright (C) 2026 OxidizedSchale & The Executive Committee of GNU: AEFR
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License.
 *
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * 
 * GitHub: https://github.com/OxidizedSchale/GNU-AEFR
 *
 * 版权所有 (C) 2026 OxidizedSchale & The Executive Committee of GNU: AEFR
 *
 * 本程序是自由软件：您可以自由分发和/或修改它。
 * 它遵循由自由软件基金会（Free Software Foundation）发布的
 * GNU Affero 通用公共许可证（GNU Affero General Public License）第 3 版。
 * 本程序的 git 仓库应带有 AGPL3 许可证，请自行查看
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
 * - Windows | GNU/Linux | macOS (原生桌面应用)
 * - Android Termux (X11/Wayland 环境)
 * - Android APK (原生应用打包)
 *
 */

// 全局禁用 Rust 的大傻逼警告
// 注意：在生产环境中应逐步解决警告，而非全局禁用
#![allow(warnings)]

// ============================================================================
// 依赖导入
// ============================================================================
// GUI框架相关
use eframe::egui;
use egui::{
    epaint::Vertex, Color32, FontData, FontDefinitions, FontFamily, Mesh, Pos2, Rect, Shape,
    TextureHandle, TextureId, Vec2, Stroke,
};

// 并行计算库
use rayon::prelude::*;

// Spine骨骼动画库（C库的Rust绑定）
use rusty_spine::{
    AnimationState, AnimationStateData, Atlas, Skeleton, SkeletonJson, SkeletonBinary, Slot,
};

// 线程通信
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

// IO和序列化
use std::io::Cursor;
use std::sync::{Arc, Mutex}; // 引入 Mutex 保障线程安全
use rodio::Source;
use serde::{Serialize, Deserialize};

// ============================================================================
// 常量定义 (干掉魔法数字)
// ============================================================================
const BASE_HEIGHT: f32 = 720.0;               // 基础画布高度，用于DPI缩放计算
const DIALOGUE_BOX_RATIO: f32 = 0.28;         // 对话框占屏幕高度的比例
const MAX_DT: f32 = 0.033;                    // 最大delta时间，防止卡顿导致的动画跳跃
const TYPEWRITER_INTERVAL: f32 = 0.03;        // 打字机效果：每个字符显示间隔（秒）
const CHAR_BASE_SCALE: f32 = 0.45;            // 角色基础缩放系数
const CHAR_X_START_PERCENT: f32 = 0.15;       // 1号位角色在屏幕水平方向起始位置（百分比）
const CHAR_X_STEP_PERCENT: f32 = 0.175;       // 角色槽位之间的水平间距（百分比）

// ============================================================================
// 数据结构定义
// ============================================================================
/// 场景数据结构：表示一帧剧本场景
/// 序列化支持：用于保存/加载剧本文件
#[derive(Serialize, Deserialize, Clone, Default)]
struct Scene {
    bg_path: Option<String>,              // 背景图片路径
    bgm_path: Option<String>,             // 背景音乐路径
    char_paths:[Option<String>; 5],       // 5个角色槽位的Spine文件路径
    char_anims:[Option<String>; 5],       // 对应角色的当前动画名称
    speaker_name: String,                 // 当前说话角色名称
    speaker_aff: String,                  // 角色所属组织/学校
    dialogue_content: String,             // 对话内容
}

/// 剧本数据结构：包含多个场景
/// 整个剧本文件对应一个Scenario实例
#[derive(Serialize, Deserialize, Clone, Default)]
struct Scenario {
    scenes: Vec<Scene>,                   // 场景列表，按时间顺序排列
}

// ============================================================================
// 程序入口点
// ============================================================================
/// 桌面平台主函数
#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result<()> {
    // 配置原生窗口选项
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])      // 初始窗口大小
            .with_title("GNU's Not Unix : AEFR's Eternal Freedom & Rust_rendered"), // 窗口标题
        vsync: true,                               // 启用垂直同步，防止画面撕裂
        ..Default::default()
    };
    
    // 启动eframe应用
    eframe::run_native(
        "AEFR_App", 
        options, 
        Box::new(|cc| Box::new(AefrApp::new(cc)))  // 创建应用实例
    )
}

/// Android平台主函数（简化配置）
#[cfg(target_os = "android")]
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "AEFR_App", 
        eframe::NativeOptions::default(),          // 使用默认配置，适应移动端
        Box::new(|cc| Box::new(AefrApp::new(cc)))
    )
}

/// Android原生入口点（供Android运行时调用）
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    // 包装eframe入口，适配Android Activity生命周期
    let _ = eframe::run_native(
        "AEFR_App", 
        eframe::NativeOptions::default(), 
        Box::new(|cc| Box::new(AefrApp::new(cc)))
    );
}

// ============================================================================
// 非阻塞 + 跨平台文件选择器模块
// ============================================================================
/// 桌面平台文件选择器实现
#[cfg(not(target_os = "android"))]
mod file_picker {
    use super::*;
    
    /// 保存剧本到JSON文件
    /// 在独立线程中运行，避免阻塞UI
    pub fn save_scenario(tx: Sender<AppCommand>, json_data: String) {
        thread::spawn(move || {
            // 使用rfd库显示保存文件对话框
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name("scenario.json")
                .save_file() 
            {
                // 异步写入文件
                if std::fs::write(&p, json_data).is_ok() {
                    let _ = tx.send(AppCommand::Log(format!("[系统] 剧本成功保存至: {}", p.display())));
                } else {
                    let _ = tx.send(AppCommand::Log("[错误] 剧本保存失败！".into()));
                }
            }
        });
    }
    
    /// 加载剧本文件
    pub fn load_scenario(tx: Sender<AppCommand>) {
        thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file() 
            {
                if let Ok(data) = std::fs::read_to_string(&p) {
                    // 解析JSON并验证格式
                    if let Ok(s) = serde_json::from_str::<Scenario>(&data) {
                        let _ = tx.send(AppCommand::ScenarioLoaded(s));
                    } else {
                        let _ = tx.send(AppCommand::Log("[错误] 解析失败，JSON格式不合法。".into()));
                    }
                } else {
                    let _ = tx.send(AppCommand::Log("[错误] 无法读取目标文件。".into()));
                }
            }
        });
    }
    
    /// 选择Spine动画文件（.atlas格式）
    pub fn pick_spine(tx: Sender<AppCommand>, slot: usize) {
        thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Atlas", &["atlas"])
                .pick_file() 
            {
                let _ = tx.send(AppCommand::RequestLoad { 
                    slot_idx: slot, 
                    path: p.display().to_string() 
                });
            }
        });
    }
    
    /// 选择背景图片
    pub fn pick_bg(tx: Sender<AppCommand>) {
        thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg"])
                .pick_file() 
            {
                let _ = tx.send(AppCommand::LoadBackground(p.display().to_string()));
            }
        });
    }
    
    /// 选择背景音乐
    pub fn pick_bgm(tx: Sender<AppCommand>) {
        thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Audio", &["mp3", "wav", "ogg"])
                .pick_file() 
            {
                let _ = tx.send(AppCommand::PlayBgm(p.display().to_string()));
            }
        });
    }
    
    /// 选择音效
    pub fn pick_se(tx: Sender<AppCommand>) {
        thread::spawn(move || {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Audio", &["mp3", "wav", "ogg"])
                .pick_file() 
            {
                let _ = tx.send(AppCommand::PlaySe(p.display().to_string()));
            }
        });
    }
}

/// Android平台文件选择器桩实现
/// 注意：当前仅为占位符，需要实现原生Android Intent调用
#[cfg(target_os = "android")]
mod file_picker {
    use super::*;
    
    pub fn save_scenario(tx: Sender<AppCommand>, _json_data: String) {
        let _ = tx.send(AppCommand::Log("[系统] 安卓端：请使用控制台指令保存".into()));
    }
    
    pub fn load_scenario(tx: Sender<AppCommand>) {
        let _ = tx.send(AppCommand::Log("[系统] 正在请求安卓原生存储访问框架 (SAF)...".into()));
        // TODO: 通过 JNI 调用 Intent.ACTION_OPEN_DOCUMENT
        // 需要集成android_activity和ndk库
    }
    
    pub fn pick_spine(tx: Sender<AppCommand>, _slot: usize) {
        let _ = tx.send(AppCommand::Log("[系统] 正在唤起安卓原生文件管理器选择 Spine...".into()));
    }
    
    pub fn pick_bg(tx: Sender<AppCommand>) {
        let _ = tx.send(AppCommand::Log("[系统] 正在唤起安卓原生文件管理器选择背景...".into()));
    }
    
    pub fn pick_bgm(tx: Sender<AppCommand>) {
        let _ = tx.send(AppCommand::Log("[系统] 正在唤起安卓原生文件管理器选择音频...".into()));
    }
    
    pub fn pick_se(tx: Sender<AppCommand>) {
        let _ = tx.send(AppCommand::Log("[系统] 正在唤起安卓原生文件管理器选择音频...".into()));
    }
}

// ============================================================================
// 核心架构组件
// ============================================================================
/// 绅士调度器：防止计算线程抢占UI和音频线程
/// 策略：保留2个CPU核心给系统和关键线程
struct AefrScheduler { 
    pool: rayon::ThreadPool  // Rayon线程池实例
}

impl AefrScheduler {
    /// 创建调度器，根据CPU核心数智能分配线程
    fn new() -> Self {
        let logic_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);  // 默认4核
        
        Self { 
            pool: rayon::ThreadPoolBuilder::new()
                .num_threads(if logic_cores > 2 { 
                    logic_cores - 2  // 保留2个核心
                } else { 
                    1  // 单核或双核系统使用1个线程
                })
                .build()
                .unwrap() 
        }
    }
    
    /// 在调度器线程池中执行并行任务
    fn run_parallel<OP>(&self, op: OP) 
    where 
        OP: FnOnce() + Send 
    { 
        self.pool.install(op);  // 在当前线程池中安装并执行
    }
}

/// 应用命令枚举：主线程与工作线程间的通信协议
/// 所有异步操作都通过此枚举进行消息传递
enum AppCommand {
    /// 更新对话内容
    Dialogue { name: String, affiliation: String, content: String },
    /// 请求加载Spine资源
    RequestLoad { slot_idx: usize, path: String },
    /// Spine资源加载成功
    LoadSuccess(usize, Box<SpineObject>, egui::ColorImage, String, Vec<String>),
    /// 移除角色
    RemoveCharacter(usize),
    /// 加载背景图片
    LoadBackground(String),
    /// 背景图片加载成功
    LoadBackgroundSuccess(egui::ColorImage),
    /// 播放背景音乐
    PlayBgm(String),
    /// 播放音效
    PlaySe(String),
    /// 音频数据准备就绪
    AudioReady(Vec<u8>, bool),  // (音频数据, 是否为BGM)
    /// 停止背景音乐
    StopBgm,
    /// 设置角色动画
    SetAnimation { slot_idx: usize, anim_name: String, loop_anim: bool },
    /// 日志消息
    Log(String),
    /// 剧本加载完成
    ScenarioLoaded(Scenario),
}

/// 音频管理器：封装rodio音频播放功能
struct AudioManager {
    _stream: rodio::OutputStream,           // 必须持有，否则流会被丢弃
    _stream_handle: rodio::OutputStreamHandle, // 音频流句柄
    bgm_sink: rodio::Sink,                  // BGM音频槽（支持循环）
    se_sink: rodio::Sink,                   // 音效音频槽（单次播放）
}

impl AudioManager {
    /// 初始化音频系统
    fn new() -> Result<Self, String> {
        // 获取默认音频输出设备
        let (_stream, stream_handle) = rodio::OutputStream::try_default()
            .map_err(|e| e.to_string())?;
        
        // 创建两个独立的音频槽：BGM和音效
        let bgm_sink = rodio::Sink::try_new(&stream_handle)
            .map_err(|e| e.to_string())?;
        let se_sink = rodio::Sink::try_new(&stream_handle)
            .map_err(|e| e.to_string())?;
        
        Ok(Self { 
            _stream, 
            _stream_handle: stream_handle, 
            bgm_sink, 
            se_sink 
        })
    }
    
    /// 播放背景音乐（自动循环）
    fn play_bgm(&self, data: Vec<u8>) {
        if let Ok(source) = rodio::Decoder::new(Cursor::new(data)) {
            self.bgm_sink.stop();  // 停止当前BGM
            self.bgm_sink.append(source.repeat_infinite());  // 无限循环
            self.bgm_sink.play();
        }
    }
    
    /// 播放音效（单次）
    fn play_se(&self, data: Vec<u8>) {
        if let Ok(source) = rodio::Decoder::new(Cursor::new(data)) { 
            self.se_sink.append(source); 
            self.se_sink.play(); 
        }
    }
    
    /// 停止背景音乐
    fn stop_bgm(&self) { 
        self.bgm_sink.stop(); 
    }
}

// ============================================================================
// Spine 2D骨骼动画对象
// ============================================================================
/// Spine动画对象：封装rusty_spine的C绑定，提供Rust友好接口
pub struct SpineObject {
    pub position: Pos2,                     // 屏幕位置
    pub scale: f32,                         // 缩放系数
    _texture: Option<TextureHandle>,        // 纹理句柄（保持所有权）
    texture_id: Option<TextureId>,          // 纹理ID（用于渲染）
    
    // 顶点缓冲区：预分配重用，实现零分配渲染
    world_vertices: Vec<f32>,

    // rusty_spine核心组件
    skeleton: Skeleton,                     // 骨骼实例
    state: AnimationState,                  // 动画状态机
    _state_data: Arc<AnimationStateData>,   // 动画状态数据（引用计数）
    _skeleton_data: Arc<rusty_spine::SkeletonData>, // 骨骼数据（引用计数）
    _atlas: Arc<Atlas>,                     // 纹理图集（引用计数）
}

// 【必要性证明 (Proof of Necessity)】
// 原因：rusty_spine 底层封装了 C 指针，默认不支持跨线程运算。
// 不可替代性：AEFR 需要使用 Rayon 在多个 CPU 核心上并行计算 5 人的 Spine 骨骼变形，以维持 144Hz 渲染。
// 安全边界：我们已经在 AEFR 状态中强制使用 `Arc<Mutex<SpineObject>>` 对对象进行包裹封装。
// 这确保了在 Rayon 线程池中，同一时刻仅有持锁的唯一线程能对其底层的 C 指针进行计算修改，物理隔绝了数据竞争。
unsafe impl Send for SpineObject {}

impl SpineObject {
    /// 异步加载Spine资源（不在GPU线程中加载纹理）
    /// 返回：(SpineObject实例, 纹理颜色数据, 页面名称, 动画列表)
    fn load_async_no_gpu(path_str: &str) -> Result<(Self, egui::ColorImage, String, Vec<String>), String> {
        // 1. 加载Atlas纹理图集
        let atlas = Arc::new(
            Atlas::new_from_file(std::path::Path::new(path_str))
                .map_err(|e| format!("Atlas Error: {}", e))?
        );
        
        // 2. 获取第一页纹理信息
        let page = atlas.pages().next().ok_or("Atlas has no pages")?;
        let page_name = page.name().to_string();
        
        // 3. 加载纹理图片
        let img_path = std::path::Path::new(path_str)
            .parent()
            .ok_or("Invalid path")?
            .join(&page_name);
        
        let img = image::open(&img_path)
            .map_err(|e| format!("Image Load Error: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width() as _, rgba.height() as _);
        let raw_pixels = rgba.into_raw();
        
        // 4. 转换为egui颜色图像（不进行预乘Alpha，在渲染时处理）
        let color_image = egui::ColorImage::from_rgba_unmultiplied([width, height], &raw_pixels);

        // 5. 查找并加载骨骼数据（支持.skel二进制和.json格式）
        let skel_path = std::path::Path::new(path_str).with_extension("skel");
        let json_path = std::path::Path::new(path_str).with_extension("json");
        
        let skeleton_data = if skel_path.exists() {
            // 二进制格式
            let skeleton_bin = SkeletonBinary::new(atlas.clone());
            Arc::new(
                skeleton_bin.read_skeleton_data_file(&skel_path)
                    .map_err(|e| format!("Binary load failed: {}", e))?
            )
        } else if json_path.exists() {
            // JSON格式
            let skeleton_json = SkeletonJson::new(atlas.clone());
            Arc::new(
                skeleton_json.read_skeleton_data_file(&json_path)
                    .map_err(|e| format!("JSON load failed: {}", e))?
            )
        } else { 
            return Err("Missing .skel or .json".into()); 
        };

        // 6. 创建动画状态机
        let state_data = Arc::new(AnimationStateData::new(skeleton_data.clone()));
        let mut state = AnimationState::new(state_data.clone());
        
        // 7. 提取所有动画名称
        let anim_names: Vec<String> = skeleton_data.animations()
            .map(|a| a.name().to_string())
            .collect();
        
        // 8. 设置默认动画（第一个动画）
        if let Some(anim) = skeleton_data.animations().next() { 
            let _ = state.set_animation(0, &anim, true);  // 循环播放
        }

        // 9. 创建骨骼实例
        let skeleton = Skeleton::new(skeleton_data.clone());

        // 10. 返回构建的SpineObject
        Ok((Self { 
            position: Pos2::ZERO, 
            scale: CHAR_BASE_SCALE, 
            _texture: None, 
            texture_id: None, 
            // 🌟 性能优化：预分配8192个顶点的缓冲区，避免运行时分配
            world_vertices: Vec::with_capacity(8192),
            skeleton, 
            state, 
            _state_data: state_data,
            _skeleton_data: skeleton_data,
            _atlas: atlas,
        }, color_image, page_name, anim_names))
    }

    /// 获取所有可用动画名称
    fn get_anim_names(&self) -> Vec<String> { 
        self._skeleton_data.animations()
            .map(|a| a.name().to_string())
            .collect() 
    }
    
    /// 按名称设置当前动画
    /// 返回：是否成功找到并设置动画
    fn set_animation_by_name(&mut self, anim_name: &str, loop_anim: bool) -> bool {
        if let Some(anim) = self._skeleton_data.animations()
            .find(|a| a.name() == anim_name) 
        {
            let _ = self.state.set_animation(0, &anim, loop_anim); 
            true
        } else { 
            false  // 动画名称不存在
        }
    }
    
    /// 并行更新：计算骨骼动画状态
    /// 在Rayon线程池中调用，需保证线程安全
    fn update_parallel(&mut self, dt: f32) {
        let dt = dt.min(MAX_DT);  // 限制最大时间步，防止卡顿导致的动画跳跃
        
        // Spine动画更新流水线
        self.state.update(dt);                     // 更新动画状态机
        self.skeleton.set_to_setup_pose();         // 重置到初始姿势
        let _ = self.state.apply(&mut self.skeleton); // 应用当前动画
        self.skeleton.update_world_transform();    // 更新世界变换
        self.skeleton.update_cache();              // 更新渲染缓存
    }
    
    /// 渲染Spine动画到UI
    /// 在UI线程中调用，将动画转换为egui Mesh
    fn paint(&mut self, ui: &mut egui::Ui) {
        // 检查纹理是否已加载
        let tex_id = match self.texture_id { 
            Some(id) => id, 
            None => return  // 纹理未就绪，跳过渲染
        };
        
        // 创建纹理Mesh
        let mut mesh = Mesh::with_texture(tex_id);
        
        // 遍历所有绘制槽位
        for slot in self.skeleton.draw_order() {
            let attachment = match slot.attachment() { 
                Some(a) => a, 
                None => continue  // 槽位无附件，跳过
            };
            
            // 处理区域附件（简单四边形）
            if let Some(region) = attachment.as_region() {
                unsafe {
                    // 确保顶点缓冲区足够大
                    if self.world_vertices.len() < 8 { 
                        self.world_vertices.resize(8, 0.0); 
                    }
                    
                    // 计算世界坐标顶点
                    region.compute_world_vertices(&slot.bone(), &mut self.world_vertices, 0, 2);
                    
                    // 将顶点推送到Mesh
                    self.push_to_mesh(
                        &mut mesh, 
                        &self.world_vertices[0..8],  // 8个浮点数 = 4个顶点 × (x,y)
                        &region.uvs(),              // UV坐标
                        &[0, 1, 2, 2, 3, 0],        // 三角形索引（两个三角形组成四边形）
                        &*slot,                      // 槽位引用
                        region.color()              // 附件颜色
                    );
                }
            } 
            // 处理网格附件（复杂网格）
            else if let Some(mesh_att) = attachment.as_mesh() {
                unsafe {
                    let len = mesh_att.world_vertices_length() as usize;
                    
                    // 确保顶点缓冲区足够大
                    if self.world_vertices.len() < len { 
                        self.world_vertices.resize(len, 0.0); 
                    }
                    
                    // 计算世界坐标顶点
                    mesh_att.compute_world_vertices(&*slot, 0, len as i32, &mut self.world_vertices, 0, 2);
                    
                    // 从C指针获取UV和三角形数据
                    let uvs = std::slice::from_raw_parts(mesh_att.uvs(), len);
                    let tris = std::slice::from_raw_parts(
                        mesh_att.triangles(), 
                        mesh_att.triangles_count() as usize
                    );
                    
                    // 将顶点推送到Mesh
                    self.push_to_mesh(
                        &mut mesh, 
                        &self.world_vertices[0..len], 
                        uvs, 
                        tris, 
                        &*slot, 
                        mesh_att.color()
                    );
                }
            }
        }
        
        // 将Mesh添加到UI绘制器
        ui.painter().add(Shape::mesh(mesh));
    }
    
    /// 将顶点数据推送到egui Mesh
    /// 处理颜色混合、坐标变换和UV映射
    fn push_to_mesh(
        &self, 
        mesh: &mut Mesh, 
        w_v: &[f32],      // 世界坐标顶点 [x1, y1, x2, y2, ...]
        uvs: &[f32],      // UV坐标 [u1, v1, u2, v2, ...]
        tris: &[u16],     // 三角形索引
        slot: &Slot,      // Spine槽位
        att_c: rusty_spine::Color  // 附件颜色
    ) {
        // 1. 颜色计算：槽位颜色 × 附件颜色
        let s_c = slot.color();      // 槽位颜色
        let a = s_c.a * att_c.a;     // 最终Alpha（预乘）
        let r = s_c.r * att_c.r * a; // 预乘红色
        let g = s_c.g * att_c.g * a; // 预乘绿色
        let b = s_c.b * att_c.b * a; // 预乘蓝色
        
        // 2. 特殊混合模式处理：Additive模式需要Alpha为0
        let final_a = match slot.data().blend_mode() {
            rusty_spine::BlendMode::Additive => 0.0,  // 加色混合
            _ => a,                                   // 正常/乘色混合
        };
        
        // 3. 转换为egui颜色格式（预乘RGBA）
        let color = Color32::from_rgba_premultiplied(
            (r * 255.0).clamp(0.0, 255.0) as u8, 
            (g * 255.0).clamp(0.0, 255.0) as u8,
            (b * 255.0).clamp(0.0, 255.0) as u8, 
            (final_a * 255.0).clamp(0.0, 255.0) as u8,
        );
        
        // 4. 计算顶点数量
        let count = usize::min(uvs.len() / 2, w_v.len() / 2);
        let idx_offset = mesh.vertices.len() as u32;  // 当前Mesh的顶点偏移
        
        // 5. 添加顶点
        for i in 0..count {
            // 应用缩放和位置变换
            let pos = Pos2::new(
                w_v[i*2] * self.scale + self.position.x,      // X坐标
                -w_v[i*2+1] * self.scale + self.position.y    // Y坐标（翻转Y轴）
            );
            
            // 添加顶点到Mesh
            mesh.vertices.push(Vertex { 
                pos, 
                uv: Pos2::new(uvs[i*2], uvs[i*2+1]),  // UV坐标
                color 
            });
        }
        
        // 6. 添加三角形索引
        for &idx in tris { 
            mesh.indices.push(idx_offset + idx as u32); 
        }
    }
}

// ============================================================================
// 主应用程序逻辑
// ============================================================================
/// 主应用程序状态
struct AefrApp {
    // 系统组件
    scheduler: AefrScheduler,      // 并行调度器
    audio_manager: Option<AudioManager>, // 音频管理器（可选，可能初始化失败）
    
    // 剧本状态
    scenario: Scenario,            // 当前剧本
    current_scene_idx: usize,      // 当前场景索引
    
    // 对话状态
    target_chars: Vec<char>,       // 目标文本字符数组
    visible_count: usize,          // 当前可见字符数
    type_timer: f32,               // 打字机计时器
    
    // UI状态
    is_auto_enabled: bool,         // 自动播放模式
    show_dialogue: bool,           // 显示对话框
    console_open: bool,            // 控制台窗口状态
    selected_slot: usize,          // 当前选中的角色槽位
    console_input: String,         // 控制台输入
    console_logs: Vec<String>,     // 控制台日志
    
    // 动画预览
    show_anim_preview: bool,       // 显示动画预览窗口
    preview_anim_idx: usize,       // 预览动画索引
    
    // 游戏对象
    // 🌟 关键：使用Arc<Mutex>包装SpineObject，实现线程安全共享
    characters: Vec<Option<Arc<Mutex<SpineObject>>>>, // 5个角色槽位
    background: Option<TextureHandle>, // 背景纹理
    
    // 线程通信
    tx: Sender<AppCommand>,        // 命令发送端
    rx: Receiver<AppCommand>,      // 命令接收端
}

impl AefrApp {
    /// 创建应用程序实例
    fn new(cc: &eframe::CreationContext) -> Self {
        // 1. 设置嵌入式字体
        setup_embedded_font(&cc.egui_ctx);
        
        // 2. 安装图片加载器
        egui_extras::install_image_loaders(&cc.egui_ctx);
        
        // 3. 创建线程通信通道
        let (tx, rx) = channel();
        
        // 4. 初始化音频系统（允许失败）
        let audio_manager = match AudioManager::new() {
            Ok(mgr) => Some(mgr),
            Err(e) => {
                // 记录错误但不中断程序
                let _ = tx.send(AppCommand::Log(
                    format!("[警告] 音频系统初始化失败 (无声卡/独占): {}", e)
                ));
                None
            }
        };
        
        // 5. 创建初始场景
        let startup_text = "GNU:AEFR 已启动！\n正在等待指令......";
        let mut first_scene = Scene::default();
        first_scene.speaker_name = "OxidizedSchale".into();
        first_scene.speaker_aff = "The Executive Committee of GNU:AEFR".into();
        first_scene.dialogue_content = startup_text.into();

        // 6. 返回应用实例
        Self {
            scheduler: AefrScheduler::new(),
            is_auto_enabled: true, 
            show_dialogue: true,
            scenario: Scenario { scenes: vec![first_scene] },
            current_scene_idx: 0,
            target_chars: startup_text.chars().collect(), 
            visible_count: 0, 
            type_timer: 0.0,
            console_open: false,
            selected_slot: 0,
            console_input: String::new(),
            console_logs: vec!["[系统] 编辑器就绪。".into()],
            show_anim_preview: false,
            preview_anim_idx: 0,
            // 初始化5个空角色槽位
            characters: (0..5).map(|_| None).collect(),
            background: None,
            audio_manager,
            tx, rx,
        }
    }

    /// 同步当前场景到UI状态
    /// 在场景切换时调用，重置对话状态
    fn sync_scene_to_ui(&mut self) {
        if let Some(scene) = self.scenario.scenes.get(self.current_scene_idx) {
            self.target_chars = scene.dialogue_content.chars().collect();
            // 🌟 修复打字机残影：切幕时必须归零
            self.visible_count = 0;
            self.type_timer = 0.0;
        }
    }

    /// 解析并执行控制台命令
    /// 支持的命令格式：
    /// - load <槽位> <路径>    # 加载Spine角色
    /// - anim <槽位> <动画名> [循环] # 设置动画
    /// - bgm <路径>           # 播放背景音乐
    /// - se <路径>            # 播放音效
    /// - talk 名称|所属|内容  # 发送对话
    /// - bg <路径>            # 设置背景
    fn parse_and_send_command(&mut self, input: &str) {
        let input_trimmed = input.trim();
        if input_trimmed.is_empty() { return; }
        
        // 记录命令到日志
        self.console_logs.push(format!("> {}", input_trimmed));
        
        let tx = self.tx.clone();
        let cmd_lower = input_trimmed.to_lowercase();

        // 命令分发
        if cmd_lower.starts_with("load ") {
            // 格式: load 0 "path/to/file.atlas"
            let parts: Vec<&str> = input_trimmed.splitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(idx) = parts[0][5..].trim().parse::<usize>() {
                   let _ = tx.send(AppCommand::RequestLoad { 
                       slot_idx: idx, 
                       path: parts[1].replace("\"", "") 
                   });
                }
            }
        } else if cmd_lower.starts_with("anim ") {
            // 格式: anim 0 idle true
            let parts: Vec<&str> = input_trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(idx) = parts[1].parse::<usize>() {
                    let anim_name = parts[2].to_string();
                    let loop_anim = parts.get(3).map_or(true, |s| s.to_lowercase() == "true");
                    let _ = tx.send(AppCommand::SetAnimation { 
                        slot_idx: idx, 
                        anim_name, 
                        loop_anim 
                    });
                }
            }
        } else if cmd_lower.starts_with("bgm ") {
             // 格式: bgm "path/to/bgm.mp3"
             let _ = tx.send(AppCommand::PlayBgm(
                 input_trimmed[4..].trim().replace("\"", "")
             ));
        } else if cmd_lower.starts_with("se ") {
             // 格式: se "path/to/se.wav"
             let _ = tx.send(AppCommand::PlaySe(
                 input_trimmed[3..].trim().replace("\"", "")
             ));
        } else if cmd_lower == "stop" {
             // 格式: stop
             let _ = tx.send(AppCommand::StopBgm);
        } else if cmd_lower.starts_with("talk ") {
            // 格式: talk 名称|所属|内容
            let p: Vec<&str> = input_trimmed[5..].split('|').collect();
            if p.len() == 3 {
                let _ = tx.send(AppCommand::Dialogue { 
                    name: p[0].into(), 
                    affiliation: p[1].into(), 
                    content: p[2].into() 
                });
            }
        } else if cmd_lower.starts_with("bg ") {
            // 格式: bg "path/to/bg.png"
            let _ = tx.send(AppCommand::LoadBackground(
                input_trimmed[3..].trim().replace("\"", "")
            ));
        }
    }

    /// 处理异步事件（命令模式）
    /// 从通道接收并处理工作线程发送的命令
    fn handle_async_events(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                // 更新对话
                AppCommand::Dialogue { name, affiliation, content } => {
                    let scene = &mut self.scenario.scenes[self.current_scene_idx];
                    scene.speaker_name = name; 
                    scene.speaker_aff = affiliation; 
                    scene.dialogue_content = content;
                    self.sync_scene_to_ui();  // 立即应用
                }
                
                // 日志消息
                AppCommand::Log(msg) => self.console_logs.push(msg),
                
                // 请求加载Spine资源
                AppCommand::RequestLoad { slot_idx, path } => {
                    let tx_cb = self.tx.clone(); 
                    self.console_logs.push(format!("[解析] {}", path));
                    
                    // 在工作线程中加载（避免阻塞UI）
                    let path_clone = path.clone();
                    thread::spawn(move || {
                        match SpineObject::load_async_no_gpu(&path_clone) {
                            Ok((obj, img, page, anims)) => { 
                                let _ = tx_cb.send(AppCommand::LoadSuccess(
                                    slot_idx, Box::new(obj), img, page, anims
                                )); 
                            },
                            Err(e) => { 
                                let _ = tx_cb.send(AppCommand::Log(
                                    format!("[错误] {}", e)
                                )); 
                            }
                        }
                    });
                }
                
                // Spine资源加载成功
                AppCommand::LoadSuccess(idx, obj, color_image, page_name, anims) => {
                    if let Some(slot) = self.characters.get_mut(idx) {
                        let mut loaded = *obj;
                        
                        // 在主线程中加载纹理到GPU
                        let handle = ctx.load_texture(
                            page_name, 
                            color_image, 
                            egui::TextureOptions::LINEAR
                        );
                        
                        loaded.texture_id = Some(handle.id()); 
                        loaded._texture = Some(handle);
                        
                        // 🌟 用Arc<Mutex>包装，确保线程安全
                        *slot = Some(Arc::new(Mutex::new(loaded)));
                    }
                }
                
                // 移除角色
                AppCommand::RemoveCharacter(idx) => { 
                    self.characters[idx] = None; 
                }
                
                // 加载背景图片
                AppCommand::LoadBackground(path) => {
                    let tx_cb = self.tx.clone();
                    let path_clone = path.clone();
                    
                    thread::spawn(move || {
                        if let Ok(img) = image::open(&path_clone) {
                            let c_img = egui::ColorImage::from_rgba_unmultiplied(
                                [img.width() as _, img.height() as _], 
                                img.to_rgba8().as_raw()
                            );
                            let _ = tx_cb.send(AppCommand::LoadBackgroundSuccess(c_img));
                        }
                    });
                    
                    self.scenario.scenes[self.current_scene_idx].bg_path = Some(path);
                }
                
                // 背景图片加载成功
                AppCommand::LoadBackgroundSuccess(c_img) => {
                    self.background = Some(ctx.load_texture(
                        "bg", 
                        c_img, 
                        egui::TextureOptions::LINEAR
                    ));
                }
                
                // 设置动画
                AppCommand::SetAnimation { slot_idx, anim_name, loop_anim } => {
                     if let Some(Some(char_arc)) = self.characters.get(slot_idx) {
                         if let Ok(mut char) = char_arc.lock() {
                             let _ = char.set_animation_by_name(&anim_name, loop_anim);
                         }
                     }
                }
                
                // 播放BGM
                AppCommand::PlayBgm(path) => {
                    let tx_cb = self.tx.clone();
                    let path_clone = path.clone();
                    
                    thread::spawn(move || { 
                        if let Ok(d) = std::fs::read(&path_clone) { 
                            let _ = tx_cb.send(AppCommand::AudioReady(d, true)); 
                        } 
                    });
                    
                    self.scenario.scenes[self.current_scene_idx].bgm_path = Some(path);
                }
                
                // 播放音效
                AppCommand::PlaySe(path) => {
                    let tx_cb = self.tx.clone();
                    let path_clone = path.clone();
                    
                    thread::spawn(move || { 
                        if let Ok(d) = std::fs::read(&path_clone) { 
                            let _ = tx_cb.send(AppCommand::AudioReady(d, false)); 
                        } 
                    });
                }
                
                // 音频数据就绪
                AppCommand::AudioReady(data, is_bgm) => {
                    if let Some(mgr) = &self.audio_manager { 
                        if is_bgm { 
                            mgr.play_bgm(data);  // 播放BGM
                        } else { 
                            mgr.play_se(data);   // 播放音效
                        } 
                    }
                }
                
                // 停止BGM
                AppCommand::StopBgm => { 
                    if let Some(mgr) = &self.audio_manager { 
                        mgr.stop_bgm(); 
                    } 
                }
                
                // 剧本加载完成
                AppCommand::ScenarioLoaded(s) => {
                    self.scenario = s;
                    self.current_scene_idx = 0;
                    self.sync_scene_to_ui();
                    self.visible_count = self.target_chars.len();  // 立即显示全部文本
                    self.console_logs.push("[系统] 剧本读取并应用成功。".into());
                }
            }
        }
    }
}

// ============================================================================
// 主应用循环实现
// ============================================================================
impl eframe::App for AefrApp {
    /// 主更新循环，每帧调用
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. 处理异步事件
        self.handle_async_events(ctx);
        
        // 2. 获取帧时间
        let dt = ctx.input(|i| i.stable_dt);
        
        // 3. 更新打字机效果
        if self.show_dialogue && self.visible_count < self.target_chars.len() {
            self.type_timer += dt;
            
            // 🌟 解决计时器精度漂移：使用减法而非归零
            while self.type_timer >= TYPEWRITER_INTERVAL {
                self.visible_count += 1; 
                self.type_timer -= TYPEWRITER_INTERVAL;
            }
        }

        // 4. 计算屏幕缩放
        let screen = ctx.screen_rect();
        let scale_factor = screen.height() / BASE_HEIGHT;
        
        // 5. 更新角色位置
        for (i, slot) in self.characters.iter().enumerate() {
            if let Some(char_arc) = slot {
                if let Ok(mut char) = char_arc.lock() {
                    // 应用DPI缩放
                    char.scale = CHAR_BASE_SCALE * scale_factor;
                    
                    // 计算水平位置（等距分布）
                    let x_percent = CHAR_X_START_PERCENT + (i as f32 * CHAR_X_STEP_PERCENT);
                    char.position = Pos2::new(
                        screen.width() * x_percent, 
                        screen.bottom() + (30.0 * scale_factor)  // 底部留白
                    );
                }
            }
        }

        // 6. 🌟 并行更新所有角色的骨骼动画
        // 使用调度器确保不占用UI/音频线程资源
        self.scheduler.run_parallel(|| {
            // 使用Rayon并行迭代器
            self.characters.par_iter().for_each(|slot| {
                if let Some(char_arc) = slot { 
                    // 获取Mutex锁（线程安全）
                    if let Ok(mut char) = char_arc.lock() {
                        char.update_parallel(dt);  // 并行计算骨骼变形
                    }
                }
            });
        });

        // 7. 主绘制区域
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))  // 黑色背景
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                
                // 7.1 绘制背景
                if let Some(bg) = &self.background {
                    let img_size = bg.size_vec2();
                    let scale = (rect.width() / img_size.x).max(rect.height() / img_size.y);
                    ui.painter().image(
                        bg.id(), 
                        Rect::from_center_size(rect.center(), img_size * scale), 
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),  // 完整纹理
                        Color32::WHITE
                    );
                }
                
                // 7.2 绘制所有角色
                for char_arc in self.characters.iter().flatten() { 
                    if let Ok(mut char) = char_arc.lock() {
                        char.paint(ui);  // 渲染Spine动画
                    }
                }
                
                // 7.3 绘制右上角按钮
                draw_top_right_buttons(ui, rect, &mut self.is_auto_enabled);
                
                // 7.4 绘制对话框
                if self.show_dialogue {
                    let committed_text: String = self.target_chars.iter().collect();
                    if !committed_text.trim().is_empty() {
                        let scene = &self.scenario.scenes[self.current_scene_idx];
                        let text: String = self.target_chars.iter()
                            .take(self.visible_count)
                            .collect();
                        
                        // 绘制《蔚蓝档案》风格对话框
                        if draw_ba_dialogue(
                            ui, rect, 
                            &scene.speaker_name, 
                            &scene.speaker_aff, 
                            &text, 
                            self.visible_count >= self.target_chars.len()  // 是否显示完成指示器
                        ) { 
                            // 点击对话框快速完成打字效果
                            self.visible_count = self.target_chars.len(); 
                        }
                    }
                }
                
                // 7.5 控制台按钮
                if ui.put(
                    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(60.0, 30.0)), 
                    egui::Button::new("CMD")
                ).clicked() { 
                    self.console_open = !self.console_open;  // 切换控制台显示
                }
                
                // 7.6 创作者面板（控制台）
                if self.console_open { 
                    draw_creator_panel(ctx, self); 
                }
            });
        
        // 8. 请求下一帧重绘
        ctx.request_repaint();
    }
}

// ============================================================================
// UI 组件函数
// ============================================================================
/// 绘制右上角控制按钮（AUTO/MENU）
fn draw_top_right_buttons(ui: &mut egui::Ui, screen: Rect, is_auto: &mut bool) {
    let (btn_w, btn_h, margin) = (90.0, 32.0, 20.0);
    
    // AUTO按钮
    let auto_rect = Rect::from_min_size(
        Pos2::new(screen.right() - btn_w * 2.0 - margin - 10.0, margin), 
        Vec2::new(btn_w, btn_h)
    );
    
    // 点击切换AUTO状态
    if ui.allocate_rect(auto_rect, egui::Sense::click()).clicked() { 
        *is_auto = !*is_auto; 
    }
    
    // 绘制AUTO按钮背景（金色表示激活）
    ui.painter().rect_filled(
        auto_rect, 
        4.0, 
        if *is_auto { 
            Color32::from_rgb(255, 215, 0)  // 金色
        } else { 
            Color32::WHITE  // 白色
        }
    );
    
    // 绘制AUTO文字
    ui.painter().text(
        auto_rect.center(), 
        egui::Align2::CENTER_CENTER, 
        "AUTO", 
        egui::FontId::proportional(18.0), 
        Color32::from_rgb(20, 30, 50)  // 深蓝色文字
    );
    
    // MENU按钮（装饰性）
    ui.painter().rect_filled(
        Rect::from_min_size(
            Pos2::new(screen.right() - btn_w - margin, margin), 
            Vec2::new(btn_w, btn_h)
        ), 
        4.0, 
        Color32::WHITE
    );
    
    ui.painter().text(
        Pos2::new(screen.right() - btn_w / 2.0 - margin, margin + btn_h / 2.0), 
        egui::Align2::CENTER_CENTER, 
        "MENU", 
        egui::FontId::proportional(18.0), 
        Color32::from_rgb(20, 30, 50)
    );
}

/// 绘制《蔚蓝档案》风格对话框
/// 返回：是否被点击（用于快速完成打字效果）
fn draw_ba_dialogue(
    ui: &mut egui::Ui, 
    screen: Rect, 
    name: &str, 
    affiliation: &str, 
    content: &str, 
    is_finished: bool
) -> bool {
    // 1. 计算对话框尺寸
    let box_h = screen.height() * DIALOGUE_BOX_RATIO;
    let box_rect = Rect::from_min_max(
        Pos2::new(screen.left(), screen.bottom() - box_h), 
        screen.max
    );
    let line_y = box_rect.top() + (box_h * 0.30);  // 分隔线Y坐标
    
    // 2. 绘制半透明深蓝色背景
    let dark_blue_opaque = Color32::from_rgba_unmultiplied(12, 18, 28, 252);
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(screen.left(), line_y), screen.max), 
        0.0, 
        dark_blue_opaque
    );
    
    // 3. 绘制顶部渐变过渡
    let gradient_rect = Rect::from_min_max(box_rect.left_top(), Pos2::new(screen.right(), line_y));
    let mut mesh = Mesh::default();
    
    let color_bottom = Color32::from_rgba_unmultiplied(12, 18, 28, 245);  // 底部较实
    let color_top = Color32::from_rgba_unmultiplied(12, 18, 28, 0);        // 顶部透明
    
    // 渐变四边形顶点
    mesh.vertices.push(Vertex { pos: gradient_rect.left_top(), uv: Pos2::ZERO, color: color_top });
    mesh.vertices.push(Vertex { pos: gradient_rect.right_top(), uv: Pos2::ZERO, color: color_top });
    mesh.vertices.push(Vertex { pos: gradient_rect.right_bottom(), uv: Pos2::ZERO, color: color_bottom });
    mesh.vertices.push(Vertex { pos: gradient_rect.left_bottom(), uv: Pos2::ZERO, color: color_bottom });
    
    // 两个三角形组成四边形
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(Shape::mesh(mesh));

    // 4. 对话框点击区域
    let resp = ui.allocate_rect(box_rect, egui::Sense::click());
    
    // 5. 绘制分隔线
    let pad_x = (screen.width() * 0.08).max(100.0);
    ui.painter().line_segment(
        [Pos2::new(pad_x, line_y), Pos2::new(screen.right() - pad_x, line_y)], 
        Stroke::new(1.5, Color32::from_rgb(100, 120, 150))
    );

    // 6. 绘制说话者名称和所属
    if !name.is_empty() {
        let n_size = (box_h * 0.16).clamp(22.0, 30.0);
        let n_gal = ui.painter().layout_no_wrap(
            name.into(), 
            egui::FontId::proportional(n_size), 
            Color32::WHITE
        );
        
        let n_width = n_gal.rect.width();
        let n_height = n_gal.rect.height();
        let margin_bottom = 4.0;
        let n_pos = Pos2::new(box_rect.left() + pad_x, line_y - n_height - margin_bottom);

        // 绘制所属（如果存在）
        if !affiliation.is_empty() {
            let aff_size = n_size * 0.75;
            let aff_gal = ui.painter().layout_no_wrap(
                affiliation.into(), 
                egui::FontId::proportional(aff_size), 
                Color32::from_rgb(100, 200, 255)  // 青色
            );
            
            let aff_height = aff_gal.rect.height();
            let visual_compensation = 0.0;  // 视觉微调
            let y_offset = n_height - aff_height + visual_compensation; 
            
            // 绘制名称
            ui.painter().galley(n_pos, n_gal.clone(), Color32::WHITE);
            
            // 绘制所属（在名称右侧）
            ui.painter().galley(
                n_pos + Vec2::new(n_width + 15.0, y_offset), 
                aff_gal, 
                Color32::from_rgb(100, 200, 255)
            );
        } else {
            ui.painter().galley(n_pos, n_gal, Color32::WHITE);
        }
    }
    
    // 7. 绘制对话内容
    ui.painter().text(
        Pos2::new(box_rect.left() + pad_x, line_y + box_h * 0.05), 
        egui::Align2::LEFT_TOP, 
        content, 
        egui::FontId::proportional((box_h * 0.13).clamp(18.0, 25.0)), 
        Color32::WHITE
    );
    
    // 8. 绘制完成指示器（闪烁三角形）
    if is_finished {
        let tri_center = Pos2::new(
            screen.right() - pad_x, 
            screen.bottom() - (box_h * 0.15) + (ui.input(|i| i.time) * 3.0).sin() as f32 * 3.0
        );
        
        let ts = box_h * 0.04;  // 三角形大小
        ui.painter().add(Shape::convex_polygon(
            vec![
                tri_center + Vec2::new(-ts, -ts),  // 左上
                tri_center + Vec2::new(ts, -ts),   // 右上
                tri_center + Vec2::new(0.0, ts),   // 下
            ], 
            Color32::from_rgb(0, 180, 255),  // 蓝色三角形
            Stroke::NONE
        ));
    }
    
    // 返回是否被点击
    resp.clicked()
}

/// 绘制创作者面板（主控制台）
fn draw_creator_panel(ctx: &egui::Context, app: &mut AefrApp) {
    let mut cmd_to_send = None;  // 待发送命令
    
    egui::Window::new("创作者面板 - GNU:AEFR")
        .default_size([500.0, 600.0])
        .show(ctx, |ui| {
            // 1. 剧本幕数管理
            ui.heading("🎬 剧本幕数管理");
            ui.horizontal(|ui| {
                // 上一幕按钮
                if ui.button("⬅ 上一幕").clicked() && app.current_scene_idx > 0 {
                    app.current_scene_idx -= 1; 
                    app.sync_scene_to_ui(); 
                    app.visible_count = app.target_chars.len();  // 立即显示全文
                }
                
                // 当前幕数显示
                ui.label(format!(" 第 {} / {} 幕 ", 
                    app.current_scene_idx + 1, 
                    app.scenario.scenes.len()
                ));
                
                // 下一幕按钮
                if ui.button("下一幕 ➡").clicked() && 
                   app.current_scene_idx < app.scenario.scenes.len() - 1 
                {
                    app.current_scene_idx += 1; 
                    app.sync_scene_to_ui(); 
                    app.visible_count = app.target_chars.len();
                }
                
                ui.separator();
                
                // 增加一幕
                if ui.button("➕ 增加一幕").clicked() {
                    let mut new_scene = app.scenario.scenes[app.current_scene_idx].clone();
                    new_scene.dialogue_content.clear();  // 清空对话
                    app.scenario.scenes.insert(app.current_scene_idx + 1, new_scene);
                    app.current_scene_idx += 1; 
                    app.sync_scene_to_ui();
                }
                
                // 删除当前幕
                if ui.button("❌ 删除").clicked() && app.scenario.scenes.len() > 1 {
                    app.scenario.scenes.remove(app.current_scene_idx);
                    app.current_scene_idx = app.current_scene_idx.min(app.scenario.scenes.len() - 1);
                    app.sync_scene_to_ui();
                }
            });
            
            // 幕数跳转输入
            ui.horizontal(|ui| {
                ui.label("跳转:");
                let mut jump = app.current_scene_idx + 1;
                let len = app.scenario.scenes.len();
                
                if ui.add(egui::DragValue::new(&mut jump).clamp_range(1..=len)).changed() {
                    app.current_scene_idx = jump - 1; 
                    app.sync_scene_to_ui(); 
                    app.visible_count = app.target_chars.len();
                }
            });

            ui.separator();
            
            // 2. 剧本文件操作
            ui.horizontal(|ui| {
                if ui.button("💾 保存剧本").clicked() {
                    if let Ok(json_data) = serde_json::to_string_pretty(&app.scenario) { 
                        file_picker::save_scenario(app.tx.clone(), json_data); 
                    }
                }
                if ui.button("📂 重载剧本").clicked() { 
                    file_picker::load_scenario(app.tx.clone()); 
                }
            });

            ui.separator();
            
            // 3. 资源管理
            ui.heading("📂 资源管理");
            ui.horizontal(|ui| {
                ui.label("槽位:");
                // 5个角色槽位选择按钮
                for i in 0..5 { 
                    if ui.radio_value(&mut app.selected_slot, i, format!("[{}]", i)).clicked() { 
                        app.preview_anim_idx = 0;  // 重置预览索引
                    } 
                }
            });
            
            ui.horizontal(|ui| {
                // Spine导入
                if ui.button("📥 导入 Spine 立绘").clicked() { 
                    file_picker::pick_spine(app.tx.clone(), app.selected_slot); 
                }
                
                // 背景导入
                if ui.button("🖼 背景").clicked() { 
                    file_picker::pick_bg(app.tx.clone()); 
                }
                
                // 立绘移除（红色按钮）
                if ui.add(egui::Button::new("🗑 立绘移除")
                    .fill(Color32::from_rgb(150, 40, 40))).clicked() 
                { 
                    cmd_to_send = Some(AppCommand::RemoveCharacter(app.selected_slot)); 
                }
                
                // 动画预览
                if ui.button("🏃 动作预览").clicked() { 
                    app.show_anim_preview = true; 
                }
            });

            ui.separator();
            
            // 4. 音频管理
            ui.heading("🎵 音频管理");
            ui.horizontal(|ui| {
                if ui.button("🔁 导入音乐(循环)").clicked() { 
                    file_picker::pick_bgm(app.tx.clone()); 
                }
                if ui.button("🔊 音效").clicked() { 
                    file_picker::pick_se(app.tx.clone()); 
                }
                if ui.add(egui::Button::new("⏹ 停止音乐")
                    .fill(Color32::from_rgb(150, 40, 40))).clicked() 
                { 
                    cmd_to_send = Some(AppCommand::StopBgm); 
                }
            });

            ui.separator();
            
            // 5. 对话编辑
            ui.heading("💬 对话 (当前幕)");
            let scene = &mut app.scenario.scenes[app.current_scene_idx];
            
            ui.horizontal(|ui| {
                ui.label("名称:");
                ui.add(egui::TextEdit::singleline(&mut scene.speaker_name)
                    .desired_width(80.0));
                
                ui.label("所属:");
                ui.add(egui::TextEdit::singleline(&mut scene.speaker_aff)
                    .desired_width(80.0));
            });
            
            // 多行对话编辑
            ui.add(egui::TextEdit::multiline(&mut scene.dialogue_content)
                .desired_width(f32::INFINITY));
            
            if ui.button("▶ 发送对话 (TALK)").clicked() { 
                app.sync_scene_to_ui();  // 立即应用编辑
            }

            ui.separator();
            
            // 6. 控制台命令行
            ui.horizontal(|ui| {
                let res = ui.add(egui::TextEdit::singleline(&mut app.console_input)
                    .hint_text("CMD..."));
                
                // 回车或点击发送
                if ui.button("发送").clicked() || 
                   (res.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))) 
                {
                    let input = app.console_input.clone(); 
                    app.parse_and_send_command(&input); 
                    app.console_input.clear(); 
                    res.request_focus();  // 保持焦点
                }
            });
            
            // 7. 日志显示（自动滚动到底部）
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(60.0)
                .show(ui, |ui| { 
                    for log in &app.console_logs { 
                        ui.label(log); 
                    } 
                });
        });

    // 8. 动画预览窗口
    if app.show_anim_preview {
        egui::Window::new("动作")
            .open(&mut app.show_anim_preview)
            .show(ctx, |ui| {
                 if let Some(Some(char_arc)) = app.characters.get(app.selected_slot) {
                    if let Ok(char) = char_arc.lock() {
                        let anims = char.get_anim_names();
                        
                        if !anims.is_empty() {
                            // 确保索引有效
                            if app.preview_anim_idx >= anims.len() { 
                                app.preview_anim_idx = 0; 
                            }
                            
                            // 显示当前动画名称
                            ui.heading(&anims[app.preview_anim_idx]);
                            
                            // 左右切换按钮
                            ui.horizontal(|ui| {
                                // 上一个动画
                                if ui.button("⬅").clicked() { 
                                    app.preview_anim_idx = (app.preview_anim_idx + anims.len() - 1) % anims.len(); 
                                    cmd_to_send = Some(AppCommand::SetAnimation { 
                                        slot_idx: app.selected_slot, 
                                        anim_name: anims[app.preview_anim_idx].clone(), 
                                        loop_anim: true 
                                    }); 
                                }
                                
                                // 下一个动画
                                if ui.button("➡").clicked() { 
                                    app.preview_anim_idx = (app.preview_anim_idx + 1) % anims.len(); 
                                    cmd_to_send = Some(AppCommand::SetAnimation { 
                                        slot_idx: app.selected_slot, 
                                        anim_name: anims[app.preview_anim_idx].clone(), 
                                        loop_anim: true 
                                    }); 
                                }
                            });
                        }
                    }
                 }
            });
    }
    
    // 9. 发送待处理命令
    if let Some(cmd) = cmd_to_send { 
        let _ = app.tx.send(cmd); 
    }
}

/// 嵌入式字体数据
const FONT_DATA: &[u8] = include_bytes!("SarasaTermSCNerd-Regular.ttf");

/// 设置嵌入式字体
fn setup_embedded_font(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    
    // 注册字体数据
    fonts.font_data.insert(
        "sarasa_font".to_owned(), 
        FontData::from_static(FONT_DATA)  // 从二进制数据加载
    );
    
    // 设置为默认比例字体
    fonts.families.get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "sarasa_font".to_owned());
    
    // 设置为默认等宽字体
    fonts.families.get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "sarasa_font".to_owned());
    
    // 应用字体设置
    ctx.set_fonts(fonts);
}
