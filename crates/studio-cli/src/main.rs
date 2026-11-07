//! Headless access to the same project core used by the native desktop.

use std::error::Error;
use std::path::PathBuf;
use studio_core::Workspace;

const USAGE: &str = "Usage: studio demo --database PATH | list --database PATH | verify --database PATH | inspect PATH.wstudio";

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        println!("{USAGE}");
        return Ok(());
    };
    if command == "--help" || command == "-h" {
        println!("{USAGE}");
        return Ok(());
    }
    if command == "inspect" {
        let path = arguments.next().ok_or(USAGE)?;
        if arguments.next().is_some() {
            return Err(USAGE.into());
        }
        let preview = studio_transfer::inspect_archive(PathBuf::from(path))?;
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }
    if command != "demo" && command != "list" && command != "verify" {
        return Err(USAGE.into());
    }
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new("--database")) {
        return Err(USAGE.into());
    }
    let path = PathBuf::from(arguments.next().ok_or(USAGE)?);
    if arguments.next().is_some() {
        return Err(USAGE.into());
    }
    if command != "demo" && !path.is_file() {
        return Err(format!("Database does not exist: {}", path.display()).into());
    }
    let workspace = Workspace::open(&path)?;
    if command == "demo" {
        let project = workspace.create_project("My first book")?;
        let document = workspace.create_document(
            &project.id,
            "First scene",
            "The morning began with a letter.\n\nEvery word has its own story.",
        )?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "database":path, "project":project, "document":document,
                "note":"Sample content created locally; no AI provider was called."
            }))?
        );
    } else if command == "list" {
        let mut projects = Vec::new();
        for project in workspace.list_projects()? {
            let documents = workspace.list_documents(&project.id)?;
            projects.push(serde_json::json!({"project":project,"documents":documents}));
        }
        println!("{}", serde_json::to_string_pretty(&projects)?);
    } else {
        let mut document_count = 0;
        for project in workspace.list_projects()? {
            for document in workspace.list_documents(&project.id)? {
                let current = workspace.document(&document.id)?;
                let revisions = workspace.revisions(&document.id)?;
                if revisions.first().is_none_or(|revision| {
                    revision.revision != current.revision || revision.body != current.body
                }) {
                    return Err(format!(
                        "Document {} has inconsistent revision history",
                        current.id
                    )
                    .into());
                }
                document_count += 1;
            }
        }
        println!(
            "{}",
            serde_json::json!({"status":"ok","documents_checked":document_count,
            "scope":"Core can read projects, documents and current revision histories; this is not a full SQLite integrity audit."})
        );
    }
    Ok(())
}
