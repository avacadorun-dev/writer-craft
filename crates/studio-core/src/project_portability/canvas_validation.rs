//! Native-independent JSON Canvas and type-schema preservation checks.
use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::json::invalid;
use super::validation::{array, number, string};
use super::{Result, StudioError};

type Resources<'a> = HashMap<&'a str, &'a Value>;
type TypedFields = HashMap<(String, String), String>;

pub(super) fn validate_project(
    manifest: &Value,
    documents: &Resources<'_>,
    media: &Resources<'_>,
    canvases: &Resources<'_>,
) -> Result<()> {
    let fields = validate_schema_history(manifest.get("canvasSchema").filter(|v| !v.is_null()))?;
    for canvas in canvases.values() {
        bounded_text(canvas, "title", 1, 160)?;
        if number(canvas, "currentRevision")? == 0 {
            return invalid("Canvas current revision must be positive");
        }
        bounded_number(canvas, "position", -1e12, 1e12, false)?;
    }
    let revisions = array(manifest, "canvasRevisions")?;
    if revisions.len() > 100_000 {
        return invalid("too many Canvas revisions");
    }
    let mut seen = HashSet::new();
    let mut ids = HashSet::new();
    let mut latest: HashMap<&str, u64> = HashMap::new();
    for revision in revisions {
        let id = string(revision, "canvasId")?;
        let ordinal = number(revision, "revision")?;
        if !canvases.contains_key(id)
            || ordinal == 0
            || number(revision, "baseRevision")? >= ordinal
            || !seen.insert((id, ordinal))
            || !ids.insert(string(revision, "id")?)
        {
            return invalid("invalid Canvas revision identity/base");
        }
        bounded_text(revision, "source", 1, 80)?;
        let schema = revision["content"]["x-writers-studio"]["schemaVersion"]
            .as_u64()
            .unwrap_or(1);
        if !(1..=2).contains(&schema) || number(revision, "schemaVersion")? != schema {
            return invalid("Canvas schema differs from its content");
        }
        validate_content(&revision["content"], documents, media, canvases, &fields)?;
        latest
            .entry(id)
            .and_modify(|n| *n = (*n).max(ordinal))
            .or_insert(ordinal);
    }
    for (id, canvas) in canvases {
        if latest.get(id).copied() != Some(number(canvas, "currentRevision")?) {
            return invalid("Canvas head does not match its history");
        }
    }
    for revision in revisions {
        let base = number(revision, "baseRevision")?;
        if base > 0 && !seen.contains(&(string(revision, "canvasId")?, base)) {
            return invalid("Canvas base revision is missing");
        }
    }
    Ok(())
}

