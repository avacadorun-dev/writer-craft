use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use studio_core::{ImageTransfer, Workspace};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn png(color: u8) -> Result<Vec<u8>, image::ImageError> {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 3, Rgb([color, 80, 120])));
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, ImageFormat::Png)?;
    Ok(output.into_inner())
}
fn export(
    workspace: &Workspace,
    project: &str,
) -> Result<ImageTransfer, Box<dyn std::error::Error>> {
    let asset = workspace.import_image(project, "cover.png", Some("image/png"), &png(40)?)?;
    workspace
        .project_images(project)?
        .export_images(&[asset.id], 1_000_000)?
        .pop()
        .ok_or_else(|| "missing transfer".into())
}
#[test]
fn fixed_project_export_only_reads_selected_ids_and_rechecks_byte_budget() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Source")?;
    let image = export(&workspace, &project.id)?;
    let other = workspace.create_project("Other")?;
    let owned = workspace.project_images(&project.id)?;
    assert!(
        owned
            .export_images(
                std::slice::from_ref(&image.source_id),
                image.bytes.len() - 1
            )
            .is_err()
    );
    assert!(
        workspace
            .project_images(&other.id)?
            .export_images(std::slice::from_ref(&image.source_id), 1_000_000)
            .is_err()
    );
    assert!(owned.export_images(&[], 1)?.is_empty());
    assert_eq!(
        owned.export_images(
            &[image.source_id.clone(), image.source_id.clone()],
            1_000_000
        )?,
        vec![image]
    );
    Ok(())
}
#[test]
fn same_project_reuses_identical_owned_image_but_cross_project_assigns_new_identity() -> TestResult
{
    let workspace = Workspace::memory()?;
    let source = workspace.create_project("Source")?;
    let image = export(&workspace, &source.id)?;
    let same = workspace
        .project_images(&source.id)?
        .import_images(std::slice::from_ref(&image), 1_000_000)?;
    assert_eq!(same[0].asset.id, image.source_id);
    let target = workspace.create_project("Target")?;
    let imported = workspace
        .project_images(&target.id)?
        .import_images(std::slice::from_ref(&image), 1_000_000)?;
    assert_ne!(imported[0].asset.id, image.source_id);
    assert_eq!(imported[0].source_id, image.source_id);
    assert_eq!(
        workspace.image_asset_bytes(&target.id, &imported[0].asset.id)?,
        image.bytes
    );
    assert!(workspace.image_asset(&target.id, &image.source_id).is_err());
    Ok(())
}
#[test]
fn every_transfer_descriptor_is_validated_before_any_import() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("images.sqlite");
    let workspace = Workspace::open(&path)?;
    let source = workspace.create_project("Source")?;
    let image = export(&workspace, &source.id)?;
    let destination = workspace.create_project("Destination")?;
    let images = workspace.project_images(&destination.id)?;
    let connection = rusqlite::Connection::open(path)?;
    for field in 0..8 {
        let mut bad = image.clone();
        bad.source_id = uuid::Uuid::new_v4().to_string();
        match field {
            0 => bad.source_id = " ".into(),
            1 => bad.byte_size += 1,
            2 => bad.sha256 = "0".repeat(64),
            3 => bad.media_type = "image/jpeg".into(),
            4 => bad.width += 1,
            5 => bad.height += 1,
            6 => bad.bytes.clear(),
            _ => bad.bytes[0] = 0,
        }
        assert!(
            images
                .import_images(&[image.clone(), bad], 1_000_000)
                .is_err()
        );
        let count: i64 = connection.query_row(
            "SELECT count(*) FROM project_assets WHERE project_id=?1",
            [&destination.id],
            |row| row.get(0),
        )?;
        assert_eq!(count, 0);
    }
    assert!(
        images
            .import_images(&[image.clone(), image], 1_000_000)
            .is_err()
    );
    Ok(())
}
#[test]
fn sql_failure_rolls_back_the_entire_bulk_import() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("images.sqlite");
    let workspace = Workspace::open(&path)?;
    let source = workspace.create_project("Source")?;
    let first = export(&workspace, &source.id)?;
    let mut second = first.clone();
    second.source_id = uuid::Uuid::new_v4().to_string();
    second.original_name = "reject.png".into();
    let destination = workspace.create_project("Destination")?;
    let connection = rusqlite::Connection::open(path)?;
    connection.execute_batch("CREATE TRIGGER reject_transfer BEFORE INSERT ON project_assets WHEN NEW.original_name='reject.png' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")?;
    assert!(
        workspace
            .project_images(&destination.id)?
            .import_images(&[first, second], 1_000_000)
            .is_err()
    );
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM project_assets WHERE project_id=?1",
        [&destination.id],
        |row| row.get(0),
    )?;
    assert_eq!(count, 0);
    Ok(())
}
#[test]
fn fixed_project_handle_can_be_sent_to_a_worker_without_cloning_workspace_state() -> TestResult {
    let workspace = Workspace::memory()?;
    let source = workspace.create_project("Source")?;
    let image = export(&workspace, &source.id)?;
    let images = workspace.project_images(&source.id)?;
    let bytes = std::thread::spawn(move || images.export_images(&[image.source_id], 1_000_000))
        .join()
        .map_err(|_| "worker panicked")??;
    assert_eq!(bytes.len(), 1);
    Ok(())
}
