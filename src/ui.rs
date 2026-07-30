//! Immediate-mode UI built from macroquad-toolkit surfaces and helpers.

use crate::data::GameData;
use crate::state::GameSession;
use macroquad::prelude::*;
use macroquad_toolkit::grid::{FogState, TilePos};
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::{RectExt, VirtualUi};

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    NewGame,
    Save,
    Load,
    DeleteSave,
    RunAction(String),
    SelectTile(TilePos),
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub session: &'a GameSession,
    pub save_exists: bool,
    pub save_slots: &'a [String],
    pub loaded_assets: usize,
    pub camera_target: Vec2,
    pub camera_zoom: f32,
    pub ui: &'a VirtualUi,
}

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let mouse = ctx.ui.mouse_position();

    draw_header(ctx.data, ctx.session);
    draw_world_panel(&ctx, mouse);
    if is_mouse_button_released(MouseButton::Left) {
        if let Some(tile) = selected_tile_from_mouse(&ctx) {
            actions.push(UiAction::SelectTile(tile));
        }
    }
    draw_control_panel(&ctx, mouse, &mut actions);
    draw_footer();

    actions
}

fn draw_header(data: &GameData, session: &GameSession) {
    let rect = Rect::new(18.0, 16.0, LOGICAL_WIDTH - 36.0, 64.0);
    let style = SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
        .with_border(1.0, dark::ACCENT)
        .with_top_highlight(2.0, Color::new(0.55, 0.72, 0.95, 0.75));
    draw_surface(rect, &style);

    draw_ui_text_ex(
        &data.config.display_name,
        rect.x + 18.0,
        rect.y + 39.0,
        TextStyle::new(30.0, dark::TEXT_BRIGHT).params(),
    );

    draw_badge(
        Rect::new(rect.right() - 332.0, rect.y + 18.0, 100.0, 28.0),
        &format!("Turn {}", session.player.turn),
        Color::new(0.18, 0.24, 0.32, 1.0),
        dark::TEXT,
    );
    draw_badge(
        Rect::new(rect.right() - 218.0, rect.y + 18.0, 92.0, 28.0),
        &format_money(session.player.points),
        Color::new(0.18, 0.28, 0.20, 1.0),
        dark::TEXT,
    );
    draw_badge(
        Rect::new(rect.right() - 112.0, rect.y + 18.0, 94.0, 28.0),
        &format!("v{}", data.config.version),
        Color::new(0.22, 0.19, 0.30, 1.0),
        dark::TEXT,
    );
}

