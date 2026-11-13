//! Captured native drop intent is revalidated inside the persisted move transaction.
use studio_core::{ProjectTreeNode, StudioError, Workspace};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn current(workspace: &Workspace, project: &str, id: &str) -> studio_core::Result<ProjectTreeNode> {
    workspace
        .initialize_project_tree(project)?
        .into_iter()
        .find(|node| node.logical_id == id)
        .ok_or_else(|| StudioError::NotFound {
            kind: "test tree node",
            id: id.into(),
        })
}

#[test]
fn changed_hover_target_refuses_drop_without_rewriting_manuscript_or_structure() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic guarded drop")?;
    let manuscript = workspace.create_document(&project.id, "Scene", "One promise. 😀 Another.")?;
    let manuscript = workspace.save_document(
        &manuscript.id,
        manuscript.revision,
        "One promise. 😀 A revised promise.",
    )?;
    let source = workspace.initialize_project_tree(&project.id)?.remove(0);
    let structure = workspace.project_structure(&project.id)?;
    let folder = workspace.create_project_tree_node(&project.id, 1, None, "folder", "Chapter")?;
    let hovered = workspace.create_project_tree_node(
        &project.id,
        1,
        Some(&folder.logical_id),
        "document",
        "Earlier scene",
    )?;
    let next = workspace.create_project_tree_node(
        &project.id,
        1,
        Some(&folder.logical_id),
        "document",
        "Later scene",
    )?;
    let history = workspace.revisions(&manuscript.id)?;

    // Another caller changes the row shown as the "after" target after UI preflight.
    // The destination parent and "before" sibling themselves are still unchanged.
    workspace.rename_project_tree_node(&project.id, &hovered, "A revised scene title")?;
    let before = workspace.initialize_project_tree(&project.id)?;
    let result = workspace.move_project_tree_node_at_snapshot(
        &structure,
        &source,
        &[folder.clone(), hovered, next.clone()],
        Some(&folder.logical_id),
        Some(&next.logical_id),
    );
    assert!(matches!(
        result,
        Err(StudioError::ProjectNodeChanged { .. })
    ));
    assert_eq!(workspace.initialize_project_tree(&project.id)?, before);
    assert_eq!(workspace.document(&manuscript.id)?, manuscript);
    assert_eq!(workspace.revisions(&manuscript.id)?, history);

    let moved = workspace.move_project_tree_node_at_snapshot(
        &structure,
        &source,
        &[folder.clone(), next.clone()],
        Some(&folder.logical_id),
        Some(&next.logical_id),
    )?;
    assert_eq!(moved.logical_id, source.logical_id);
    assert_eq!(moved.metadata["parentId"], folder.logical_id);
    let next = current(&workspace, &project.id, &next.logical_id)?;
    assert!(moved.metadata["position"].as_f64() < next.metadata["position"].as_f64());
    assert_eq!(workspace.document(&manuscript.id)?, manuscript);
    assert_eq!(workspace.revisions(&manuscript.id)?, history);
    Ok(())
}

#[test]
fn stale_schema_or_parent_and_missing_destination_snapshot_cannot_authorize_a_drop() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic drop scope")?;
    let source = workspace.create_project_tree_node(&project.id, 1, None, "document", "Scene")?;
    let folder = workspace.create_project_tree_node(&project.id, 1, None, "folder", "Chapter")?;
    let structure = workspace.project_structure(&project.id)?;
    let before = workspace.initialize_project_tree(&project.id)?;
    let mut mismatched = structure.clone();
    mismatched.schema["rootLabel"] = "A different captured schema".into();
    assert!(matches!(
        workspace.move_project_tree_node_at_snapshot(
            &mismatched,
            &source,
            std::slice::from_ref(&folder),
            Some(&folder.logical_id),
            None,
        ),
        Err(StudioError::ProjectStructureConflict { .. })
    ));
    assert!(
        workspace
            .move_project_tree_node_at_snapshot(
                &structure,
                &source,
                &[],
                Some(&folder.logical_id),
                None,
            )
            .is_err()
    );
    assert_eq!(workspace.initialize_project_tree(&project.id)?, before);

    workspace.rename_project_tree_node(&project.id, &folder, "Changed destination")?;
    let before = workspace.initialize_project_tree(&project.id)?;
    assert!(matches!(
        workspace.move_project_tree_node_at_snapshot(
            &structure,
            &source,
            std::slice::from_ref(&folder),
            Some(&folder.logical_id),
            None,
        ),
        Err(StudioError::ProjectNodeChanged { .. })
    ));
    assert_eq!(workspace.initialize_project_tree(&project.id)?, before);
    Ok(())
}

#[test]
fn dropping_back_into_the_existing_slot_is_an_exact_no_op() -> TestResult {
    let workspace = Workspace::memory()?;
    let project = workspace.create_project("Synthetic unchanged placement")?;
    let first = workspace.create_project_tree_node(&project.id, 1, None, "document", "First")?;
    let second = workspace.create_project_tree_node(&project.id, 1, None, "document", "Second")?;
    let structure = workspace.project_structure(&project.id)?;
    let before = workspace.initialize_project_tree(&project.id)?;
    assert_eq!(
        workspace.move_project_tree_node_at_snapshot(
            &structure,
            &first,
            std::slice::from_ref(&second),
            None,
            Some(&second.logical_id),
        )?,
        first
    );
    assert_eq!(
        workspace.move_project_tree_node_at_snapshot(&structure, &second, &[], None, None,)?,
        second
    );
    assert_eq!(workspace.initialize_project_tree(&project.id)?, before);
    Ok(())
}
