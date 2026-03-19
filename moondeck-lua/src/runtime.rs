use crate::bindings::{self, lua_serde::table_to_json};
use anyhow::{Context, Result};
use moondeck_core::ui::{Page, WidgetInstance};
use moondeck_hal::EnvConfig;
use piccolo::{Closure, Executor, Fuel, Lua, StashedExecutor, Value};

pub const EMBEDDED_PAGES_LUA: &str = include_str!("../../config/pages.lua");

pub struct LuaRuntime {
    lua: Lua,
    executor: Option<StashedExecutor>,
    config_path: Option<String>,
}

impl LuaRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            lua: Lua::full(),
            executor: None,
            config_path: None,
        })
    }

    pub fn with_config_path(mut self, path: &str) -> Self {
        self.config_path = Some(path.to_string());
        self
    }

    pub fn init(&mut self, env: &EnvConfig) -> Result<()> {
        bindings::register_all(&mut self.lua, env).context("Failed to register Lua bindings")?;
        self.load_script("utils = require(\"utils\")")?;
        self.run_pending().context("Failed to initialize utils")
    }

    pub fn read_widget_source(&self, module: &str) -> Option<String> {
        let base = self.config_path.as_ref()?;
        // module names are like "widgets.clock" -> "widgets/clock/init.lua"
        let rel_path = module.replace('.', "/");
        let dir_path = format!("{}/{}/init.lua", base, rel_path);
        std::fs::read_to_string(&dir_path)
            .or_else(|_| {
                // Fallback to flat file for compatibility
                std::fs::read_to_string(format!("{}/{}.lua", base, rel_path))
            })
            .ok()
    }

    pub fn load_script(&mut self, script: &str) -> Result<()> {
        let executor = self.lua.try_enter(|ctx| {
            let closure = Closure::load(ctx, None, script.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to load script: {:?}", e))?;
            Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
        })?;
        self.executor = Some(executor);
        Ok(())
    }

    pub fn run_pending(&mut self) -> Result<()> {
        if let Some(ref executor) = self.executor {
            self.lua
                .execute::<()>(executor)
                .map_err(|e| anyhow::anyhow!("Lua error: {:?}", e))?;
        }
        Ok(())
    }

    pub fn load_pages(&mut self) -> Result<Vec<Page>> {
        self.load_pages_from_config().or_else(|e| {
            log::warn!("Failed to load pages: {}, using demo", e);
            Ok(vec![Page::new("home", "Home")])
        })
    }

    fn load_pages_from_config(&mut self) -> Result<Vec<Page>> {
        let lua_src = self
            .config_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(format!("{}/pages.lua", p)).ok())
            .unwrap_or_else(|| EMBEDDED_PAGES_LUA.to_string());

        // Run pages.lua in the main runtime so require() works
        let stashed = self.lua.try_enter(|ctx| {
            let closure = Closure::load(ctx, Some("pages.lua".into()), lua_src.as_bytes())
                .map_err(|e| anyhow::anyhow!("Compile error: {:?}", e))?;
            Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
        })?;

        let json_string: String = self.lua.enter(|ctx| {
            let exec = ctx.fetch(&stashed);
            let mut fuel = Fuel::with(1000000);
            while !exec.step(ctx, &mut fuel) {
                if fuel.remaining() <= 0 {
                    break;
                }
            }
            match exec.take_result::<Value>(ctx) {
                Ok(Ok(Value::Table(t))) => {
                    serde_json::to_string(&table_to_json(ctx, t)).unwrap_or_default()
                }
                _ => String::new(),
            }
        });

        if json_string.is_empty() {
            return Err(anyhow::anyhow!("pages.lua did not return valid table"));
        }

        parse_pages_json(&json_string)
    }

    pub fn lua(&mut self) -> &mut Lua {
        &mut self.lua
    }
    pub fn get_theme_background(&self) -> String {
        bindings::get_theme_bg_primary().to_string()
    }
    pub fn get_current_theme(&self) -> String {
        bindings::get_current_theme()
    }
}

