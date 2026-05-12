use gpui::*;
use led_core::config::Config;
use led_core::i18n::I18n;
use led_core::theme::Theme;
use crate::window_view::WindowView;
use crate::workspace::Workspace;
use anyhow::Result;
use serde_json;
use futures::StreamExt;
use url::Url;

pub fn setup_app(app: &mut App, rx: futures::channel::mpsc::UnboundedReceiver<Vec<String>>) {
    let config = Config::load();
    let i18n = I18n::load(&config.language);

    setup_menu(app, &i18n);

    // Global key bindings
    app.bind_keys(vec![
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-n", New {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-o", Open {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-s", Save {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-s", SaveAs {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-w", CloseTab {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", Undo {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", Redo {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-y", Redo {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-f", Find {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-h", Replace {}, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll {}, None),

        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-n", New {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", Open {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-s", Save {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-s", SaveAs {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-w", CloseTab {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-q", Quit {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", Undo {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", Redo {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-f", Find {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-h", Replace {}, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll {}, None),
    ]);

    // App-level action handlers to handle actions when no window is open
    let config_new = config.clone();
    let i18n_new = i18n.clone();
    app.on_action(move |_: &New, cx| {
        new_window(config_new.clone(), i18n_new.clone(), cx);
    });

    let config_open = config.clone();
    let i18n_open = i18n.clone();
    app.on_action(move |_: &Open, cx| {
        let config = config_open.clone();
        let i18n = i18n_open.clone();
        cx.spawn(|cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                let files = rfd::AsyncFileDialog::new().pick_files().await;
                if let Some(files) = files {
                    let paths: Vec<_> = files.into_iter().map(|f| f.path().to_path_buf()).collect();
                    cx.update(|cx| {
                        cx.open_window(centered_window_options(cx), move |window, cx| {
                            let workspace = cx.new(|_| {
                                let mut w = Workspace::new(config.clone());
                                for path in paths {
                                    if let Ok(editor) = led_core::buffer::Editor::from_file(&path) {
                                        w.add_editor(editor);
                                    }
                                }
                                w
                            });
                            cx.new(|cx| WindowView::new(config.clone(), i18n.clone(), workspace, window, cx))
                        }).expect("Failed to open window");
                    });
                }
            }
        }).detach();
    });

    let i18n_about = i18n.clone();
    app.on_action(move |_: &About, cx| {
        let i18n = i18n_about.clone();
        cx.open_window(centered_window_options(cx), move |window, cx| {
            let workspace = cx.new(|_| Workspace::new(Config::default()));
            cx.new(|cx| {
                let mut view = WindowView::new(Config::default(), i18n.clone(), workspace, window, cx);
                // Immediately show about dialog
                view.handle_about(&About {}, window, cx);
                view
            })
        }).expect("Failed to open about window");
    });

    app.on_action(|_: &Quit, cx| {
        for hw in cx.windows() {
            let modified = cx.update_window(hw, |any_view, _window, cx| {
                if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                    view_handle.read(cx).has_modified_buffers(cx)
                } else {
                    false
                }
            }).unwrap_or(false);

            if modified {
                let _ = cx.update_window(hw, |any_view, window, cx| {
                    window.activate_window();
                    if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                        view_handle.update(cx, |view, cx| {
                            view.handle_quit(&Quit {}, window, cx);
                        });
                    }
                });
                return;
            }
        }
        cx.quit();
    });

    app.on_action(|_: &Exit, cx| {
        for hw in cx.windows() {
            let modified = cx.update_window(hw, |any_view, _window, cx| {
                if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                    view_handle.read(cx).has_modified_buffers(cx)
                } else {
                    false
                }
            }).unwrap_or(false);

            if modified {
                let _ = cx.update_window(hw, |any_view, window, cx| {
                    window.activate_window();
                    if let Ok(view_handle) = any_view.downcast::<WindowView>() {
                        view_handle.update(cx, |view, cx| {
                            view.handle_quit(&Quit {}, window, cx);
                        });
                    }
                });
                return;
            }
        }
        cx.quit();
    });

    // Initial window
    new_window(config.clone(), i18n.clone(), app);

    // Handle files dropped on the Dock icon or opened via Finder
    app.spawn(|cx: &mut AsyncApp| {
        let cx = cx.clone();
        let mut rx = rx;
        async move {
            while let Some(urls) = rx.next().await {
                let config = Config::load();
                let i18n = I18n::load(&config.language);
                let paths: Vec<_> = urls.into_iter()
                    .filter_map(|u| {
                        if let Ok(url) = Url::parse(&u) {
                            url.to_file_path().ok()
                        } else {
                            Some(std::path::PathBuf::from(u))
                        }
                    })
                    .collect();
                
                if !paths.is_empty() {
                    cx.update(|cx| {
                        open_paths(paths, config, i18n, cx);
                    });
                }
            }
        }
    }).detach();
}

fn centered_window_options(cx: &App) -> WindowOptions {
    let window_size = size(px(1008.0), px(826.0));
    let mut origin = Point::default();
    if let Some(display) = cx.primary_display() {
        let display_bounds = display.bounds();
        origin.x = display_bounds.origin.x + (display_bounds.size.width - window_size.width) / 2.0;
        origin.y = display_bounds.origin.y + (display_bounds.size.height - window_size.height) / 2.0;
    }

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin,
            size: window_size,
        })),
        ..Default::default()
    }
}

