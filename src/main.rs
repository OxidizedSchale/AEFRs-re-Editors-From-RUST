/*
 * ============================================================================
 * Project: PARE Studio (PARE's Accelerated Runtime Elimination)
 * Version: 0.2.0-Alpha "Pure C Strike"
 * GitHub: https://github.com/OxidizedSchale/PARE-s-Accelerated-Runtime-Elimination
 * * [核心架构说明 / Architectural Vision]
 * PARE 不仅仅是一个打包工具，它是一个“运行时消除器”。
 * * 1. 影子变量 (Shadow Variables): 
 * 在 C 层面为 Python 变量创建原生映射。计算时优先在 CPU 寄存器和原生 C 栈中进行。
 * * 2. 纯 C 循环 (Pure C Loops): 
 * 通过静态分析识别 range() 模式，直接生成原生 for(long long...) 语句，
 * 绕过 Python 迭代器的对象包装开销。
 * * 3. 延迟同步 (Lazy Synchronization): 
 * 仅在必须调用 Python 侧功能（如 print 或 import）前，才将 C 状态“刷”回 
 * CPython VM 的 globals 字典中。
 * * 版权所有 (C) 2026 OxidizedSchale & PARE Contributors
 * 许可证: GNU General Public License v3.0 (自由是强制性的)
 * ============================================================================
 */

#![allow(warnings)]///关闭rust的大傻逼警告

use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::fs;
use std::process::Command;
use anyhow::{Context, Result};
use rustpython_parser::{ast, Parse};

// ============================================================================
// [1. 类型系统] 
// 实现 Python 动态类型到 C 强类型的映射推导
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum CType {
    Int,      // 映射为 long long
    Float,    // 映射为 double
    Dynamic,  // 回退为 PyObject*
}

impl CType {
    /// 将推导出的类型转换为 C 语言声明关键字
    fn to_c_type(&self) -> &str {
        match self {
            CType::Int => "long long",
            CType::Float => "double",
            _ => "PyObject*",
        }
    }
}

// ============================================================================
// [2. UI 消息驱动] 
// 确保编译高负载任务不阻塞 GUI 渲染主线程
// ============================================================================

enum AppMessage {
    Log(String),
    Error(String),
    Success(String),
    Progress(f32, String),
    Finished,
}

struct PareApp {
    selected_file: Option<PathBuf>,
    logs: String,
    is_working: bool,
    progress: f32,
    status_text: String,
    rx: Receiver<AppMessage>,
    tx: Sender<AppMessage>,
}

impl PareApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 🌟 初始化静态资源：将字体编译进二进制文件
        setup_embedded_font(&cc.egui_ctx);
        
        let (tx, rx) = channel();
        Self {
            selected_file: None,
            logs: String::from("🚀 PARE Studio (Pure C Loop 优化版) 已就绪...\n"),
            is_working: false,
            progress: 0.0,
            status_text: String::from("空闲"),
            rx,
            tx,
        }
    }

    /// 启动多线程编译任务，隔离 GUI 与转译逻辑
    fn start_mission(&mut self) {
        if let Some(path) = self.selected_file.clone() {
            self.is_working = true;
            self.progress = 0.0;
            self.logs.push_str(&format!("\n🔥 任务启动: {:?}\n", path));
            let tx = self.tx.clone();
            
            thread::spawn(move || {
                if let Err(e) = run_compilation_pipeline(path, tx.clone()) {
                    let _ = tx.send(AppMessage::Error(format!("❌ 失败: {}", e)));
                }
                let _ = tx.send(AppMessage::Finished);
            });
        }
    }
}

// ============================================================================
// [3. 核心流水线] 
// 控制从 Python 源码到可执行二进制文件的全过程
// ============================================================================

fn run_compilation_pipeline(input_path: PathBuf, tx: Sender<AppMessage>) -> Result<()> {
    // 1. 读取与 AST 解析
    let source = fs::read_to_string(&input_path)?;
    let ast_suite = ast::Suite::parse(&source, "<pare>")?;

    tx.send(AppMessage::Progress(0.4, "正在生成优化后的 C 代码...".into()))?;
    
    // 2. 执行转译引擎
    let mut symbols = HashMap::new();
    let c_code = transpile_optimized(&ast_suite, &mut symbols);

    // 3. 准备临时构建环境
    let temp_dir = tempfile::tempdir()?;
    let c_file_path = temp_dir.path().join("out.c");
    fs::write(&c_file_path, &c_code)?;

    tx.send(AppMessage::Progress(0.7, "正在调用 GCC 开启 -O3 降维打击...".into()))?;
    
    // 4. 自动检索 CPython 开发环境配置
    let cflags = get_python_config("--cflags")?;
    let ldflags = get_python_config("--ldflags")?;
    let embed = get_python_config_embed().unwrap_or_default();

    let output_exe = input_path.with_extension(if cfg!(windows) { "exe" } else { "out" });
    let mut cmd = Command::new("gcc");
    
    // 注入最高等级优化策略
    cmd.arg("-O3").arg(&c_file_path).arg("-o").arg(&output_exe);
    
    for flag in cflags.split_whitespace() { cmd.arg(flag); }
    for flag in ldflags.split_whitespace() { cmd.arg(flag); }
    for flag in embed.split_whitespace() { cmd.arg(flag); }

    let status = cmd.status()?;
    if status.success() {
        tx.send(AppMessage::Success(format!("编译成功: {:?}", output_exe)))?;
    } else {
        return Err(anyhow::anyhow!("GCC 编译失败。请检查 CPython 开发环境。"));
    }
    Ok(())
}

