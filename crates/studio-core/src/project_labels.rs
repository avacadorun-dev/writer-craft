//! Guarded project label definitions. Exact names remain the existing assignment identities.
use std::collections::{BTreeMap, HashSet};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Result, StudioError, Workspace, require_project};

/// Stable palette meaning; the desktop chooses its accessible theme presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLabelColor {
    /// Blue; the compatible default for existing labels without a saved color.
    Blue,
    /// Violet.
    Violet,
    /// Rose.
    Rose,
    /// Orange.
    Orange,
    /// Amber.
    Amber,
    /// Neutral gray.
    Gray,
}
impl ProjectLabelColor {
    /// The complete bounded author-selectable palette.
    pub const ALL: [Self; 6] = [
        Self::Blue,
        Self::Violet,
        Self::Rose,
        Self::Orange,
        Self::Amber,
        Self::Gray,
    ];
}

/// One exact case-sensitive project label, independent of whether passages use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLabel {
    /// Verbatim authored name, also used by existing annotation tag assignments.
    pub name: String,
    /// Shared semantic color of every current usage of this label.
    pub color: ProjectLabelColor,
}

/// Opaque optimistic snapshot of active and retired label definitions.
/// Unknown stored fields remain private so every mutation preserves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLabelCatalog {
    project_id: String,
    revision: i64,
    labels: Vec<ProjectLabel>,
    retired: Vec<ProjectLabel>,
    raw: Value,
}
impl ProjectLabelCatalog {
    /// Owning native project namespace.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    /// Catalog metadata revision, unrelated to document or annotation revisions.
    pub fn revision(&self) -> i64 {
        self.revision
    }
    /// Available labels, including unused definitions, ordered by exact name.
    pub fn labels(&self) -> &[ProjectLabel] {
        &self.labels
    }
    /// Retained names/colors; these names cannot be reused for a different label.
    /// Restoring deleted markup also restores definitions named by its retained tags.
    pub fn retired_labels(&self) -> &[ProjectLabel] {
        &self.retired
    }
}

fn invalid<T>(message: &str) -> Result<T> {
    Err(StudioError::Validation(message.into()))
}

fn validate_name(name: &str) -> Result<()> {
    crate::annotations::validate_annotation_tags(&[name.to_owned()])
}

fn decode(project: &str, raw: Value) -> Result<ProjectLabelCatalog> {
    if raw["schemaVersion"].as_u64() != Some(1) {
        return invalid(
            "This project label catalog version is not supported; preserve the original project",
        );
    }
    let revision = raw["revision"]
        .as_i64()
        .filter(|value| *value >= 0)
        .ok_or_else(|| StudioError::Validation("The project label revision is invalid".into()))?;
    let rows = raw["labels"].as_array().ok_or_else(|| {
        StudioError::Validation("The project label catalog needs label definitions".into())
    })?;
    let mut seen = HashSet::new();
    let mut labels = Vec::new();
    let mut retired = Vec::new();
    for row in rows {
        let name = row["name"]
            .as_str()
            .ok_or_else(|| StudioError::Validation("A project label name is missing".into()))?;
        validate_name(name)?;
        if !seen.insert(name) {
            return invalid("Project label names must be unique, including retired labels");
        }
        let color: ProjectLabelColor =
            serde_json::from_value(row["color"].clone()).map_err(|_| {
                StudioError::Validation(
                    "This project label color is not supported; preserve the original project"
                        .into(),
                )
            })?;
        let removed = match row.get("retired") {
            None => false,
            Some(value) => value.as_bool().ok_or_else(|| {
                StudioError::Validation("Project label retirement must be explicit".into())
            })?,
        };
        let label = ProjectLabel {
            name: name.into(),
            color,
        };
        if removed {
            retired.push(label);
        } else {
            labels.push(label);
        }
    }
    labels.sort_by(|a, b| a.name.cmp(&b.name));
    retired.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ProjectLabelCatalog {
        project_id: project.into(),
        revision,
        labels,
        retired,
        raw,
    })
}

fn current_names(connection: &Connection, project: &str) -> Result<BTreeMap<String, bool>> {
    let mut statement = connection.prepare("SELECT a.annotation_json,a.deleted FROM document_annotations a JOIN documents d ON d.id=a.document_id WHERE d.project_id=?1 ORDER BY a.id")?;
    let mut rows = statement.query([project])?;
    let mut names = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let raw: Value = serde_json::from_str(&row.get::<_, String>(0)?)?;
        let tags: Vec<String> = raw
            .get("tags")
            .map(|tags| serde_json::from_value(tags.clone()))
            .transpose()?
            .unwrap_or_default();
        crate::annotations::validate_annotation_tags(&tags)?;
        let active = row.get::<_, i64>(1)? == 0;
        for tag in tags {
            names
                .entry(tag)
                .and_modify(|used| *used |= active)
                .or_insert(active);
        }
    }
    Ok(names)
}

