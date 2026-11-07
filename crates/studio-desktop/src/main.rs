//! WriterCraft native executable; the domain engine is shared with the CLI.
mod app;
mod brand;
pub mod i18n;
pub mod icons;
pub mod theme;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut database = std::env::var_os("WRITERCRAFT_DATABASE")
        .map(PathBuf::from)
        .unwrap_or_else(default_database);
    let mut demo = std::env::var("WRITERCRAFT_DEMO").as_deref() == Ok("1");
    let mut compact = false;
    let mut capture = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--database" => database = arguments.next().map(PathBuf::from).unwrap_or(database),
            "--demo" => demo = true,
            "--compact" => compact = true,
            "--version" | "-V" => {
                println!("WriterCraft {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "WriterCraft {}\n\nUsage: writercraft [--database PATH] [--demo]\n\n--demo       Create a local sample project if the library is empty\n--database   Use an isolated workspace database\n--version    Print the application version",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
            "--capture" => capture = arguments.next().map(PathBuf::from),
            _ => {}
        }
    }
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/WriterCraft.png"))
        .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WriterCraft")
            .with_icon(std::sync::Arc::new(icon))
            .with_app_id("app.writercraft.desktop")
            .with_inner_size(if compact {
                [1000.0, 680.0]
            } else {
                [1440.0, 940.0]
            })
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
