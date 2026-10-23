//! Prompt Pad 后端入口。
//!
//! 分层约定（见 AGENTS.md §4）：
//! - `commands` 只做参数校验与编排，不直接碰窗口 API 或数据库；
//! - `services` 封装窗口控制、模型适配、夜间任务等能力；
//! - `storage` 负责 SQLite 索引层与 Markdown 记忆层；
//! - `geometry` 是无副作用的纯函数，便于单测。

mod clock;
mod commands;
mod config;
mod error;
mod fsutil;
mod geometry;
mod models;
mod services;
mod storage;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

use crate::commands::panel::PanelMode;
use crate::config::{AppConfig, ConfigFileStatus};
use crate::geometry::Anchor;
use crate::models::PadView;
use crate::services::window_ctl::PAD_WINDOW_LABEL;
use crate::storage::db::Db;

/// 全局共享状态。
///
/// SQLite 的 `Connection` 是 `Send` 但不是 `Sync`，用 `Mutex` 包一层即可同时满足
/// Tauri 要求的 `Send + Sync`，也天然保证「单写」—— 与设计里的单机单写约束一致。
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub config_status: Mutex<ConfigFileStatus>,
    pub db: Mutex<Db>,
    /// DOT 的位置锚点（哪块屏 + 屏内百分比坐标）。
    pub anchor: Mutex<Anchor>,
    pub config_path: PathBuf,
    pub position_path: PathBuf,
    pub memory_dir: PathBuf,
    /// 复用同一个 HTTP 客户端，连接池对常驻应用是必要的。
    pub http: reqwest::Client,
    /// 当前面板形态。Rust 侧是唯一权威，前端只做镜像。
    pub mode: Mutex<PanelMode>,
    /// 窗口最后一次获得焦点的时间戳。
    ///
    /// 展开面板时我们先 `set_focusable(true)` 再 `set_focus()`，这期间 Windows
    /// 可能补发一次 `Focused(false)`。若不加保护，面板刚展开就会被立刻收起。
    /// 因此记录获得焦点的时间，`Focused(false)` 事件距其过近时视为展开过程
    /// 中的过渡事件，忽略之。
    pub focused_at: Mutex<Option<std::time::Instant>>,
    /// 展开动作的截止时刻：热键展开时 Focused(false) 可能先于 Focused(true) 到达。
    pub expanding_until: Mutex<Option<std::time::Instant>>,
    /// 锚点上次落盘时间，拖拽期节流用。
    pub last_anchor_save: Mutex<Option<std::time::Instant>>,
    /// 展开面板之前的前台窗口句柄，收起时把焦点还回去。
    ///
    /// 「焦点必须能还给原窗口」是本产品的核心承诺：展开面板时我们主动抢焦点，
    /// 收起时必须还回去，否则用户继续打字会全部进到 Pad 里。
    pub prev_focus: Mutex<Option<isize>>,
    /// 面板当前显示的视图。
    ///
    /// 设置界面里用户要去别的应用复制 API key，点面板外部是正常工作流；
    /// 若这时把面板收起，用户填了一半的表单就没了。后端据此决定
    /// 「点外部收起」在设置视图下是否生效。
    pub view: Mutex<PadView>,
}