pub(super) fn validate_content(
    content: &Value,
    documents: &Resources<'_>,
    media: &Resources<'_>,
    canvases: &Resources<'_>,
    fields: &TypedFields,
) -> Result<()> {
    if !content.is_object()
        || content
            .get("x-writers-studio")
            .is_some_and(|v| !v.is_null() && !v.is_object())
        || serde_json::to_vec(content)?.len() > 12 * 1024 * 1024
    {
        return invalid("Canvas snapshot is invalid or exceeds 12 MiB");
    }
    let nodes = array(content, "nodes")?;
    let edges = array(content, "edges")?;
    if nodes.len() > 5_000 || edges.len() > 10_000 {
        return invalid("Canvas graph exceeds source bounds");
    }
    let mut ids = HashSet::with_capacity(nodes.len());
    let mut strokes = HashSet::new();
    let mut total_points = 0;
    for node in nodes {
        let id = string(node, "id")?;
        if id.trim().is_empty() || id.len() > 200 || !ids.insert(id) {
            return invalid("Canvas node IDs must be bounded and unique");
        }
        for key in ["x", "y"] {
            bounded_number(node, key, -10_000_000.0, 10_000_000.0, true)?;
        }
        for key in ["width", "height"] {
            bounded_number(node, key, 40.0, 10_000.0, true)?;
        }
        optional_color(node, "color", false)?;
        let extension = &node["x-writers-studio"];
        match string(node, "type")? {
            "text" => bounded_text(node, "text", 0, 100_000)?,
            "file" => {
                bounded_text(node, "file", 1, 1024)?;
                if let Some(resource) = extension.get("resource").filter(|v| !v.is_null()) {
                    reference(
                        string(resource, "kind")?,
                        string(resource, "id")?,
                        documents,
                        media,
                        canvases,
                    )?;
                }
            }
            "link" => {
                let url = string(node, "url")?;
                if url.len() > 2048
                    || !(url.trim().starts_with("https://") || url.trim().starts_with("http://"))
                {
                    return invalid("invalid Canvas link URL");
                }
            }
            "group" => {
                if node.get("label").is_some_and(|v| !v.is_null()) {
                    bounded_text(node, "label", 0, 500)?;
                }
                if let Some(element) = extension.get("element").filter(|v| !v.is_null()) {
                    let points = validate_element(element, node)?;
                    total_points += points;
                    if points > 0 {
                        strokes.insert(id);
                    }
                    if total_points > 200_000 {
                        return invalid("Canvas total stroke points exceed source bound");
                    }
                }
            }
            _ => return invalid("unsupported Canvas node kind"),
        }
        if let Some(card) = extension.get("card").filter(|v| !v.is_null()) {
            if let Some(id) = card
                .get("promotedDocumentId")
                .filter(|v| !v.is_null())
                .and_then(Value::as_str)
            {
                if !documents.contains_key(id) {
                    return invalid("promoted Canvas document is missing");
                }
            }
            if let Some(type_id) = card["typeId"].as_str() {
                if let Some(values) = card["fields"].as_object() {
                    for (field, value) in values {
                        if let Some(kind) = fields.get(&(type_id.to_owned(), field.clone())) {
                            if value.is_null() || value.as_str() == Some("") {
                                continue;
                            }
                            let id = value.as_str().ok_or_else(|| {
                                StudioError::Validation(
                                    "Typed Canvas reference must be a string".into(),
                                )
                            })?;
                            reference(kind, id, documents, media, canvases)?;
                        }
                    }
                }
            }
        }
    }
    let mut edge_ids = HashSet::with_capacity(edges.len());
    for edge in edges {
        let id = string(edge, "id")?;
        if id.trim().is_empty() || id.len() > 200 || !edge_ids.insert(id) {
            return invalid("Canvas edge IDs must be bounded and unique");
        }
        for key in ["fromNode", "toNode"] {
            let target = string(edge, key)?;
            if !ids.contains(target) || strokes.contains(target) {
                return invalid("Canvas edge points to a missing node or stroke");
            }
        }
        for key in ["fromSide", "toSide"] {
            optional_choice(edge, key, &["", "top", "right", "bottom", "left"])?;
        }
        for key in ["fromEnd", "toEnd"] {
            optional_choice(edge, key, &["", "none", "arrow"])?;
        }
        optional_color(edge, "color", false)?;
        if edge.get("label").is_some_and(|v| !v.is_null()) {
            bounded_text(edge, "label", 0, 500)?;
        }
    }
    Ok(())
}
fn reference(
    kind: &str,
    id: &str,
    documents: &Resources<'_>,
    media: &Resources<'_>,
    canvases: &Resources<'_>,
) -> Result<()> {
    let found = match kind {
        "document" => documents.contains_key(id),
        "media" => media.contains_key(id),
        "canvas" => canvases.contains_key(id),
        _ => false,
    };
    if !found {
        return invalid("Canvas resource references a missing or unsupported project identity");
    }
    Ok(())
}
fn validate_element(element: &Value, node: &Value) -> Result<usize> {
    match string(element, "kind")? {
        "stroke" => {
            let points = array(element, "points")?;
            if !(2..=4096).contains(&points.len()) {
                return invalid("invalid Canvas stroke point count");
            }
            choice(element, "brush", &["pen", "highlighter"])?;
            bounded_number(element, "size", 1.0, 128.0, false)?;
            bounded_number(element, "opacity", 0.01, 1.0, false)?;
            color(element, "color", false)?;
            let width = node["width"]
                .as_f64()
                .ok_or_else(|| StudioError::Validation("Invalid Canvas width".into()))?;
            let height = node["height"]
                .as_f64()
                .ok_or_else(|| StudioError::Validation("Invalid Canvas height".into()))?;
            for point in points {
                let point = point
                    .as_array()
                    .ok_or_else(|| StudioError::Validation("Invalid Canvas stroke point".into()))?;
                if !(2..=3).contains(&point.len())
                    || !valid_number(&point[0], -128.0, width + 128.0, false)
                    || !valid_number(&point[1], -128.0, height + 128.0, false)
                    || (point.len() == 3 && !valid_number(&point[2], 0.0, 1.0, false))
                {
                    return invalid("invalid Canvas stroke point");
                }
            }
            Ok(points.len())
        }
        "shape" => {
            choice(
                element,
                "shape",
                &["rectangle", "ellipse", "diamond", "line", "arrow"],
            )?;
            bounded_number(element, "size", 1.0, 32.0, false)?;
            bounded_number(element, "opacity", 0.01, 1.0, false)?;
            if element.get("rotation").is_some() {
                bounded_number(element, "rotation", -360.0, 360.0, false)?;
            }
            color(element, "color", false)?;
            color(element, "fill", true)?;
            optional_choice(element, "dash", &["", "solid", "dashed", "dotted"])?;
            if element.get("label").is_some_and(|v| !v.is_null()) {
                bounded_text(element, "label", 0, 500)?;
            }
            Ok(0)
        }
        "frame" => {
            if element.get("order").is_some() && number(element, "order")? > 5000 {
                return invalid("invalid Canvas frame order");
            }
            if element.get("label").is_some_and(|v| !v.is_null()) {
                bounded_text(element, "label", 0, 500)?;
            }
            Ok(0)
        }
        _ => invalid("unsupported Canvas drawing element"),
    }
}
fn validate_schema_history(schema: Option<&Value>) -> Result<TypedFields> {
    let mut result = HashMap::new();
    let Some(schema) = schema else {
        return Ok(result);
    };
    let revisions = array(schema, "revisions")?;
    let current = number(schema, "currentRevision")?;
    if current == 0 || revisions.is_empty() || revisions.len() > 10_000 {
        return invalid("invalid Canvas type-schema history");
    }
    let mut seen = HashSet::new();
    let mut max_revision = 0;
    for revision in revisions {
        let ordinal = number(revision, "revision")?;
        if ordinal == 0
            || number(revision, "baseRevision")? != ordinal - 1
            || !seen.insert(ordinal)
            || number(&revision["schema"], "revision")? != ordinal
        {
            return invalid("invalid Canvas type-schema revision");
        }
        bounded_text(revision, "source", 1, 80)?;
        validate_type_schema(&revision["schema"], &mut result)?;
        max_revision = max_revision.max(ordinal);
    }
    if max_revision != current {
        return invalid("Canvas type-schema head is missing");
    }
    Ok(result)
}
fn validate_type_schema(schema: &Value, fields: &mut TypedFields) -> Result<()> {
    if serde_json::to_vec(schema)?.len() > 512 * 1024 {
        return invalid("Canvas type schema exceeds 512 KiB");
    }
    let cards = array(schema, "cardTypes")?;
    let relations = array(schema, "relationTypes")?;
    if cards.len() > 100 || relations.len() > 100 {
        return invalid("Canvas type-schema type count exceeds source bounds");
    }
    let mut card_ids = HashSet::new();
    let mut relation_ids = HashSet::new();
    for card in cards {
        let id = string(card, "id")?;
        if !schema_id(id) || !card_ids.insert(id) {
            return invalid("invalid Canvas card type identity");
        }
        bounded_text(card, "name", 1, 120)?;
        bounded_text(card, "icon", 1, 80)?;
        color(card, "color", false)?;
        choice(
            card,
            "category",
            &["general", "narrative", "game-design", "custom"],
        )?;
        if card
            .get("documentKind")
            .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
            && !schema_id(string(card, "documentKind")?)
        {
            return invalid("invalid Canvas card document kind");
        }
        let definitions = array(card, "fields")?;
        if definitions.len() > 50 {
            return invalid("Canvas card has too many fields");
        }
        let mut keys = HashSet::new();
        for field in definitions {
            let field_id = string(field, "id")?;
            if !schema_id(field_id) || !keys.insert(field_id) {
                return invalid("invalid Canvas field identity");
            }
            bounded_text(field, "name", 1, 120)?;
            choice(
                field,
                "type",
                &[
                    "short-text",
                    "markdown",
                    "number",
                    "checkbox",
                    "single-select",
                    "multi-select",
                    "date",
                    "duration",
                    "document",
                    "media",
                    "canvas",
                ],
            )?;
            let kind = string(field, "type")?;
            let options = array(field, "options")?;
            if options.len() > 100
                || (!options.is_empty() && !matches!(kind, "single-select" | "multi-select"))
            {
                return invalid("invalid Canvas select options");
            }
            let mut unique = HashSet::new();
            for option in options {
                let value = option
                    .as_str()
                    .ok_or_else(|| StudioError::Validation("Invalid Canvas field option".into()))?;
                if value.trim().is_empty() || value.chars().count() > 120 || !unique.insert(value) {
                    return invalid("invalid Canvas field option");
                }
            }
            if matches!(kind, "document" | "media" | "canvas") {
                fields.insert((id.to_owned(), field_id.to_owned()), kind.to_owned());
            }
        }
    }
    for relation in relations {
        let id = string(relation, "id")?;
        if !schema_id(id) || !relation_ids.insert(id) {
            return invalid("invalid Canvas relation identity");
        }
        bounded_text(relation, "name", 1, 120)?;
        color(relation, "color", false)?;
        for key in ["fromTypes", "toTypes"] {
            for card in array(relation, key)? {
                if !card.as_str().is_some_and(|id| card_ids.contains(id)) {
                    return invalid("Canvas relation references a missing card type");
                }
            }
        }
    }
    for card in cards {
        for relation in array(card, "allowedRelations")? {
            if !relation
                .as_str()
                .is_some_and(|id| relation_ids.contains(id))
            {
                return invalid("Canvas card references a missing relation type");
            }
        }
    }
    Ok(())
}
fn schema_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 80
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
}
fn bounded_text(value: &Value, key: &str, min: usize, max: usize) -> Result<()> {
    let text = string(value, key)?;
    let count = text.chars().count();
    if count < min || count > max || (min > 0 && text.trim().is_empty()) {
        return invalid(format!("invalid Canvas {key}"));
    }
    Ok(())
}
fn valid_number(value: &Value, min: f64, max: f64, integer: bool) -> bool {
    value
        .as_f64()
        .is_some_and(|n| n.is_finite() && n >= min && n <= max && (!integer || n.fract() == 0.0))
}
fn bounded_number(value: &Value, key: &str, min: f64, max: f64, integer: bool) -> Result<()> {
    if !valid_number(&value[key], min, max, integer) {
        return invalid(format!("invalid Canvas {key}"));
    }
    Ok(())
}
fn choice(value: &Value, key: &str, allowed: &[&str]) -> Result<()> {
    if !allowed.contains(&string(value, key)?) {
        return invalid(format!("unsupported Canvas {key}"));
    }
    Ok(())
}
fn optional_choice(value: &Value, key: &str, allowed: &[&str]) -> Result<()> {
    if value.get(key).is_some_and(|v| !v.is_null()) {
        choice(value, key, allowed)?;
    }
    Ok(())
}
fn valid_color(value: &str, transparent: bool) -> bool {
    (transparent && value == "transparent")
        || matches!(value, "1" | "2" | "3" | "4" | "5" | "6")
        || ((value.len() == 7 || value.len() == 9)
            && value.starts_with('#')
            && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
}
fn color(value: &Value, key: &str, transparent: bool) -> Result<()> {
    if !valid_color(string(value, key)?, transparent) {
        return invalid(format!("invalid Canvas {key} color"));
    }
    Ok(())
}
fn optional_color(value: &Value, key: &str, transparent: bool) -> Result<()> {
    if value
        .get(key)
        .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
    {
        color(value, key, transparent)?;
    }
    Ok(())
}
