use gpui::*;

mod app;
mod window_view;
mod widgets;
mod workspace;

use crate::app::setup_app;

fn main() {
    let (tx, rx) = futures::channel::mpsc::unbounded::<Vec<String>>();
    let app = gpui_platform::application();
    
    app.on_open_urls(move |urls| {
        let _ = tx.unbounded_send(urls);
    });

    app.run(|app: &mut App| {
        setup_app(app, rx);
    });
}