// ============================================================================
// [4. 极限转译引擎] 
// PARE 的心脏：将 Python 逻辑降维为原生 C 结构
// ============================================================================

fn transpile_optimized(ast: &ast::Suite, symbols: &mut HashMap<String, CType>) -> String {
    let mut c = String::new();
    c.push_str("#define PY_SSIZE_T_CLEAN\n#include <Python.h>\n#include <stdio.h>\n\n");
    c.push_str("int main() {\n");
    c.push_str("    Py_Initialize();\n");
    c.push_str("    PyObject *globals = PyDict_New();\n");
    c.push_str("    PyDict_SetItemString(globals, \"__builtins__\", PyEval_GetBuiltins());\n\n");

    // 🌟 核心优化阶段 1: 声明影子变量 (Shadow Variables Area)
    // 预扫描 AST，在 C 栈中提前分配原生空间，避免运行时的字典查找
    c.push_str("    // --- [Shadow Variables Area] ---\n");
    let mut declarations = Vec::new();
    find_all_vars(ast, &mut declarations);
    for (name, ctype) in &declarations {
        c.push_str(&format!("    {} _c_{} = 0;\n", ctype.to_c_type(), name));
        symbols.insert(name.clone(), ctype.clone());
    }
    c.push_str("\n");

    // 🌟 核心优化阶段 2: 递归逻辑生成
    c.push_str(&transpile_block(ast, symbols, 1));

    c.push_str("\n    Py_FinalizeEx();\n    return 0;\n}\n");
    c
}

/// 转译递归块：处理缩进、嵌套循环及逻辑深度
fn transpile_block(suite: &ast::Suite, symbols: &mut HashMap<String, CType>, indent_level: usize) -> String {
    let mut b = String::new();
    let pad = "    ".repeat(indent_level);

    for stmt in suite {
        match stmt {
            // [极致优化: 纯 C 循环 (Nuitka Killer 逻辑)]
            // 识别 Python 的 range() 并将其压平为原生 C for 循环
            ast::Stmt::For(f) => {
                if let ast::Expr::Name(target) = &*f.target {
                    if let ast::Expr::Call(call) = &*f.iter {
                        if let ast::Expr::Name(func) = &*call.func {
                            if func.id.as_str() == "range" {
                                let arg = call.args.first().expect("range() 必须包含参数");
                                let (limit_code, _) = eval_expr(arg, symbols);
                                let var = &target.id;
                                
                                b.push_str(&format!("{}for (_c_{} = 0; _c_{} < {}; _c_{}++) {{\n", pad, var, var, limit_code, var));
                                // 递归处理循环体内部逻辑
                                b.push_str(&transpile_block(&f.body, symbols, indent_level + 1));
                                b.push_str(&format!("{}}}\n", pad));
                                continue;
                            }
                        }
                    }
                }
            }

            // [赋值语句优化]
            // 计算结果仅保留在 C 影子变量中，暂不同步到虚拟机
            ast::Stmt::Assign(a) => {
                if let Some(ast::Expr::Name(n)) = a.targets.first().map(|e| &**e) {
                    let (expr_code, _) = eval_expr(&a.value, symbols);
                    b.push_str(&format!("{}_c_{} = {};\n", pad, n.id, expr_code));
                }
            }

            // [同步触发点 (Sync Gate)]
            // 当检测到输出或库导入时，强制将影子变量刷回 CPython 状态空间
            ast::Stmt::Expr(e) => {
                if let ast::Expr::Call(call) = &*e.value {
                    if let ast::Expr::Name(n) = &*call.func {
                        if n.id.as_str() == "print" {
                            b.push_str(&generate_sync_code(symbols, indent_level));
                            if let Some(arg) = call.args.first() {
                                if let ast::Expr::Name(an) = arg {
                                    b.push_str(&format!("{}PyRun_String(\"print({})\", Py_file_input, globals, globals);\n", pad, an.id));
                                }
                            }
                        }
                    }
                }
            }
            
            ast::Stmt::Import(_) | ast::Stmt::ImportFrom(_) => {
                b.push_str(&generate_sync_code(symbols, indent_level));
                b.push_str(&format!("{}PyRun_String(\"import numpy as np\", Py_file_input, globals, globals);\n", pad));
            }

            _ => {}
        }
    }
    b
}

