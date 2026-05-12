use gpui::*;
use crate::workspace::Workspace;
use crate::widgets::led_color_to_gpui;

pub struct StatusBar {
    workspace: Entity<Workspace>,
}

impl StatusBar {
    pub fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
        cx.observe(&workspace, |_, _, cx| {
            cx.notify();
        }).detach();
        Self { workspace }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;
        let editor = workspace.active_editor();
        
        let file_name = editor.path.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("[No Name]");
        
        let modified_flag = if editor.is_modified() { " [+]" } else { "" };
        let vi_mode = format!("{:?}", editor.vi_mode).to_uppercase();
        
        let (line, col) = editor.char_to_line_col(editor.cursor);

        let selection_info = if let Some(ref sel) = editor.selection {
            format!("{} chars", sel.end - sel.start)
        } else {
            "".to_string()
        };

        let encoding = format!("{:?}", editor.encoding).to_uppercase();
        let line_ending = format!("{:?}", editor.line_ending).to_uppercase();
        let syntax = editor.syntax_highlighter.as_ref().map(|h| h.def.meta.name.clone()).unwrap_or("Plain Text".to_string());

        div()
            .h(px(24.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .bg(led_color_to_gpui(theme.ui.status_bar_bg))
            .text_color(led_color_to_gpui(theme.ui.status_bar_fg))
            .text_size(px(12.0))
            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
            .border_t_1()
            .border_color(led_color_to_gpui(theme.editor.line_number))
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .px_2()
                    .child(format!("{}{}", file_name, modified_flag))
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .px_2()
                    .gap_4()
                    .child(div().h_full().flex().items_center().child(selection_info))
                    .child(div().h_full().flex().items_center().child(vi_mode))
                    .child(div().h_full().flex().items_center().child(format!("Ln {}, Col {}", line + 1, col + 1)))
                    .child(div().h_full().flex().items_center().child(encoding))
                    .child(div().h_full().flex().items_center().child(line_ending))
                    .child(div().h_full().flex().items_center().child(syntax))
            )
    }
}
