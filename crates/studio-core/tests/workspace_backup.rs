use craft_editor_model::{Document as RichTree, ScalarOffset};
use rusqlite::{Connection, OpenFlags, params};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use studio_core::{Workspace, backup_workspace};

#[test]
fn wal_backup_keeps_histories_notes_drafts_and_opaque_owned_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("live.sqlite");
    let workspace = Workspace::open(&path)?;
    let project = workspace.create_project("Synthetic backup")?;
    let tree = RichTree::plain("p", "Я😀 echo echo")?;
    let document = workspace.create_rich_document(&project.id, "Original", tree.to_json()?)?;
    let anchor = workspace.capture_annotation_anchor(
        &document.id,
        document.revision,
        "p",
        ScalarOffset(8),
        ScalarOffset(12),
        "echo",
    )?;
    let note = workspace.create_annotation(
        &document.id,
        document.revision,
        anchor,
        "Private saved note",
    )?;
    let draft = workspace.save_rich_draft(
        &document.id,
        document.revision,
        None,
        RichTree::plain("p", "Unfinished recovery 😀")?.to_json()?,
    )?;
    let raw = Connection::open(&path)?;
    raw.execute_batch(
        "CREATE TABLE future_owned_artifact (id TEXT PRIMARY KEY, data BLOB NOT NULL)",
    )?;
    let artifact = vec![42_u8; 256 * 1024];
    raw.execute(
        "INSERT INTO future_owned_artifact VALUES('synthetic',?1)",
        params![artifact],
    )?;
    assert!(path.with_extension("sqlite-wal").is_file());
    let output = directory.path().join("backup.sqlite");
    let receipt = backup_workspace(&path, &output, &AtomicBool::new(false))?;
    let bytes = std::fs::read(&output)?;
    assert_eq!(receipt.byte_size, bytes.len() as u64);
    assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(&bytes)));
    // A later source commit cannot enter the already published snapshot.
    workspace.rename_document(&project.id, &document.id, "Original", "Later")?;
    let copy = Workspace::open(&output)?;
    assert_eq!(copy.document(&document.id)?, document);
    assert_eq!(copy.rich_draft(&document.id)?, Some(draft));
    assert_eq!(copy.annotations(&document.id)?, vec![note]);
    assert_eq!(
        copy.rich_revisions(&document.id)?,
        workspace.rich_revisions(&document.id)?
    );
    let copied = Connection::open_with_flags(&output, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let retained: Vec<u8> = copied.query_row(
        "SELECT data FROM future_owned_artifact WHERE id='synthetic'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(retained, artifact);
    Ok(())
}

#[test]
fn backup_never_clobbers_existing_output_or_a_source_alias()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source.sqlite");
    let workspace = Workspace::open(&source)?;
    let project = workspace.create_project("Keep source")?;
    let existing = directory.path().join("existing.sqlite");
    std::fs::write(&existing, b"unrelated original")?;
    let cancel = AtomicBool::new(false);
    assert!(backup_workspace(&source, &existing, &cancel).is_err());
    assert_eq!(std::fs::read(&existing)?, b"unrelated original");
    assert!(backup_workspace(&source, &source, &cancel).is_err());
    let alias = directory.path().join("alias.sqlite");
    std::fs::hard_link(&source, &alias)?;
    assert!(backup_workspace(&source, &alias, &cancel).is_err());
    assert_eq!(workspace.list_projects()?, vec![project]);
    Ok(())
}

#[test]
fn cancelled_or_invalid_backup_publishes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("source.sqlite");
    let _workspace = Workspace::open(&path)?;
    let output = directory.path().join("cancelled.sqlite");
    assert!(backup_workspace(&path, &output, &AtomicBool::new(true)).is_err());
    assert!(!output.exists());
    let foreign = directory.path().join("foreign.sqlite");
    let connection = Connection::open(&foreign)?;
    connection
        .execute_batch("CREATE TABLE original(value TEXT);INSERT INTO original VALUES('keep')")?;
    let rejected = directory.path().join("rejected.sqlite");
    assert!(backup_workspace(&foreign, &rejected, &AtomicBool::new(false)).is_err());
    assert!(!rejected.exists());
    assert_eq!(
        connection.query_row("SELECT value FROM original", [], |row| row
            .get::<_, String>(0))?,
        "keep"
    );
    assert!(
        std::fs::read_dir(directory.path())?.all(|entry| entry.is_ok_and(|e| !e
            .file_name()
            .to_string_lossy()
            .starts_with(".writercraft-backup-")))
    );
    Ok(())
}