/// 表达式递归评估与类型静态推导
fn eval_expr(expr: &ast::Expr, symbols: &HashMap<String, CType>) -> (String, CType) {
    match expr {
        ast::Expr::Constant(k) => match &k.value {
            ast::Constant::Int(i) => (i.to_string(), CType::Int),
            ast::Constant::Float(f) => (f.to_string(), CType::Float),
            _ => ("NULL".into(), CType::Dynamic),
        },
        // 从影子变量池中检索映射名
        ast::Expr::Name(n) => (format!("_c_{}", n.id), symbols.get(n.id.as_str()).cloned().unwrap_or(CType::Int)),
        ast::Expr::BinOp(b) => {
            let (l, lt) = eval_expr(&b.left, symbols);
            let (r, rt) = eval_expr(&b.right, symbols);
            let op = match b.op {
                ast::Operator::Add => "+",
                ast::Operator::Sub => "-",
                ast::Operator::Mult => "*",
                ast::Operator::Div => "/",
                _ => "+",
            };
            // 自动提升类型：Int + Float -> Float
            let res_t = if lt == CType::Float || rt == CType::Float { CType::Float } else { CType::Int };
            (format!("({} {} {})", l, op, r), res_t)
        }
        _ => ("0".into(), CType::Dynamic),
    }
}

/// 核心：延迟同步生成逻辑 (Flush Shadow to VM)
fn generate_sync_code(symbols: &HashMap<String, CType>, indent: usize) -> String {
    let mut sync = String::new();
    let pad = "    ".repeat(indent);
    for (name, ctype) in symbols {
        let py_creator = match ctype {
            CType::Int => "PyLong_FromLongLong",
            CType::Float => "PyFloat_FromDouble",
            _ => continue,
        };
        sync.push_str(&format!("{}PyDict_SetItemString(globals, \"{}\", {}(_c_{}));\n", pad, name, py_creator, name));
    }
    sync
}

/// 静态扫描：在主函数开头预置所有变量声明
fn find_all_vars(suite: &ast::Suite, decls: &mut Vec<(String, CType)>) {
    for stmt in suite {
        match stmt {
            ast::Stmt::Assign(a) => {
                if let Some(ast::Expr::Name(n)) = a.targets.first().map(|e| &**e) {
                    if !decls.iter().any(|(name, _)| name == n.id.as_str()) {
                        decls.push((n.id.to_string(), CType::Int)); 
                    }
                }
            }
            ast::Stmt::For(f) => {
                if let ast::Expr::Name(n) = &*f.target {
                    if !decls.iter().any(|(name, _)| name == n.id.as_str()) {
                        decls.push((n.id.to_string(), CType::Int));
                    }
                }
                find_all_vars(&f.body, decls);
            }
            _ => {}
        }
    }
}

// ============================================================================
// [5. GUI 逻辑实现] 
// ============================================================================

impl eframe::App for PareApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 接收异步编译线程反馈
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::Log(s) => self.logs.push_str(&format!("ℹ️ {}\n", s)),
                AppMessage::Error(s) => self.logs.push_str(&format!("{}\n", s)),
                AppMessage::Success(s) => self.logs.push_str(&format!("✅ {}\n", s)),
                AppMessage::Progress(p, s) => { self.progress = p; self.status_text = s; }
                AppMessage::Finished => { self.is_working = false; self.progress = 1.0; }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PARE Studio: 极限降维编译器 (Optimization Alpha)");
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("📂 选择任务脚本").clicked() && !self.is_working {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Python", &["py"]).pick_file() {
                        self.selected_file = Some(path);
                    }
                }
                ui.label(self.selected_file.as_ref().map_or("等待脚本...", |p| p.to_str().unwrap()));
            });

            ui.add_space(10.0);
            let strike_btn = egui::Button::new("🔥 执行极致降维转译 (CPython Strike)").min_size(egui::vec2(200.0, 40.0));
            if ui.add_enabled(!self.is_working && self.selected_file.is_some(), strike_btn).clicked() {
                self.start_mission();
            }

            ui.add_space(10.0);
            ui.add(egui::ProgressBar::new(self.progress).show_percentage());
            ui.label(format!("当前状态: {}", self.status_text));
            
            ui.separator();
            ui.heading("📜 降维打击日志");
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.logs)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY));
            });
        });
        if self.is_working { ctx.request_repaint(); }
    }
}

// ============================================================================
// [6. 辅助工具] 
// ============================================================================

fn setup_embedded_font(ctx: &egui::Context) {
    let font_data = include_bytes!("font.ttf"); // 🌟 静态编译期内嵌资源
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("my_font".to_owned(), egui::FontData::from_static(font_data));
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "my_font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "my_font".to_owned());
    ctx.set_fonts(fonts);
}

fn get_python_config(arg: &str) -> Result<String> {
    let out = Command::new("python3-config").arg(arg).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn get_python_config_embed() -> Result<String> {
    let out = Command::new("python3-config").arg("--ldflags").arg("--embed").output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 500.0])
            .with_title("PARE Studio - Pure C Edition"),
        ..Default::default()
    };
    eframe::run_native("PARE Studio", options, Box::new(|cc| Box::new(PareApp::new(cc))))
        .map_err(|e| anyhow::anyhow!("GUI 运行环境异常: {}", e))
}
