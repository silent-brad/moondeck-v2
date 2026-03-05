use crate::LuaRuntime;
use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::DrawTarget;
use moondeck_core::gfx::{Color, DrawContext};
use moondeck_core::ui::{Event, WidgetContext};
use moondeck_core::TtfFont;

pub struct WidgetPlugin {
    pub module: String,
    #[allow(dead_code)]
    state_key: String,
    initialized: bool,
}

impl WidgetPlugin {
    pub fn new(module: &str, instance_id: usize) -> Self {
        Self {
            module: module.to_string(),
            state_key: format!("{}_{}", module.replace('.', "_"), instance_id),
            initialized: false,
        }
    }

    pub fn init(&mut self, _runtime: &mut LuaRuntime, _ctx: &WidgetContext) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    pub fn update(&self, _runtime: &mut LuaRuntime, _delta_ms: u32) -> Result<()> {
        Ok(())
    }

    pub fn render<T: DrawTarget<Color = Rgb565>>(
        &self,
        _runtime: &mut LuaRuntime,
        ctx: &WidgetContext,
        draw_ctx: &mut DrawContext<'_, T>,
    ) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        draw_ctx.fill_rect(ctx.x, ctx.y, ctx.width, ctx.height, Color::from_hex("#58855C").unwrap_or(Color::GREEN));

        let title = format!("Widget: {}", self.module);
        draw_ctx.text_ttf(ctx.x + 20, ctx.y + 40, &title, Color::WHITE, TtfFont::garamond_italic(44));

        let info = format!("{}x{} @ ({},{})", ctx.width, ctx.height, ctx.x, ctx.y);
        draw_ctx.text_ttf(ctx.x + 20, ctx.y + 70, &info, Color::from_hex("#9EB8A0").unwrap_or(Color::GRAY), TtfFont::inter(38));

        draw_ctx.stroke_rect(ctx.x, ctx.y, ctx.width, ctx.height, Color::from_hex("#ADEBB3").unwrap_or(Color::GREEN), 1);

        Ok(())
    }

    pub fn on_event(&self, _runtime: &mut LuaRuntime, _event: &Event) -> Result<bool> {
        Ok(false)
    }
}