/// 应用入口。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 第二次启动：唤醒已有实例而不是开出第二个 Pad。
            // 用「展开面板」而不是 show + set_focus：
            // 收起态的 DOT 只有 48×48 且带 WS_EX_NOACTIVATE，
            // 强行激活它会把当前应用的键盘焦点抢走，却什么也看不见。
            let _ = commands::panel::toggle_panel(
                app.clone(),
                Some(commands::panel::PanelAction::Expand),
            );
        }))
        .plugin(
            // 注意：`tauri.conf.json` 里 `plugins.global-shortcut` 必须是 `null`，不能写 `{}`。
            // Tauri 会按插件的配置类型反序列化该值，而这三个插件都用默认的
            // `PluginBuilder::new()`（配置类型为 `()`），只接受 unit；
            // 写成空对象会让应用在插件初始化阶段直接 panic，连窗口都起不来。
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = commands::panel::toggle_panel(app.clone(), None);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&config_dir)?;

            let config_path = config_dir.join("config.json");
            let position_path = data_dir.join("position.json");
            let memory_dir = data_dir.join("memory");
            std::fs::create_dir_all(&memory_dir)?;
            // 清掉上次崩溃留下的半截临时文件，避免卡住配置 / 记忆写盘。
            let _ = fsutil::cleanup_stale_tmps(&config_dir);
            let _ = fsutil::cleanup_stale_tmps(&data_dir);
            let _ = fsutil::cleanup_stale_tmps(&memory_dir);

            let (config, config_status) = config::load_with_status(&config_path)?;
            let anchor = services::position::load_anchor(&position_path);
            let db = Db::open(&data_dir.join("prompt_pad.db"))?;

            // 记忆库文件：不存在时建立空模板，保证「纯 Markdown 全量可读」。
            let memory_file = storage::markdown::memory_path(&memory_dir);
            if !memory_file.exists() {
                storage::markdown::write_memory(&memory_file, &[], config.memory_token_budget)?;
            }
            let record_file = storage::markdown::record_path(&memory_dir);
            if !record_file.exists() {
                std::fs::write(
                    &record_file,
                    "# 提示词事实层（L1）\n\n逐字不可改，只允许追加。每条记录 ID 形如 `PE-YYYY-NNN`。\n",
                )?;
            }
            let _ = services::hydrate::rebuild_from_markdown(&db, &memory_dir);

            let http = reqwest::Client::builder()
                .user_agent(concat!("PromptPad/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|err| {
                    error::AppError::provider_unavailable("初始化 HTTP 客户端失败")
                        .with_details(err.to_string())
                })?;

            app.manage(AppState {
                config: Mutex::new(config.clone()),
                config_status: Mutex::new(config_status),
                db: Mutex::new(db),
                anchor: Mutex::new(anchor.clone()),
                config_path,
                position_path,
                memory_dir,
                http,
                mode: Mutex::new(PanelMode::Collapsed),
                focused_at: Mutex::new(None),
                expanding_until: Mutex::new(None),
                last_anchor_save: Mutex::new(None),
                prev_focus: Mutex::new(None),
                view: Mutex::new(PadView::Prompt),
            });

            // 恢复 DOT 位置并进入收起态。
            if let Some(window) = app.get_webview_window(PAD_WINDOW_LABEL) {
                services::window_ctl::restore_position(&window, &anchor)?;
                // 冷启动没有「展开前的原窗口」需要归还。
                let mut prev_focus = None;
                services::window_ctl::collapse(&window, &config, &anchor, &mut prev_focus, false)?;
            }

            services::tray::build(app.handle(), &config.global_shortcut)?;

            // 热键来自配置（默认 Ctrl+Q），不是写死的字符串：
            // 用户在设置里改键后要能立刻生效，就必须在改配置时重新绑定。
            services::shortcut::bind(app.handle(), &config.global_shortcut)?;

            if config.autostart_enabled {
                let _ = services::autostart::apply(app.handle(), true);
            }

            spawn_nightly_worker(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Moved(_) => {
                // 只在收起态记录位置：展开态窗口被挪到了面板位置，
                // 那时写回锚点会把 DOT 的位置覆盖成面板位置。
                let app = window.app_handle();
                let Some(state) = app.try_state::<AppState>() else {
                    return;
                };
                let Ok(mode) = state.mode.lock() else {
                    return;
                };
                if *mode != PanelMode::Collapsed {
                    return;
                }
                drop(mode);

                // on_window_event 的回调首参是 `&Window<R>`，而 current_anchor 形参是
                // `&WebviewWindow<R>`。`WebviewWindow` 没有 `Deref<Target = Window>`，
                // 也没有公开的 `window()` 取用器，两者之间不存在自动转换路径，
                // 因此只能按 label 取回 `WebviewWindow` 再传进去。
                let Some(pad) = app.get_webview_window(PAD_WINDOW_LABEL) else {
                    return;
                };
                match services::window_ctl::current_anchor(&pad) {
                    Ok(anchor) => {
                        // 锚点没变就不落盘：程序化移动（展开/收起）也会触发 Moved，
                        // 每次都写文件既无谓又会把磁盘写放大。
                        let unchanged = match state.anchor.lock() {
                            Ok(current) => {
                                current.x_pct == anchor.x_pct
                                    && current.y_pct == anchor.y_pct
                                    && current.monitor_index == anchor.monitor_index
                                    && current.monitor_name == anchor.monitor_name
                            }
                            Err(_) => false,
                        };
                        if !unchanged {
                            if let Ok(mut current) = state.anchor.lock() {
                                *current = anchor.clone();
                            }
                            let should_write = match state.last_anchor_save.lock() {
                                Ok(mut last) => {
                                    let now = std::time::Instant::now();
                                    let due = last
                                        .map(|at| now.duration_since(at).as_millis() >= 250)
                                        .unwrap_or(true);
                                    if due {
                                        *last = Some(now);
                                    }
                                    due
                                }
                                Err(_) => true,
                            };
                            if should_write {
                                if let Err(err) =
                                    services::position::save_anchor(&anchor, &state.position_path)
                                {
                                    eprintln!("[position] 持久化失败: {err}");
                                }
                            }
                        }
                    }
                    Err(err) => eprintln!("[position] 计算锚点失败: {err}"),
                }
            }
            tauri::WindowEvent::Focused(true) => {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut slot) = state.focused_at.lock() {
                        *slot = Some(std::time::Instant::now());
                    }
                }
            }
            tauri::WindowEvent::Focused(false) => {
                // 点面板外部 → 收起。收起态下窗口本来就不抢焦点，不处理。
                let app = window.app_handle();
                let Some(state) = app.try_state::<AppState>() else {
                    return;
                };
                let Ok(mode) = state.mode.lock() else {
                    return;
                };
                if *mode != PanelMode::Expanded {
                    return;
                }
                drop(mode);
                eprintln!("[focus] false");

                // 展开过程中的过渡性失焦不触发收起。
                // expanding_until 在 expand 开头就写入，覆盖「Focused(false) 先于 true」的情况。
                if let Ok(slot) = state.expanding_until.lock() {
                    if let Some(until) = *slot {
                        if std::time::Instant::now() < until {
                            return;
                        }
                    }
                }
                if let Ok(slot) = state.focused_at.lock() {
                    if let Some(at) = *slot {
                        if at.elapsed()
                            < std::time::Duration::from_millis(
                                services::window_ctl::FOCUS_GRACE_MS,
                            )
                        {
                            return;
                        }
                    }
                }

                // 设置视图里点外部是正常工作流（去别的应用复制 API key），
                // 这时收起面板等于把用户填了一半的表单弄丢。
                let in_settings = {
                    let guard = state.view.lock().ok();
                    match guard {
                        Some(view) => *view == PadView::Settings,
                        None => false,
                    }
                };
                if in_settings {
                    return;
                }

                // 拖拽面板时 Windows 会把面板置为非激活（鼠标划过别的应用窗口就会触发）。
                // 这不是「用户点了外面」，光标此刻仍压在面板上 —— 用它把两种情况分开。
                let Some(pad) = app.get_webview_window(PAD_WINDOW_LABEL) else {
                    return;
                };
                if services::window_ctl::cursor_inside(&pad) {
                    eprintln!("[focus] false 但光标在面板内（拖拽中），不收起");
                    return;
                }

                commands::panel::collapse(app);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::panel::toggle_panel,
            commands::optimize::optimize_prompt,
            commands::record::save_pe_record,
            commands::record::submit_feedback,
            commands::memory::retrieve_memories,
            commands::memory::run_nightly_job,
            commands::config_cmd::get_config,
            commands::config_cmd::set_config,
            commands::dot::begin_dot_drag,
            commands::panel::set_panel_view,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Prompt Pad 失败");
}

/// 每小时尝试跑一次「昨天」的夜间沉淀；同一天有标记文件则服务层直接跳过写盘。
fn spawn_nightly_worker<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    std::thread::Builder::new()
        .name("prompt-pad-nightly".into())
        .spawn(move || loop {
            let date = clock::yesterday_date();
            let Some(state) = app.try_state::<AppState>() else {
                std::thread::sleep(std::time::Duration::from_secs(60));
                continue;
            };
            let budget = state
                .config
                .lock()
                .map(|config| config.memory_token_budget)
                .unwrap_or(800);
            let memory_dir = state.memory_dir.clone();
            if let Ok(db) = state.db.lock() {
                if let Err(err) =
                    services::memory::run_nightly(&db, &memory_dir, budget, &date, false)
                {
                    eprintln!("[nightly] {err}");
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(3600));
        })
        .ok();
}