pub fn new_window(config: Config, i18n: I18n, cx: &mut App) {
    let themes = Theme::builtins();
    
    let theme_to_use = if config.theme == "terminal-default" || config.theme.is_empty() {
        match cx.window_appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                themes.iter().find(|t| t.meta.name == "Tokyo Night").cloned().unwrap_or_else(Theme::default)
            }
            WindowAppearance::Light | WindowAppearance::VibrantLight => {
                themes.iter().find(|t| t.meta.name == "Catppuccin Latte").cloned().unwrap_or_else(Theme::default)
            }
        }
    } else {
        themes.iter()
            .find(|t| t.meta.name.to_lowercase().replace(" ", "-") == config.theme.to_lowercase())
            .cloned()
            .unwrap_or_else(Theme::default)
    };

    let options = centered_window_options(cx);
    cx.open_window(options, move |window, cx| {
        let workspace = cx.new(|_| {
            let mut w = Workspace::new(config.clone());
            w.theme = theme_to_use;
            w
        });
        cx.new(|cx| WindowView::new(config, i18n, workspace, window, cx))
    }).expect("Failed to open window");
}

pub fn open_paths(paths: Vec<std::path::PathBuf>, config: Config, i18n: I18n, cx: &mut App) {
    let themes = Theme::builtins();
    
    let theme_to_use = if config.theme == "terminal-default" || config.theme.is_empty() {
        match cx.window_appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                themes.iter().find(|t| t.meta.name == "Tokyo Night").cloned().unwrap_or_else(Theme::default)
            }
            WindowAppearance::Light | WindowAppearance::VibrantLight => {
                themes.iter().find(|t| t.meta.name == "Catppuccin Latte").cloned().unwrap_or_else(Theme::default)
            }
        }
    } else {
        themes.iter()
            .find(|t| t.meta.name.to_lowercase().replace(" ", "-") == config.theme.to_lowercase())
            .cloned()
            .unwrap_or_else(Theme::default)
    };

    let options = centered_window_options(cx);
    cx.open_window(options, move |window, cx| {
        let workspace = cx.new(|_| {
            let mut w = Workspace::new(config.clone());
            w.theme = theme_to_use;
            let mut opened_any = false;
            for path in paths {
                if let Ok(editor) = led_core::buffer::Editor::from_file(&path) {
                    w.add_editor(editor);
                    opened_any = true;
                }
            }
            if !opened_any {
                w.add_editor(led_core::buffer::Editor::new());
            }
            w
        });
        cx.new(|cx| WindowView::new(config, i18n, workspace, window, cx))
    }).expect("Failed to open window");
}

#[cfg(target_os = "macos")]
fn setup_menu(app: &mut App, i18n: &I18n) {
    app.set_menus(build_native_menus(i18n));
}