pub(crate) fn read_catalog(connection: &Connection, project: &str) -> Result<ProjectLabelCatalog> {
    require_project(connection, project)?;
    let stored: Option<(i64, String)> = connection
        .query_row(
            "SELECT revision,raw_json FROM project_label_catalog WHERE project_id=?1",
            [project],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let mut catalog = if let Some((revision, raw)) = stored {
        let catalog = decode(project, serde_json::from_str(&raw)?)?;
        if catalog.revision != revision {
            return invalid("Project label revision disagrees with its saved catalog");
        }
        catalog
    } else {
        decode(project, json!({"schemaVersion":1,"revision":0,"labels":[]}))?
    };
    let known: HashSet<_> = catalog
        .labels
        .iter()
        .chain(&catalog.retired)
        .map(|label| label.name.as_str())
        .collect();
    let additions: Vec<_> = current_names(connection, project)?
        .into_keys()
        .filter(|name| !known.contains(name.as_str()))
        .collect();
    if !additions.is_empty() {
        let rows = catalog.raw["labels"].as_array_mut().ok_or_else(|| {
            StudioError::CorruptData("Saved label definitions are not an array".into())
        })?;
        rows.extend(
            additions
                .into_iter()
                .map(|name| json!({"name":name,"color":"blue","retired":false})),
        );
        catalog = decode(project, catalog.raw)?;
    }
    Ok(catalog)
}

pub(crate) fn guard_catalog(connection: &Connection, expected: &ProjectLabelCatalog) -> Result<()> {
    if read_catalog(connection, &expected.project_id)? != *expected {
        return invalid("The project labels changed; review the label catalog again");
    }
    Ok(())
}

pub(crate) fn guard_assignments(
    connection: &Connection,
    project: &str,
    expected: &ProjectLabelCatalog,
    tags: &[String],
    restoring: bool,
) -> Result<()> {
    if project != expected.project_id {
        return invalid("These project labels belong to another project; reopen the label picker");
    }
    guard_catalog(connection, expected)?;
    if tags.iter().any(|name| {
        !expected.labels.iter().any(|label| &label.name == name)
            && !(restoring && expected.retired.iter().any(|label| &label.name == name))
    }) {
        return invalid("A selected label is unavailable; review the project label picker again");
    }
    Ok(())
}

pub(crate) fn commit_catalog(
    connection: &Connection,
    expected: &ProjectLabelCatalog,
    mut raw: Value,
) -> Result<ProjectLabelCatalog> {
    let unchanged = raw == expected.raw;
    if unchanged {
        let stored: Option<String> = connection
            .query_row(
                "SELECT raw_json FROM project_label_catalog WHERE project_id=?1",
                [&expected.project_id],
                |row| row.get(0),
            )
            .optional()?;
        let stored = stored
            .map(|value| serde_json::from_str::<Value>(&value))
            .transpose()?;
        if stored.as_ref() == Some(&raw)
            || (stored.is_none() && raw == json!({"schemaVersion":1,"revision":0,"labels":[]}))
        {
            return Ok(expected.clone());
        }
        // The first explicit mutation materializes legacy definitions before the last
        // assignment can disappear. It changes no catalog meaning/revision, so other
        // same-session picker drafts remain valid. Merely reading never materializes it.
    }
    if !unchanged {
        raw["revision"] = json!(
            expected
                .revision
                .checked_add(1)
                .ok_or_else(|| StudioError::CorruptData(
                    "Project label revision overflow".into()
                ))?
        );
    }
    let catalog = decode(&expected.project_id, raw)?;
    connection.execute("INSERT INTO project_label_catalog(project_id,revision,raw_json) VALUES(?1,?2,?3) ON CONFLICT(project_id) DO UPDATE SET revision=excluded.revision,raw_json=excluded.raw_json",
        params![catalog.project_id,catalog.revision,serde_json::to_string(&catalog.raw)?])?;
    Ok(catalog)
}

fn record_mut<'a>(raw: &'a mut Value, name: &str) -> Result<&'a mut Value> {
    raw["labels"]
        .as_array_mut()
        .and_then(|rows| {
            rows.iter_mut()
                .find(|row| row["name"].as_str() == Some(name))
        })
        .ok_or_else(|| {
            StudioError::Validation("This project label is unavailable; refresh the catalog".into())
        })
}

pub(crate) fn changed_definition(
    catalog: &ProjectLabelCatalog,
    name: &str,
    replacement: Option<&str>,
) -> Result<Value> {
    if !catalog.labels.iter().any(|label| label.name == name) {
        return invalid("This project label is not active; restore it before changing it");
    }
    let mut raw = catalog.raw.clone();
    if let Some(replacement) = replacement {
        validate_name(replacement)?;
        if replacement != name
            && catalog
                .labels
                .iter()
                .chain(&catalog.retired)
                .any(|label| label.name == replacement)
        {
            return invalid("That tag already exists; choose a distinct name");
        }
        record_mut(&mut raw, name)?["name"] = json!(replacement);
    } else {
        record_mut(&mut raw, name)?["retired"] = json!(true);
    }
    Ok(raw)
}

