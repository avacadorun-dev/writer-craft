use std::{path::PathBuf, sync::atomic::AtomicBool};
use studio_core::{Workspace, backup_workspace, is_workspace_data_path};

#[test]
fn reserved_preference_database_names_are_rejected_before_creating_files()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    for name in [
        "appearance.json",
        "interface.json",
        "editor.json",
        "APPEARANCE.JSON",
    ] {
        let path = directory.path().join(name);
        assert!(Workspace::open(&path).is_err());
        assert!(!path.exists());
    }
    let database = directory.path().join("active.txt");
    let workspace = Workspace::open(&database)?;
    let project = workspace.create_project("Preserve")?;
    let alias = directory.path().join("appearance.json");
    std::fs::hard_link(&database, &alias)?;
    assert!(Workspace::open(&database).is_err());
    assert_eq!(workspace.list_projects()?, vec![project]);
    Ok(())
}

#[test]
fn absent_future_sidecars_and_preference_paths_cannot_receive_a_backup()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("active.txt");
    let project = {
        let workspace = Workspace::open(&database)?;
        workspace.create_project("Retain committed data")?
    };
    let sidecar = |suffix: &str| {
        let mut path = database.as_os_str().to_owned();
        path.push(suffix);
        PathBuf::from(path)
    };
    for path in [
        sidecar("-wal"),
        sidecar("-shm"),
        sidecar("-journal"),
        directory.path().join("interface.json"),
        directory.path().join("appearance.json"),
        directory.path().join("editor.json"),
    ] {
        assert!(!path.exists());
        assert!(is_workspace_data_path(&path, &database)?);
        assert!(backup_workspace(&database, &path, &AtomicBool::new(false)).is_err());
        assert!(!path.exists());
    }
    assert_eq!(Workspace::open(&database)?.list_projects()?, vec![project]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn selected_database_symlink_reserves_real_database_future_sidecars()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let actual = directory.path().join("actual.sqlite");
    drop(Workspace::open(&actual)?);
    let selected = directory.path().join("selected.sqlite");
    std::os::unix::fs::symlink(&actual, &selected)?;
    for name in [
        "actual.sqlite-wal",
        "actual.sqlite-shm",
        "actual.sqlite-journal",
        "selected.sqlite-wal",
    ] {
        let output = directory.path().join(name);
        assert!(!output.exists());
        assert!(is_workspace_data_path(&output, &selected)?);
        assert!(backup_workspace(&selected, &output, &AtomicBool::new(false)).is_err());
        assert!(!output.exists());
    }
    Ok(())
}