#[cfg(not(target_os = "macos"))]
fn setup_menu(_app: &mut App, _i18n: &I18n) {
    // In-window menu bar is handled in window_view.rs
}

#[derive(serde::Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct SetTheme {
    pub name: String,
}

impl Action for SetTheme {
    fn name(&self) -> &'static str { "SetTheme" }
    fn boxed_clone(&self) -> Box<dyn Action> { Box::new(self.clone()) }
    fn build(v: gpui::private::serde_json::Value) -> Result<Box<dyn Action>> { Ok(Box::new(serde_json::from_value::<Self>(v)?)) }
    fn partial_eq(&self, _other: &dyn Action) -> bool { false }
    fn name_for_type() -> &'static str { "SetTheme" }
}

#[derive(serde::Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ReopenWithEncoding {
    pub encoding: String,
}

impl Action for ReopenWithEncoding {
    fn name(&self) -> &'static str { "ReopenWithEncoding" }
    fn boxed_clone(&self) -> Box<dyn Action> { Box::new(self.clone()) }
    fn build(v: gpui::private::serde_json::Value) -> Result<Box<dyn Action>> { Ok(Box::new(serde_json::from_value::<Self>(v)?)) }
    fn partial_eq(&self, _other: &dyn Action) -> bool { false }
    fn name_for_type() -> &'static str { "ReopenWithEncoding" }
}

#[derive(serde::Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ConvertToEncoding {
    pub encoding: String,
}

impl Action for ConvertToEncoding {
    fn name(&self) -> &'static str { "ConvertToEncoding" }
    fn boxed_clone(&self) -> Box<dyn Action> { Box::new(self.clone()) }
    fn build(v: gpui::private::serde_json::Value) -> Result<Box<dyn Action>> { Ok(Box::new(serde_json::from_value::<Self>(v)?)) }
    fn partial_eq(&self, _other: &dyn Action) -> bool { false }
    fn name_for_type() -> &'static str { "ConvertToEncoding" }
}

#[derive(serde::Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct SetSyntax {
    pub name: String,
}

impl Action for SetSyntax {
    fn name(&self) -> &'static str { "SetSyntax" }
    fn boxed_clone(&self) -> Box<dyn Action> { Box::new(self.clone()) }
    fn build(v: gpui::private::serde_json::Value) -> Result<Box<dyn Action>> { Ok(Box::new(serde_json::from_value::<Self>(v)?)) }
    fn partial_eq(&self, _other: &dyn Action) -> bool { false }
    fn name_for_type() -> &'static str { "SetSyntax" }
}

