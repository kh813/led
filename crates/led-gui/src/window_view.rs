use gpui::*;
use led_core::config::Config;
use led_core::i18n::I18n;
use crate::widgets::editor_view::EditorView;
use crate::widgets::tab_bar::TabBar;
use crate::widgets::status_bar::StatusBar;
use crate::widgets::find_panel::FindPanel;
use crate::workspace::Workspace;
use crate::app::*;
use led_core::buffer::Editor;

#[cfg(not(target_os = "macos"))]
use crate::widgets::menu_bar::MenuBar;

use crate::widgets::dialog::{Dialog, DialogType, DialogEvent, UnsavedChangesIntent};

pub struct WindowView {
    config: Config,
    i18n: I18n,
    workspace: Entity<Workspace>,
    editor: Entity<EditorView>,
    tab_bar: Entity<TabBar>,
    status_bar: Entity<StatusBar>,
    find_panel: Entity<FindPanel>,
    #[cfg(not(target_os = "macos"))]
    menu_bar: Entity<MenuBar>,
    dialog: Option<Entity<Dialog>>,
    focus_handle: FocusHandle,
}

impl WindowView {
    pub fn new(config: Config, i18n: I18n, workspace: Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| EditorView::new(workspace.clone(), cx));
        let tab_bar = cx.new(|cx| TabBar::new(workspace.clone(), cx));
        let status_bar = cx.new(|cx| StatusBar::new(workspace.clone(), cx));
        let find_panel = cx.new(|cx| FindPanel::new(workspace.clone(), cx));
        #[cfg(not(target_os = "macos"))]
        let menu_bar = cx.new(|cx| MenuBar::new(workspace.clone(), i18n.clone(), cx));
        
        let focus_handle = cx.focus_handle();
        
        // Focus the editor by default
        editor.update(cx, |editor, cx| {
            editor.focus_handle.focus(window, cx);
        });

        cx.observe(&workspace, |_, _, cx| {
            cx.notify();
        }).detach();

