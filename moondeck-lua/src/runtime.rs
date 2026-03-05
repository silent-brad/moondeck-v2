use crate::bindings;
use anyhow::{Context, Result};
use moondeck_core::ui::Page;
use moondeck_hal::EnvConfig;
use piccolo::{Closure, Executor, Lua, StashedExecutor};

pub struct LuaRuntime {
    lua: Lua,
    executor: Option<StashedExecutor>,
}

impl LuaRuntime {
    pub fn new() -> Result<Self> {
        let lua = Lua::full();
        Ok(Self { lua, executor: None })
    }

    pub fn init(&mut self, env: &EnvConfig) -> Result<()> {
        bindings::register_all(&mut self.lua, env)
            .context("Failed to register Lua bindings")?;
        Ok(())
    }

    pub fn load_script(&mut self, script: &str) -> Result<()> {
        let executor = self.lua.try_enter(|ctx| {
            let closure = Closure::load(ctx, None, script.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to load script: {:?}", e))?;
            let executor = Executor::start(ctx, closure.into(), ());
            Ok(ctx.stash(executor))
        })?;
        self.executor = Some(executor);
        Ok(())
    }

    pub fn run_pending(&mut self) -> Result<()> {
        if let Some(ref executor) = self.executor {
            self.lua.execute::<()>(executor)
                .map_err(|e| anyhow::anyhow!("Lua execution error: {:?}", e))?;
        }
        Ok(())
    }

    pub fn load_file(&mut self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read Lua file: {}", path))?;
        self.load_script(&content)?;
        self.run_pending()
            .with_context(|| format!("Failed to execute Lua file: {}", path))
    }

    pub fn load_pages(&mut self) -> Result<Vec<Page>> {
        Ok(create_demo_pages())
    }

    pub fn lua(&mut self) -> &mut Lua {
        &mut self.lua
    }
}

fn create_demo_pages() -> Vec<Page> {
    use moondeck_core::ui::{Page, WidgetInstance};

    vec![
        Page::new("home", "Home")
            .with_background("#1a1a2e")
            .with_widget(
                WidgetInstance::new("clock", 20, 20, 360, 180)
                    .with_update_interval(1000),
            )
            .with_widget(
                WidgetInstance::new("status", 400, 20, 380, 180)
                    .with_update_interval(5000),
            )
            .with_widget(
                WidgetInstance::new("quote", 20, 220, 760, 200)
                    .with_update_interval(60000),
            ),
        Page::new("info", "System Info")
            .with_background("#16213e")
            .with_widget(
                WidgetInstance::new("sysinfo", 20, 20, 760, 420)
                    .with_update_interval(2000),
            ),
    ]
}

impl Default for LuaRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create Lua runtime")
    }
}