impl Default for LuaRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create Lua runtime")
    }
}

// Layout constants
const SCREEN: (i32, i32) = (800, 480);
const MARGIN: i32 = 20;
const GUTTER: i32 = 16;
const COLS: i32 = 12;

#[derive(Debug, Clone, serde::Deserialize)]
struct PagesConfig {
    pages: Vec<PageConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PageConfig {
    id: String,
    title: Option<String>,
    layout: Option<String>,
    #[serde(default)]
    widgets: Vec<WidgetConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct WidgetRef {
    _module: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct WidgetConfig {
    #[serde(default)]
    module: Option<String>,
    #[serde(default)]
    widget: Option<WidgetRef>,
    #[serde(default)]
    slot: usize,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default = "default_dim")]
    w: u32,
    #[serde(default = "default_dim")]
    h: u32,
    update_interval: Option<u64>,
    opts: Option<serde_json::Value>,
}

impl WidgetConfig {
    fn module_name(&self) -> Option<&str> {
        self.module
            .as_deref()
            .or(self.widget.as_ref().map(|w| w._module.as_str()))
    }
}

fn default_dim() -> u32 {
    100
}

fn get_layout_slots(name: &str) -> Vec<(i32, i32, i32, i32)> {
    // (col, span, row, row_span)
    match name {
        "full" => vec![(1, 12, 1, 2)],
        "half_half" => vec![(1, 6, 1, 2), (7, 6, 1, 2)],
        "thirds" => vec![(1, 4, 1, 2), (5, 4, 1, 2), (9, 4, 1, 2)],
        "main_sidebar" | "dashboard" => vec![(1, 8, 1, 2), (9, 4, 1, 1), (9, 4, 2, 1)],
        "quad" => vec![(1, 6, 1, 1), (7, 6, 1, 1), (1, 6, 2, 1), (7, 6, 2, 1)],
        _ => vec![(1, 12, 1, 2)],
    }
}

fn slot_bounds(col: i32, span: i32, row: i32, row_span: i32, rows: i32) -> (i32, i32, u32, u32) {
    let col_w = (SCREEN.0 - MARGIN * 2 - GUTTER * (COLS - 1)) / COLS;
    let row_h = (SCREEN.1 - MARGIN * 2 - GUTTER * (rows - 1)) / rows;
    let x = MARGIN + (col - 1) * (col_w + GUTTER);
    let y = MARGIN + (row - 1) * (row_h + GUTTER);
    let w = col_w * span + GUTTER * (span - 1);
    let h = row_h * row_span + GUTTER * (row_span - 1);
    (x, y, w as u32, h as u32)
}

fn parse_pages_json(json_string: &str) -> Result<Vec<Page>> {
    let config: PagesConfig = serde_json::from_str(json_string)
        .with_context(|| format!("Parse error: {}", json_string))?;

    Ok(config
        .pages
        .into_iter()
        .map(|p| {
            let mut page = Page::new(&p.id, p.title.as_deref().unwrap_or(&p.id));
            let slots = p.layout.as_deref().map(get_layout_slots);

            for w in p.widgets {
                let module = match w.module_name() {
                    Some(m) => m.to_string(),
                    None => {
                        log::warn!("Widget missing module/widget reference, skipping");
                        continue;
                    }
                };

                let (x, y, width, height) = slots
                    .as_ref()
                    .and_then(|s| s.get(w.slot.saturating_sub(1).max(0)))
                    .map(|&(c, sp, r, rs)| slot_bounds(c, sp, r, rs, 2))
                    .unwrap_or((w.x, w.y, w.w, w.h));

                let mut widget = WidgetInstance::new(&module, x, y, width, height)
                    .with_update_interval(w.update_interval.unwrap_or(1000));
                if let Some(opts) = w.opts {
                    widget.context.opts = opts
                        .as_object()
                        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default();
                }
                page = page.with_widget(widget);
            }
            page
        })
        .collect())
}