        Self {
            config,
            i18n,
            workspace,
            editor,
            tab_bar,
            status_bar,
            find_panel,
            #[cfg(not(target_os = "macos"))]
            menu_bar,
            dialog: None,
            focus_handle,
        }
    }

    fn show_dialog(&mut self, dialog_type: DialogType, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.new(|cx| Dialog::new(self.workspace.clone(), self.i18n.clone(), dialog_type, cx));
        cx.subscribe(&dialog, |this, _dialog, event, cx| {
            match event {
                DialogEvent::Close => {
                    this.dialog = None;
                    cx.notify();
                    
                    // Defer focus restoration to avoid re-entrancy
                    cx.spawn(|_, cx: &mut AsyncApp| {
                        let cx = cx.clone();
                        async move {
                            cx.update(|cx| {
                                if let Some(window_handle) = cx.active_window() {
                                    let _ = cx.update_window(window_handle, |any_view, window, cx| {
                                        if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                                            view_handle.update(cx, |view, cx| {
                                                view.editor.update(cx, |editor, cx| {
                                                    editor.focus_handle.focus(window, cx);
                                                });
                                            });
                                        }
                                    });
                                }
                            });
                        }
                    }).detach();
                }
                DialogEvent::Save(intent) => {
                    let intent = *intent;
                    this.workspace.update(cx, |w, cx| {
                        let _ = w.active_editor_mut().save();
                        cx.notify();
                    });
                    this.dialog = None;
                    cx.notify();
                    
                    cx.spawn(move |_, cx: &mut AsyncApp| {
                        let cx = cx.clone();
                        async move {
                            cx.update(|cx| {
                                // Restore focus
                                if let Some(window_handle) = cx.active_window() {
                                    let _ = cx.update_window(window_handle, |any_view, window, cx| {
                                        if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                                            view_handle.update(cx, |view, cx| {
                                                view.editor.update(cx, |editor, cx| {
                                                    editor.focus_handle.focus(window, cx);
                                                });
                                            });
                                        }
                                    });
                                }
                                
                                // Dispatch action
                                match intent {
                                    UnsavedChangesIntent::Quit => cx.dispatch_action(&Quit {}),
                                    UnsavedChangesIntent::CloseTab => cx.dispatch_action(&CloseTab {}),
                                }
                            });
                        }
                    }).detach();
                }
                DialogEvent::DontSave(intent) => {
                    let intent = *intent;
                    this.workspace.update(cx, |w, cx| {
                        w.close_active_editor();
                        cx.notify();
                    });
                    this.dialog = None;
                    cx.notify();
                    
                    cx.spawn(move |_, cx: &mut AsyncApp| {
                        let cx = cx.clone();
                        async move {
                            cx.update(|cx| {
                                // Restore focus
                                if let Some(window_handle) = cx.active_window() {
                                    let _ = cx.update_window(window_handle, |any_view, window, cx| {
                                        if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                                            view_handle.update(cx, |view, cx| {
                                                view.editor.update(cx, |editor, cx| {
                                                    editor.focus_handle.focus(window, cx);
                                                });
                                            });
                                        }
                                    });
                                }
                                
                                // Dispatch action
                                if intent == UnsavedChangesIntent::Quit {
                                    cx.dispatch_action(&Quit {});
                                }
                            });
                        }
                    }).detach();
                }
                _ => {}
            }
        }).detach();
        self.dialog = Some(dialog.clone());
        dialog.update(cx, |d, cx| d.focus(window, cx));
        cx.notify();
    }

    fn handle_new(&mut self, _: &New, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.add_editor(Editor::new());
            cx.notify();
        });
    }

    fn handle_open(&mut self, _: &Open, _window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.workspace.clone();
        cx.spawn(|_, cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                let files = rfd::AsyncFileDialog::new()
                    .pick_files()
                    .await;
                
                if let Some(files) = files {
                    for file in files {
                        let path = file.path().to_path_buf();
                        if let Ok(editor) = Editor::from_file(&path) {
                            cx.update(|cx| {
                                workspace.update(cx, |w, cx| {
                                    w.add_editor(editor);
                                    cx.notify();
                                });
                            });
                        }
                    }
                }
            }
        }).detach();
    }

    fn handle_save(&mut self, _: &Save, _window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.workspace.clone();
        let editor = workspace.read(cx).active_editor();
        if let Some(_path) = editor.path.clone() {
            workspace.update(cx, |w, cx| {
                let _ = w.active_editor_mut().save();
                cx.notify();
            });
        } else {
            cx.spawn(|_, cx: &mut AsyncApp| {
                let cx = cx.clone();
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .save_file()
                        .await;
                    if let Some(file) = file {
                        let path = file.path().to_path_buf();
                        cx.update(|cx| {
                            workspace.update(cx, |w, cx| {
                                let _ = w.active_editor_mut().save_as(&path);
                                cx.notify();
                            });
                        });
                    }
                }
            }).detach();
        }
    }

    fn handle_save_as(&mut self, _: &SaveAs, _window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.workspace.clone();
        cx.spawn(|_, cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .save_file()
                    .await;
                
                if let Some(file) = file {
                    let path = file.path().to_path_buf();
                    cx.update(|cx| {
                        workspace.update(cx, |w, cx| {
                            let _ = w.active_editor_mut().save_as(&path);
                            cx.notify();
                        });
                    });
                }
            }
        }).detach();
    }

    fn handle_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        let is_modified = self.workspace.read(cx).active_editor().is_modified();
        if is_modified {
            let filename = self.workspace.read(cx).active_editor().path.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or(self.i18n.get("status.no_name").to_string());
            self.show_dialog(DialogType::UnsavedChanges { filename, intent: UnsavedChangesIntent::CloseTab }, window, cx);
        } else {
            self.workspace.update(cx, |w, cx| {
                w.close_active_editor();
                cx.notify();
            });
        }
    }

    fn handle_next_tab(&mut self, _: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.next_tab();
            cx.notify();
        });
    }

    fn handle_prev_tab(&mut self, _: &PrevTab, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.prev_tab();
            cx.notify();
        });
    }

    fn handle_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().undo();
            cx.notify();
        });
    }

    fn handle_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().redo();
            cx.notify();
        });
    }

    fn handle_cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        let mut text_to_copy = None;
        self.workspace.update(cx, |w, cx| {
            let editor = w.active_editor_mut();
            if let Some(range) = editor.selection.clone() {
                text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                editor.delete(range);
            }
            cx.notify();
        });
        if let Some(text) = text_to_copy {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn handle_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor();
        if let Some(range) = editor.selection.clone() {
            let text = editor.rope.slice(range).to_string();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn handle_paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                let text = text.clone();
                self.workspace.update(cx, |w, cx| {
                    let editor = w.active_editor_mut();
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    }
                    editor.insert(editor.cursor, &text);
                    cx.notify();
                });
            }
        }
    }

    fn handle_select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            let editor = w.active_editor_mut();
            editor.select_all();
            cx.notify();
        });
    }

    fn handle_find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
        self.find_panel.update(cx, |p, cx| p.show(false, window, cx));
    }

    fn handle_replace(&mut self, _: &Replace, window: &mut Window, cx: &mut Context<Self>) {
        self.find_panel.update(cx, |p, cx| p.show(true, window, cx));
    }

    fn handle_toggle_line_numbers(&mut self, _: &ToggleLineNumbers, _window: &mut Window, cx: &mut Context<Self>) {
        self.config.line_numbers = !self.config.line_numbers;
        let _ = Config::write_key("line_numbers", &self.config.line_numbers.to_string());
        cx.notify();
    }

    fn handle_toggle_word_wrap(&mut self, _: &ToggleWordWrap, _window: &mut Window, cx: &mut Context<Self>) {
        self.config.word_wrap = !self.config.word_wrap;
        let _ = Config::write_key("word_wrap", &self.config.word_wrap.to_string());
        cx.notify();
    }

    fn handle_toggle_vi_mode(&mut self, _: &ToggleViMode, _window: &mut Window, cx: &mut Context<Self>) {
        self.config.vi_mode = !self.config.vi_mode;
        let _ = Config::write_key("vi_mode", &self.config.vi_mode.to_string());
        self.workspace.update(cx, |w, cx| {
            for editor in w.editors.iter_mut() {
                editor.vi_mode = if self.config.vi_mode { led_core::ViMode::Normal } else { led_core::ViMode::Insert };
            }
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_utf8(&mut self, _: &SetEncodingUtf8, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::Utf8;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_shift_jis(&mut self, _: &SetEncodingShiftJis, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::ShiftJis;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_euc_jp(&mut self, _: &SetEncodingEucJp, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::EucJp;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_utf8_bom(&mut self, _: &SetEncodingUtf8Bom, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::Utf8Bom;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_utf16_le(&mut self, _: &SetEncodingUtf16Le, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::Utf16Le;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_utf16_be(&mut self, _: &SetEncodingUtf16Be, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::Utf16Be;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_iso_2022_jp(&mut self, _: &SetEncodingIso2022Jp, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::Iso2022Jp;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_encoding_latin1(&mut self, _: &SetEncodingLatin1, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = led_core::Encoding::Latin1;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_line_ending_lf(&mut self, _: &SetLineEndingLf, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().line_ending = led_core::LineEnding::Lf;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_line_ending_crlf(&mut self, _: &SetLineEndingCrlf, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().line_ending = led_core::LineEnding::Crlf;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_line_ending_cr(&mut self, _: &SetLineEndingCr, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().line_ending = led_core::LineEnding::Cr;
            cx.notify();
        });
        cx.notify();
    }

    fn handle_set_theme(&mut self, action: &SetTheme, _window: &mut Window, cx: &mut Context<Self>) {
        self.set_theme(&action.name, cx);
    }

    fn handle_set_syntax(&mut self, action: &SetSyntax, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            let _buffer = w.active_editor_mut();
            // In a real app we'd find the syntax definition by name
            // For now we'll just log or stub it
            println!("Setting syntax to {}", action.name);
            cx.notify();
        });
        cx.notify();
    }

    fn set_theme(&mut self, name: &str, cx: &mut Context<Self>) {
        let theme = led_core::theme::Theme::builtins().into_iter()
            .find(|t| t.meta.name == name)
            .unwrap_or_default();
        
        self.workspace.update(cx, |w, cx| {
            w.theme = theme;
            cx.notify();
        });
        
        let theme_file_name = match name {
            "Tokyo Night" => "tokyo-night",
            "Solarized Dark" => "solarized-dark",
            "Solarized Light" => "solarized-light",
            "Catppuccin Mocha" => "catppuccin-mocha",
            "Catppuccin Latte" => "catppuccin-latte",
            "Light" => "light",
            _ => "tokyo-night",
        };
        let _ = Config::write_key("theme", theme_file_name);
        cx.notify();
    }

    fn handle_go_to_line(&mut self, _: &GoToLine, window: &mut Window, cx: &mut Context<Self>) {
        self.show_dialog(DialogType::GoToLine, window, cx);
    }

    pub fn has_modified_buffers(&self, cx: &App) -> bool {
        self.workspace.read(cx).has_modified_buffers()
    }

    pub fn handle_about(&mut self, _: &About, window: &mut Window, cx: &mut Context<Self>) {
        self.show_dialog(DialogType::About, window, cx);
    }

    pub fn handle_quit(&mut self, _action: &Quit, window: &mut Window, cx: &mut Context<Self>) {
        let mut modified_file = None;
        let mut target_idx = None;
        
        self.workspace.update(cx, |w, _| {
            for (idx, editor) in w.editors.iter().enumerate() {
                if editor.is_modified() {
                    modified_file = Some(editor.path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or(self.i18n.get("status.no_name").to_string()));
                    target_idx = Some(idx);
                    break;
                }
            }
            if let Some(idx) = target_idx {
                w.active_editor_index = idx;
            }
        });

        if let Some(filename) = modified_file {
            self.show_dialog(DialogType::UnsavedChanges { filename, intent: UnsavedChangesIntent::Quit }, window, cx);
        } else {
            cx.propagate();
        }
    }

    pub fn handle_exit(&mut self, _action: &Exit, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_modified_buffers(cx) {
            self.handle_quit(&Quit {}, window, cx);
        } else {
            cx.propagate();
        }
    }

    fn handle_reopen_with_encoding(&mut self, action: &ReopenWithEncoding, _window: &mut Window, cx: &mut Context<Self>) {
        let enc = self.parse_encoding(&action.encoding);
        self.workspace.update(cx, |w, cx| {
            let buffer = w.active_editor();
            if let Some(path) = buffer.path.clone() {
                if let Ok(mut new_buffer) = Editor::from_file(&path) {
                    new_buffer.encoding = enc;
                    w.editors[w.active_editor_index] = new_buffer;
                    cx.notify();
                }
            }
        });
        cx.notify();
    }

    fn handle_convert_to_encoding(&mut self, action: &ConvertToEncoding, _window: &mut Window, cx: &mut Context<Self>) {
        let enc = self.parse_encoding(&action.encoding);
        self.workspace.update(cx, |w, cx| {
            w.active_editor_mut().encoding = enc;
            cx.notify();
        });
        cx.notify();
    }

    fn parse_encoding(&self, name: &str) -> led_core::Encoding {
        match name {
            "UTF-8" => led_core::Encoding::Utf8,
            "UTF-8 with BOM" => led_core::Encoding::Utf8Bom,
            "UTF-16 LE" => led_core::Encoding::Utf16Le,
            "UTF-16 BE" => led_core::Encoding::Utf16Be,
            "Shift-JIS" => led_core::Encoding::ShiftJis,
            "EUC-JP" => led_core::Encoding::EucJp,
            "ISO-2022-JP" => led_core::Encoding::Iso2022Jp,
            "Latin-1" => led_core::Encoding::Latin1,
            _ => led_core::Encoding::Utf8,
        }
    }

    fn led_color_to_gpui(&self, color: led_core::theme::Color) -> Rgba {
        match color {
            led_core::theme::Color::Rgb(r, g, b) => {
                Rgba {
                    r: r as f32 / 255.0,
                    g: g as f32 / 255.0,
                    b: b as f32 / 255.0,
                    a: 1.0,
                }
            }
            led_core::theme::Color::Ansi(i) => {
                let (r, g, b) = match i {
                    0 => (0, 0, 0),
                    1 => (170, 0, 0),
                    2 => (0, 170, 0),
                    3 => (170, 170, 0),
                    4 => (0, 0, 170),
                    5 => (170, 0, 170),
                    6 => (0, 170, 170),
                    7 => (170, 170, 170),
                    8 => (85, 85, 85),
                    9 => (255, 85, 85),
                    10 => (85, 255, 85),
                    11 => (255, 255, 85),
                    12 => (85, 85, 255),
                    13 => (255, 85, 255),
                    14 => (85, 255, 255),
                    15 => (255, 255, 255),
                    _ => (128, 128, 128),
                };
                Rgba {
                    r: r as f32 / 255.0,
                    g: g as f32 / 255.0,
                    b: b as f32 / 255.0,
                    a: 1.0,
                }
            }
        }
    }
}

impl Render for WindowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;
        let bg = self.led_color_to_gpui(theme.editor.background);

        div()
            .w_full()
            .h_full()
            .relative() // So dialog can be absolute
            .bg(bg)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_new))
            .on_action(cx.listener(Self::handle_open))
            .on_action(cx.listener(Self::handle_save))
            .on_action(cx.listener(Self::handle_save_as))
            .on_action(cx.listener(Self::handle_close_tab))
            .on_action(cx.listener(Self::handle_next_tab))
            .on_action(cx.listener(Self::handle_prev_tab))
            .on_action(cx.listener(Self::handle_undo))
            .on_action(cx.listener(Self::handle_redo))
            .on_action(cx.listener(Self::handle_cut))
            .on_action(cx.listener(Self::handle_copy))
            .on_action(cx.listener(Self::handle_paste))
            .on_action(cx.listener(Self::handle_select_all))
            .on_action(cx.listener(Self::handle_find))
            .on_action(cx.listener(Self::handle_replace))
            .on_action(cx.listener(Self::handle_toggle_line_numbers))
            .on_action(cx.listener(Self::handle_toggle_word_wrap))
            .on_action(cx.listener(Self::handle_toggle_vi_mode))
            .on_action(cx.listener(Self::handle_set_encoding_utf8))
            .on_action(cx.listener(Self::handle_set_encoding_utf8_bom))
            .on_action(cx.listener(Self::handle_set_encoding_utf16_le))
            .on_action(cx.listener(Self::handle_set_encoding_utf16_be))
            .on_action(cx.listener(Self::handle_set_encoding_shift_jis))
            .on_action(cx.listener(Self::handle_set_encoding_euc_jp))
            .on_action(cx.listener(Self::handle_set_encoding_iso_2022_jp))
            .on_action(cx.listener(Self::handle_set_encoding_latin1))
            .on_action(cx.listener(Self::handle_reopen_with_encoding))
            .on_action(cx.listener(Self::handle_convert_to_encoding))
            .on_action(cx.listener(Self::handle_set_line_ending_lf))
            .on_action(cx.listener(Self::handle_set_line_ending_crlf))
            .on_action(cx.listener(Self::handle_set_line_ending_cr))
            .on_action(cx.listener(Self::handle_set_theme))
            .on_action(cx.listener(Self::handle_set_syntax))
            .on_action(cx.listener(Self::handle_go_to_line))
            .on_action(cx.listener(Self::handle_about))
            .on_action(cx.listener(Self::handle_quit))
            .on_action(cx.listener(Self::handle_exit))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                let mut opened_any = false;
                for path in paths.paths() {
                    if let Ok(editor) = Editor::from_file(path) {
                        this.workspace.update(cx, |w, cx| {
                            w.add_editor(editor);
                            cx.notify();
                        });
                        opened_any = true;
                    }
                }
                if opened_any {
                    cx.notify();
                }
            }))
            .child(self.render_layout())
            .child(if let Some(ref dialog) = self.dialog {
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h_full()
                    .child(dialog.clone())
            } else {
                div()
            })
    }
}

impl WindowView {
    fn render_layout(&self) -> impl IntoElement {
        let container = div().w_full().h_full().flex().flex_col();

        #[cfg(not(target_os = "macos"))]
        let container = container.child(self.menu_bar.clone());

        container
            .child(self.tab_bar.clone())
            .child(self.find_panel.clone())
            .child(div().flex_grow().child(self.editor.clone()))
            .child(self.status_bar.clone())
    }
}
