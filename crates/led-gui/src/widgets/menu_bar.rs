use gpui::*;
use crate::workspace::Workspace;
use led_core::i18n::I18n;
use crate::widgets::{led_color_to_gpui, ui_font_family, with_alpha};

#[allow(dead_code)]
pub struct MenuBar {
    workspace: Entity<Workspace>,
    i18n: I18n,
    open_menu: Option<usize>,
}

impl MenuBar {
    #[allow(dead_code)]
    pub fn new(workspace: Entity<Workspace>, i18n: I18n, _cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            i18n,
            open_menu: None,
        }
    }

    #[allow(dead_code)]
    fn toggle_menu(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if self.open_menu == Some(idx) {
            self.open_menu = None;
        } else {
            self.open_menu = Some(idx);
        }
        cx.notify();
    }
}

impl Render for MenuBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;
        let bg = led_color_to_gpui(theme.ui.menu_bar_bg);
        let fg = led_color_to_gpui(theme.ui.menu_bar_fg);
        let border = with_alpha(led_color_to_gpui(theme.editor.line_number), 0.35);
        let hover_bg = with_alpha(fg, 0.12);

        div()
            .w_full()
            .h(px(28.0))
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.5))
            .font_family(ui_font_family())
            .border_b_1()
            .border_color(border)
            .flex()
            .items_center()
            .px_3()
            .gap_1()
            .child(
                div()
                    .h(px(22.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(0, window, cx)))
                    .child(self.i18n.get("menu.file").to_string())
            )
            .child(
                div()
                    .h(px(22.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(1, window, cx)))
                    .child(self.i18n.get("menu.edit").to_string())
            )
            .child(
                div()
                    .h(px(22.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(2, window, cx)))
                    .child(self.i18n.get("menu.view").to_string())
            )
            .child(
                div()
                    .h(px(22.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(3, window, cx)))
                    .child(self.i18n.get("menu.help").to_string())
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(1, window, cx)))
                    .child(self.i18n.get("menu.edit").to_string())
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(2, window, cx)))
                    .child(self.i18n.get("menu.view").to_string())
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.toggle_menu(3, window, cx)))
                    .child(self.i18n.get("menu.help").to_string())
            )
            // Dropdowns
            .child(if let Some(idx) = self.open_menu {
                div()
                    .absolute()
                    .top(px(24.0))
                    .left(px(idx as f32 * 50.0 + 8.0)) // Rough position
                    .w(px(150.0))
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .shadow_md()
                    .child(format!("Menu {} Dropdown", idx))
            } else {
                div()
            })
    }
}