fn draw_world_panel(ctx: &UiContext<'_>, mouse: Vec2) {
    let rect = world_panel_rect();
    let style = SurfaceStyle::new(Color::new(0.06, 0.065, 0.08, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.65))
        .with_inner_border(4.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.05))
        .with_header(42.0, Color::new(0.09, 0.105, 0.13, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(
        rect,
        Some("Toolkit Grid + Camera Example"),
        &style,
        TextStyle::new(18.0, dark::TEXT),
    );

    let grid_rect = world_grid_rect();
    draw_grid_demo(ctx, grid_rect);

    if grid_rect.contains_point(mouse) {
        draw_tooltip(
            "Arrow keys move the selected tile. Right mouse drag and +/- adjust the toolkit camera state.",
            mouse,
        );
    }
}

fn draw_grid_demo(ctx: &UiContext<'_>, rect: Rect) {
    let grid = &ctx.session.world.fog;
    let view = GridView::new(ctx, rect);
    let selected = ctx.session.player.selected_tile;

    for (pos, fog) in grid.iter_with_pos() {
        let tile_rect = view.tile_rect(pos);
        if !rect.overlaps(&tile_rect) {
            continue;
        }

        let base = match fog {
            FogState::Hidden => Color::new(0.08, 0.08, 0.10, 1.0),
            FogState::Revealed => Color::new(0.16, 0.17, 0.20, 1.0),
            FogState::Visible => Color::new(0.23, 0.30, 0.25, 1.0),
        };
        let fill = if ctx.session.world.reachable.contains(&pos) {
            Color::new(base.r + 0.08, base.g + 0.08, base.b + 0.05, 1.0)
        } else {
            base
        };
        draw_rectangle(tile_rect.x, tile_rect.y, tile_rect.w, tile_rect.h, fill);

        if pos == selected {
            draw_rectangle_lines(
                tile_rect.x,
                tile_rect.y,
                tile_rect.w,
                tile_rect.h,
                3.0,
                dark::ACCENT,
            );
        }
    }

    let footer = Rect::new(rect.x, rect.bottom() - 28.0, rect.w, 28.0);
    draw_text_centered_in_box(
        &format!(
            "Selected tile: {}, {} | Reachable: {}",
            selected.x,
            selected.y,
            ctx.session.world.reachable.len()
        ),
        footer.x,
        footer.y,
        footer.w,
        footer.h,
        16.0,
        dark::TEXT_DIM,
    );
}

fn draw_control_panel(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(852.0, 96.0, 410.0, 520.0);
    let style = SurfaceStyle::new(Color::new(0.08, 0.085, 0.105, 0.97))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.65))
        .with_header(42.0, Color::new(0.105, 0.12, 0.15, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(
        rect,
        Some("Toolkit UI + Persistence"),
        &style,
        TextStyle::new(18.0, dark::TEXT),
    );

    let content = rect.inset(18.0);
    let mut y = content.y + 40.0;
    meter(
        Rect::new(content.x, y, content.w, 24.0),
        ctx.session.player.energy,
        ctx.data.config.max_energy,
        dark::POSITIVE,
        Some(&format!(
            "Energy {:.0}/{:.0}",
            ctx.session.player.energy, ctx.data.config.max_energy
        )),
    );
    y += 42.0;

    draw_ui_text_ex(
        "Actions",
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 12.0;

    let action_count = ctx.data.actions.len();
    let layout = GridLayout::new(content.x, y + 10.0, content.w, 10.0, 1, 72.0);
    for (index, (_, action)) in ctx.data.actions.iter().enumerate() {
        let (x, card_y, w, h) = layout.get_item_rect(index, 0.0);
        let card_rect = Rect::new(x, card_y, w, h);
        let enabled = ctx.session.can_run_action(action);
        if draw_action_card(card_rect, action, enabled, mouse) {
            actions.push(UiAction::RunAction(action.id.clone()));
        }
    }
    y += layout.content_height(action_count) + 22.0;

    draw_ui_text_ex(
        "Save Data",
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 14.0;

    let btn_w = (content.w - 10.0) / 2.0;
    if virtual_button(
        Rect::new(content.x, y, btn_w, 38.0),
        "Save",
        true,
        ButtonTone::Positive,
        mouse,
    ) {
        actions.push(UiAction::Save);
    }
    if virtual_button(
        Rect::new(content.x + btn_w + 10.0, y, btn_w, 38.0),
        "Load",
        ctx.save_exists,
        ButtonTone::Primary,
        mouse,
    ) {
        actions.push(UiAction::Load);
    }
    y += 48.0;

    if virtual_button(
        Rect::new(content.x, y, btn_w, 36.0),
        "New Game",
        true,
        ButtonTone::Secondary,
        mouse,
    ) {
        actions.push(UiAction::NewGame);
    }
    if virtual_button(
        Rect::new(content.x + btn_w + 10.0, y, btn_w, 36.0),
        "Delete Save",
        ctx.save_exists,
        ButtonTone::Danger,
        mouse,
    ) {
        actions.push(UiAction::DeleteSave);
    }
    y += 52.0;

    let saves = if ctx.save_slots.is_empty() {
        "No save slots found".to_owned()
    } else {
        format!("Save slots: {}", ctx.save_slots.join(", "))
    };
    draw_text_block(
        &format!(
            "{}\nManifest textures loaded: {}\nToolkit save slot: {}",
            saves, ctx.loaded_assets, ctx.data.config.save_slot
        ),
        content.x,
        y,
        content.w,
        74.0,
        15.0,
        3.0,
        dark::TEXT_DIM,
    );
}

fn draw_action_card(
    rect: Rect,
    action: &crate::data::ActionDef,
    enabled: bool,
    mouse: Vec2,
) -> bool {
    let hovered = enabled && rect.contains_point(mouse);
    let fill = if hovered {
        Color::new(0.15, 0.18, 0.23, 1.0)
    } else {
        Color::new(0.11, 0.125, 0.16, 1.0)
    };
    let style = SurfaceStyle::new(fill)
        .with_left_accent(
            4.0,
            if enabled {
                dark::ACCENT
            } else {
                dark::TEXT_DIM
            },
        )
        .with_border(1.0, Color::new(0.5, 0.55, 0.65, 0.35));
    draw_surface(rect, &style);

    draw_ui_text_ex(
        &action.name,
        rect.x + 16.0,
        rect.y + 26.0,
        TextStyle::new(18.0, if enabled { dark::TEXT } else { dark::TEXT_DIM }).params(),
    );
    draw_ui_text_ex(
        &action.description,
        rect.x + 16.0,
        rect.y + 50.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        &format!(
            "-{:.0} energy / +{}",
            action.energy_cost, action.points_reward
        ),
        rect.right() - 14.0,
        rect.y + 28.0,
        TextStyle::new(14.0, dark::TEXT_DIM),
    );

    hovered && is_mouse_button_released(MouseButton::Left)
}

fn virtual_button(rect: Rect, text: &str, enabled: bool, tone: ButtonTone, mouse: Vec2) -> bool {
    let style = ButtonStyle::from_tone(tone);
    let hovered = enabled && rect.contains_point(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let activated = hovered && is_mouse_button_released(MouseButton::Left);
    let fill = if !enabled {
        style.disabled
    } else if pressed {
        style.pressed
    } else if hovered {
        style.hovered
    } else {
        style.normal
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill).with_border(1.0, style.border),
    );
    draw_text_centered_in_box_ex(
        text,
        rect.x + 8.0,
        rect.y + if pressed { 2.0 } else { 0.0 },
        rect.w - 16.0,
        rect.h,
        TextStyle::new(
            17.0,
            if enabled {
                style.text_color
            } else {
                dark::TEXT_DIM
            },
        ),
    );
    activated
}

fn draw_footer() {
    let rect = Rect::new(18.0, 632.0, LOGICAL_WIDTH - 36.0, 70.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.055, 0.06, 0.075, 0.96))
            .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.45)),
    );
    draw_text_block(
        "Template systems: macroquad-toolkit VirtualUi, SurfaceStyle, TextStyle, GridLayout, FlatGrid, FogState, Camera2D, EventBus, NotificationManager, DataRegistry, AssetManager, save slots, and migration callbacks.",
        rect.x + 18.0,
        rect.y + 14.0,
        rect.w - 36.0,
        rect.h - 20.0,
        17.0,
        4.0,
        dark::TEXT_DIM,
    );
}

