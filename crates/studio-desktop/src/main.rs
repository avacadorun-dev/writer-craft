//! WriterCraft native executable; the domain engine is shared with the CLI.
mod app;
pub mod icons;
pub mod theme;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut database = default_database();
    let mut demo = false;
    let mut capture = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--database" => database = arguments.next().map(PathBuf::from).unwrap_or(database),
            "--demo" => demo = true,
            "--capture" => capture = arguments.next().map(PathBuf::from),
            _ => {}
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WriterCraft")
            .with_app_id("app.writercraft.desktop")
            .with_inner_size([1440.0, 940.0])
            .with_min_inner_size([1000.0, 680.0])
            .with_fullsize_content_view(true)
            .with_title_shown(false)
            .with_titlebar_shown(false),
        ..Default::default()
    };
    eframe::run_native(
        "WriterCraft",
        options,
        Box::new(move |cc| Ok(Box::new(app::StudioApp::new(cc, database, demo, capture)?))),
    )
}

fn default_database() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    #[cfg(target_os = "macos")]
    let base = home.join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or(home);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    base.join("WriterCraft/workspace.sqlite")
}
