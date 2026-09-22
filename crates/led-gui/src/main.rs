#![windows_subsystem = "windows"]

use gpui::*;

mod app;
mod window_view;
mod widgets;
mod workspace;

use crate::app::setup_app;

fn main() {
    let (tx, rx) = futures::channel::mpsc::unbounded::<Vec<String>>();
    let app = gpui_platform::application();
    
    let tx_urls = tx.clone();
    app.on_open_urls(move |urls| {
        let _ = tx_urls.unbounded_send(urls);
    });

    app.on_reopen(move |cx| {
        if cx.windows().is_empty() {
            let config = led_core::config::Config::load();
            let i18n = led_core::i18n::I18n::load(&config.language);
            crate::app::new_window(config, i18n, cx);
        }
    });

    app.run(|app: &mut App| {
        setup_app(app, rx);
    });
}
