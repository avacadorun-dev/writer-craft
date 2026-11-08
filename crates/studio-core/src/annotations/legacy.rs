//! Explicit compatibility for the two original, inconsistent annotation projections.
use super::{AnchorStatus, AnnotationAnchor, Result, RichTree, StudioError, anchor_at};
use craft_editor_model::{
    BlockTextProjection, InlineAtomText, ScalarOffset, TextProjectionOptions, scalar_to_byte,
};
use serde_json::Value;

/// A known originating offset convention, never inferred from a quote search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyAnchorProjection {
    /// Original frontend: newline-separated paragraphs; inline atoms/hard breaks omitted.
    Frontend,
    /// Original Go backend: concatenated paragraphs; newline breaks and math source labels.
    Backend,
}
/// Exact conversion result retaining the entire original anchor object for transfer history.
#[derive(Clone, Debug)]
pub struct LegacyAnchorResolution {
    /// Original JSON, including unknown metadata and original field spellings.
    pub original: Value,
    /// Verified native anchor, absent for ambiguous or invalid known projections.
    pub native: Option<AnnotationAnchor>,
    /// Attached only for one unambiguous physical passage; otherwise detached.
    pub anchor_status: AnchorStatus,
    /// Known conventions that validated the original exact quote and both contexts.
    pub matching_projections: Vec<LegacyAnchorProjection>,
}
impl LegacyAnchorProjection {
    fn options(self) -> TextProjectionOptions<'static> {
        match self {
            Self::Frontend => TextProjectionOptions {
                block_separator: "\n",
                hard_break: "",
                inline_math: InlineAtomText::Omit,
                block_atom: "",
            },
            Self::Backend => TextProjectionOptions {
                block_separator: "",
                hard_break: "\n",
                inline_math: InlineAtomText::WithAffixes {
                    prefix: "[Формула: ",
                    suffix: "]",
                },
                block_atom: "",
            },
        }
    }
}
impl AnnotationAnchor {
    /// Convert only exact known legacy offset policies into current native coordinates.
    ///
    /// With no explicit origin, both known conventions are checked at the stored
    /// range with the stored quote and 32-scalar contexts. Multiple physical
    /// targets, an interior expanded math label or missing exact evidence detach.
    /// No substring search, nearest occurrence or old broken DOM locator is used.
    /// The caller must retain `original` in import history and set the note's
    /// lifecycle to needs-attention if `native` is absent; this pure adapter does
    /// not silently insert or modify durable annotations.
    pub fn adapt_legacy(
        tree: &RichTree,
        original: &Value,
        origin: Option<LegacyAnchorProjection>,
    ) -> Result<LegacyAnchorResolution> {
        let anchor: Self = serde_json::from_value(original.clone())?;
        if anchor.block_id.is_empty() || anchor.start >= anchor.end || anchor.exact_text.is_empty()
        {
            return Err(StudioError::Validation(
                "Invalid original annotation anchor".into(),
            ));
        }
        let policies = origin.map_or_else(
            || {
                vec![
                    LegacyAnchorProjection::Frontend,
                    LegacyAnchorProjection::Backend,
                ]
            },
            |origin| vec![origin],
        );
        let mut native = None;
        let mut ambiguous = false;
        let mut matching_projections = vec![];
        for policy in policies {
            let candidate = (|| -> Result<AnnotationAnchor> {
                let projection =
                    BlockTextProjection::with_options(tree, &anchor.block_id, policy.options())
                        .map_err(|_| StudioError::ExpectedTextMismatch)?;
                if !matches_original(projection.text(), &anchor) {
                    return Err(StudioError::ExpectedTextMismatch);
                }
                let a = projection
                    .point(anchor.start, true)
                    .map_err(|_| StudioError::ExpectedTextMismatch)?;
                let b = projection
                    .point(anchor.end, false)
                    .map_err(|_| StudioError::ExpectedTextMismatch)?;
                let canonical = BlockTextProjection::new(tree, &anchor.block_id)
                    .map_err(|_| StudioError::ExpectedTextMismatch)?;
                let start = canonical
                    .offset(&a)
                    .map_err(|_| StudioError::ExpectedTextMismatch)?;
                let end = canonical
                    .offset(&b)
                    .map_err(|_| StudioError::ExpectedTextMismatch)?;
                anchor_at(tree, &anchor.block_id, start, end)
            })();
            if let Ok(candidate) = candidate {
                matching_projections.push(policy);
                if native
                    .as_ref()
                    .is_some_and(|previous| previous != &candidate)
                {
                    ambiguous = true;
                } else if native.is_none() {
                    native = Some(candidate);
                }
            }
        }
        if ambiguous {
            native = None;
        }
        Ok(LegacyAnchorResolution {
            original: original.clone(),
            anchor_status: if native.is_some() {
                AnchorStatus::Attached
            } else {
                AnchorStatus::Detached
            },
            native,
            matching_projections,
        })
    }
}
fn matches_original(text: &str, anchor: &AnnotationAnchor) -> bool {
    let length = text.chars().count();
    if anchor.end.0 > length {
        return false;
    }
    let slice = |a: usize, b: usize| -> Option<&str> {
        let a = scalar_to_byte(text, ScalarOffset(a)).ok()?.0;
        let b = scalar_to_byte(text, ScalarOffset(b)).ok()?.0;
        text.get(a..b)
    };
    slice(anchor.start.0, anchor.end.0) == Some(anchor.exact_text.as_str())
        && slice(anchor.start.0.saturating_sub(32), anchor.start.0) == Some(anchor.prefix.as_str())
        && slice(anchor.end.0, anchor.end.0.saturating_add(32).min(length))
            == Some(anchor.suffix.as_str())
}