pub fn tile_move_from_keys() -> Option<(i32, i32)> {
    if is_key_pressed(KeyCode::Up) {
        Some((0, -1))
    } else if is_key_pressed(KeyCode::Right) {
        Some((1, 0))
    } else if is_key_pressed(KeyCode::Down) {
        Some((0, 1))
    } else if is_key_pressed(KeyCode::Left) {
        Some((-1, 0))
    } else {
        None
    }
}

pub fn selected_tile_from_mouse(ctx: &UiContext<'_>) -> Option<TilePos> {
    let mouse = ctx.ui.mouse_position();
    let rect = world_grid_rect();
    if !rect.contains_point(mouse) {
        return None;
    }

    let grid = &ctx.session.world.fog;
    let pos = GridView::new(ctx, rect).tile_at(mouse);

    grid.is_valid(pos).then_some(pos)
}

fn world_panel_rect() -> Rect {
    Rect::new(18.0, 96.0, 812.0, 520.0)
}

fn world_grid_rect() -> Rect {
    let rect = world_panel_rect();
    Rect::new(rect.x + 24.0, rect.y + 66.0, rect.w - 48.0, rect.h - 92.0)
}

#[derive(Debug, Clone, Copy)]
struct GridView {
    origin: Vec2,
    scaled_tile_size: f32,
}

impl GridView {
    fn new(ctx: &UiContext<'_>, rect: Rect) -> Self {
        let grid = &ctx.session.world.fog;
        let tile_size = (rect.w / grid.width as f32)
            .min(rect.h / grid.height as f32)
            .floor()
            .max(12.0);
        let grid_w = grid.width as f32 * tile_size;
        let grid_h = grid.height as f32 * tile_size;
        let origin = vec2(
            rect.x + (rect.w - grid_w) * 0.5 - ctx.camera_target.x * 0.08,
            rect.y + (rect.h - grid_h) * 0.5 - ctx.camera_target.y * 0.08,
        );
        Self {
            origin,
            scaled_tile_size: tile_size * ctx.camera_zoom,
        }
    }

    fn tile_rect(self, pos: TilePos) -> Rect {
        Rect::new(
            self.origin.x + pos.x as f32 * self.scaled_tile_size,
            self.origin.y + pos.y as f32 * self.scaled_tile_size,
            (self.scaled_tile_size - 2.0).max(6.0),
            (self.scaled_tile_size - 2.0).max(6.0),
        )
    }

    fn tile_at(self, point: Vec2) -> TilePos {
        TilePos::new(
            ((point.x - self.origin.x) / self.scaled_tile_size).floor() as i32,
            ((point.y - self.origin.y) / self.scaled_tile_size).floor() as i32,
        )
    }
}
