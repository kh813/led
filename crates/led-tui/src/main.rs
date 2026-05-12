mod app;
mod renderer;
mod layout;
mod clipboard;
mod widgets;

use std::path::PathBuf;
use app::App;
use anyhow::Result;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();
    
    let mut app = App::new(paths)?;
    app.run()
}