#[cfg(target_os = "macos")]
fn build_native_menus(i18n: &I18n) -> Vec<Menu> {
    let mut theme_items = Vec::new();
    for theme in led_core::theme::Theme::builtins() {
        theme_items.push(MenuItem::action(
            theme.meta.name.clone(),
            SetTheme { name: theme.meta.name.clone() },
        ));
    }

    let mut syntax_items = Vec::new();
    // In a real app we'd load these from core, but for now we'll hardcode or use builtins if available
    for syntax in ["Plain Text", "Markdown", "Rust", "TOML", "Python", "Go", "Swift", "JavaScript", "HTML", "CSS", "XML"] {
        syntax_items.push(MenuItem::action(
            syntax,
            SetSyntax { name: syntax.to_string() },
        ));
    }

    let encodings = vec![
        "UTF-8", "UTF-8 with BOM", "UTF-16 LE", "UTF-16 BE", 
        "Shift-JIS", "EUC-JP", "ISO-2022-JP", "Latin-1"
    ];

    let reopen_items = encodings.iter().map(|e| {
        MenuItem::action(e.to_string(), ReopenWithEncoding { encoding: e.to_string() })
    }).collect();

    let convert_items = encodings.iter().map(|e| {
        MenuItem::action(e.to_string(), ConvertToEncoding { encoding: e.to_string() })
    }).collect();

    vec![
        Menu {
            name: "led-gui".into(),
            items: vec![
                MenuItem::action("About led-gui", About {}),
                MenuItem::separator(),
                MenuItem::action("Quit led-gui", Quit {}),
            ],
            disabled: false,
        },
        Menu {
            name: i18n.get("menu.file").into(),
            items: vec![
                MenuItem::action(i18n.get("menu.file.new"), New {}),
                MenuItem::action(i18n.get("menu.file.open"), Open {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.file.save"), Save {}),
                MenuItem::action(i18n.get("menu.file.save_as"), SaveAs {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.file.close"), CloseTab {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.file.exit"), Exit {}),
            ],
            disabled: false,
        },
        Menu {
            name: i18n.get("menu.edit").into(),
            items: vec![
                MenuItem::action(i18n.get("menu.edit.undo"), Undo {}),
                MenuItem::action(i18n.get("menu.edit.redo"), Redo {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.edit.cut"), Cut {}),
                MenuItem::action(i18n.get("menu.edit.copy"), Copy {}),
                MenuItem::action(i18n.get("menu.edit.paste"), Paste {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.edit.find"), Find {}),
                MenuItem::action(i18n.get("menu.edit.replace"), Replace {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.edit.select_all"), SelectAll {}),
            ],
            disabled: false,
        },
        Menu {
            name: i18n.get("menu.view").into(),
            items: vec![
                MenuItem::action(i18n.get("menu.view.go_to_line"), GoToLine {}),
                MenuItem::separator(),
                MenuItem::action(i18n.get("menu.view.line_numbers"), ToggleLineNumbers {}),
                MenuItem::action(i18n.get("menu.view.word_wrap"), ToggleWordWrap {}),
                MenuItem::action(i18n.get("menu.view.vi_mode"), ToggleViMode {}),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: i18n.get("menu.view.encoding").into(),
                    items: vec![
                        MenuItem::submenu(Menu { name: "Reopen with Encoding".into(), items: reopen_items, disabled: false }),
                        MenuItem::submenu(Menu { name: "Convert to Encoding".into(), items: convert_items, disabled: false }),
                    ],
                    disabled: false,
                }),
                MenuItem::submenu(Menu {
                    name: i18n.get("menu.view.line_ending").into(),
                    items: vec![
                        MenuItem::action("LF", SetLineEndingLf {}),
                        MenuItem::action("CRLF", SetLineEndingCrlf {}),
                        MenuItem::action("CR", SetLineEndingCr {}),
                    ],
                    disabled: false,
                }),
                MenuItem::submenu(Menu {
                    name: i18n.get("menu.view.theme").into(),
                    items: theme_items,
                    disabled: false,
                }),
                MenuItem::submenu(Menu {
                    name: i18n.get("menu.view.syntax").into(),
                    items: syntax_items,
                    disabled: false,
                }),
            ],
            disabled: false,
        },
        Menu {
            name: "Tabs".into(),
            items: vec![
                MenuItem::action("Next Tab", NextTab {}),
                MenuItem::action("Previous Tab", PrevTab {}),
            ],
            disabled: false,
        },
        Menu {
            name: i18n.get("menu.help").into(),
            items: vec![
                MenuItem::action(i18n.get("menu.help.about"), About {}),
            ],
            disabled: false,
        }
    ]
}

actions!(led, [
    // App/File
    About, Quit, Exit, New, Open, Save, SaveAs, CloseTab,
    // Edit
    Undo, Redo, Cut, Copy, Paste, Find, Replace, SelectAll,
    // Tabs
    NextTab, PrevTab,
    // View
    GoToLine, ToggleLineNumbers, ToggleWordWrap, ToggleViMode,
    SetEncodingUtf8, SetEncodingUtf8Bom, SetEncodingUtf16Le, SetEncodingUtf16Be,
    SetEncodingShiftJis, SetEncodingEucJp, SetEncodingIso2022Jp, SetEncodingLatin1,
    SetLineEndingLf, SetLineEndingCrlf, SetLineEndingCr,
    // Search
    SearchNext, SearchPrev, SearchReplace, SearchReplaceAll,
    ToggleMatchCase, ToggleWholeWord, ToggleRegex,
    // Other
    NoOp
]);