/// Keep the compatibility tag-writing APIs usable without losing unused definitions.
/// Only explicit note restoration may reactivate a retired assignment definition.
pub(crate) fn ensure_assignments(
    connection: &Connection,
    project: &str,
    tags: &[String],
    restoring: bool,
) -> Result<()> {
    let catalog = read_catalog(connection, project)?;
    let mut raw = catalog.raw.clone();
    for name in tags {
        if catalog.retired.iter().any(|label| &label.name == name) {
            if !restoring {
                return invalid(
                    "This label was removed; restore it in the project label catalog before assigning it",
                );
            }
            record_mut(&mut raw, name)?["retired"] = json!(false);
        } else if !catalog.labels.iter().any(|label| &label.name == name) {
            raw["labels"]
                .as_array_mut()
                .ok_or_else(|| {
                    StudioError::CorruptData("Saved label definitions are not an array".into())
                })?
                .push(json!({"name":name,"color":"blue","retired":false}));
        }
    }
    commit_catalog(connection, &catalog, raw)?;
    Ok(())
}

pub(crate) fn capture_catalog(connection: &Connection, project: &str) -> Result<Value> {
    Ok(read_catalog(connection, project)?.raw)
}

pub(crate) fn validate_catalog(raw: &Value) -> Result<()> {
    decode("", raw.clone()).map(|_| ())
}

pub(crate) fn restore_catalog(connection: &Connection, project: &str, raw: &Value) -> Result<()> {
    let catalog = decode(project, raw.clone())?;
    for (name, active) in current_names(connection, project)? {
        let present = catalog.labels.iter().any(|label| label.name == name);
        if !present && (active || !catalog.retired.iter().any(|label| label.name == name)) {
            return invalid(
                "The restored label catalog does not match its current passage assignments",
            );
        }
    }
    connection.execute(
        "INSERT INTO project_label_catalog(project_id,revision,raw_json) VALUES(?1,?2,?3)",
        params![project, catalog.revision, serde_json::to_string(raw)?],
    )?;
    Ok(())
}

impl Workspace {
    /// Read definitions, synthesizing Blue for legacy names without rewriting notes or history.
    pub fn project_labels(&self, project: &str) -> Result<ProjectLabelCatalog> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let catalog = read_catalog(&transaction, project)?;
        transaction.commit()?;
        Ok(catalog)
    }

    /// Create an unused label under the exact reviewed catalog. Retired names stay reserved.
    pub fn create_project_label(
        &self,
        expected: &ProjectLabelCatalog,
        name: &str,
        color: ProjectLabelColor,
    ) -> Result<ProjectLabelCatalog> {
        validate_name(name)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        guard_catalog(&transaction, expected)?;
        if expected.retired.iter().any(|label| label.name == name) {
            return invalid(
                "This label name is retired; restore the retained label instead of reusing its name",
            );
        }
        if expected.labels.iter().any(|label| label.name == name) {
            return invalid("That tag already exists; choose a distinct name");
        }
        let mut raw = expected.raw.clone();
        raw["labels"]
            .as_array_mut()
            .ok_or_else(|| {
                StudioError::CorruptData("Saved label definitions are not an array".into())
            })?
            .push(json!({"name":name,"color":color,"retired":false}));
        let catalog = commit_catalog(&transaction, expected, raw)?;
        transaction.commit()?;
        Ok(catalog)
    }

    /// Change presentation metadata only; annotation revisions, anchors and histories are untouched.
    pub fn set_project_label_color(
        &self,
        expected: &ProjectLabelCatalog,
        name: &str,
        color: ProjectLabelColor,
    ) -> Result<ProjectLabelCatalog> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        guard_catalog(&transaction, expected)?;
        let mut raw = changed_definition(expected, name, Some(name))?;
        record_mut(&mut raw, name)?["color"] = serde_json::to_value(color)?;
        let catalog = commit_catalog(&transaction, expected, raw)?;
        transaction.commit()?;
        Ok(catalog)
    }

    /// Reactivate the same retained label and color; this does not reassign any passage.
    pub fn restore_project_label(
        &self,
        expected: &ProjectLabelCatalog,
        name: &str,
    ) -> Result<ProjectLabelCatalog> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        guard_catalog(&transaction, expected)?;
        if !expected.retired.iter().any(|label| label.name == name) {
            return invalid("This project label is not retired; refresh the catalog");
        }
        let mut raw = expected.raw.clone();
        record_mut(&mut raw, name)?["retired"] = json!(false);
        let catalog = commit_catalog(&transaction, expected, raw)?;
        transaction.commit()?;
        Ok(catalog)
    }
}
