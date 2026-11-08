//! Source-backed authoring controls; every action commits through the shared rich session.
use super::*;
use craft_editor_egui::{ContentSlice, ListKind, SelectionBookmark};
use craft_editor_model::MarkMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Mermaid,
    Math,
    InlineMath,
    Code,
    Image,
    Link,
}
impl Kind {
    fn title(self) -> &'static str {
        match self {
            Self::Mermaid => "Mermaid diagram",
            Self::Math => "Block formula",
            Self::InlineMath => "Inline formula",
            Self::Code => "Code block",
            Self::Image => "Image",
            Self::Link => "Link",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::Mermaid => Icon::Diagram,
            Self::Math | Self::InlineMath => Icon::Formula,
            Self::Code => Icon::Code,
            Self::Image => Icon::Image,
            Self::Link => Icon::Link,
        }
    }
    fn node_type(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaidBlock",
            Self::Math => "mathBlock",
            Self::InlineMath => "inlineMath",
            Self::Code => "codeBlock",
            Self::Image => "imageBlock",
            Self::Link => "",
        }
    }
    fn from_node(node: &Node) -> Option<Self> {
        match node.node_type.as_str() {
            "mermaidBlock" => Some(Self::Mermaid),
            "mathBlock" => Some(Self::Math),
            "inlineMath" => Some(Self::InlineMath),
            "codeBlock" => Some(Self::Code),
            "imageBlock" => Some(Self::Image),
            _ => None,
        }
    }
}

pub(super) struct BlockDialog {
    kind: Kind,
    revision: Revision,
    bookmark: SelectionBookmark,
    target: Option<NodeRef>,
    source: String,
    caption: String,
    language: String,
    alt: String,
    image_path: Option<PathBuf>,
    focus: bool,
    error: Option<String>,
}
impl BlockDialog {
    fn new(session: &EditorSession, kind: Kind, target: Option<NodeRef>) -> AppResult<Self> {
        let bookmark = session.capture_selection_bookmark()?;
        let node = target.as_ref().map(|target| target.expected.as_ref());
        let source = match kind {
            Kind::Mermaid => node
                .and_then(|node| node.string_attr("code"))
                .unwrap_or("flowchart TD\n  Start[Start] --> Result[Result]")
                .into(),
            Kind::Math | Kind::InlineMath => node
                .and_then(|node| node.string_attr("latex"))
                .unwrap_or("x^2 + y^2")
                .into(),
            Kind::Code => node.map_or_else(String::new, |node| {
                node.children()
                    .iter()
                    .filter_map(|child| child.text.as_deref())
                    .collect()
            }),
            Kind::Image => node
                .and_then(|node| node.string_attr("src"))
                .unwrap_or_default()
                .into(),
            Kind::Link => link_at_selection(session).unwrap_or_default(),
        };
        Ok(Self {
            kind,
            revision: session.revision(),
            bookmark,
            caption: node
                .and_then(|node| {
                    node.string_attr(if matches!(kind, Kind::Math | Kind::InlineMath) {
                        "description"
                    } else {
                        "caption"
                    })
                })
                .unwrap_or_default()
                .into(),
            language: node
                .and_then(|node| node.string_attr("language"))
                .unwrap_or_default()
                .into(),
            alt: node
                .and_then(|node| node.string_attr("alt"))
                .unwrap_or_default()
                .into(),
            target,
            source,
            image_path: None,
            focus: true,
            error: None,
        })
    }
    fn restore_target(&self, session: &mut EditorSession) -> AppResult<()> {
        if session.revision() != self.revision {
            return Err("stale_block_dialog".into());
        }
        session.restore_selection_bookmark(&self.bookmark)?;
        Ok(())
    }
    fn can_apply(&self) -> bool {
        match self.kind {
            Kind::Image => self.target.is_some() || self.image_path.is_some(),
            Kind::Code | Kind::Link => true,
            _ => !self.source.trim().is_empty(),
        }
    }
    fn apply(
        &self,
        session: &mut EditorSession,
        workspace: &Workspace,
        project_id: &str,
    ) -> AppResult<()> {
        if session.revision() != self.revision {
            return Err("stale_block_dialog".into());
        }
        self.restore_target(session)?;
        if self.kind == Kind::Link {
            let mut mark = Mark::new("link");
            let mode = if self.source.trim().is_empty() {
                MarkMode::Remove
            } else {
                let href = self.source.trim();
                if !craft_editor_model::is_allowed_link_uri(href) {
                    return Err("invalid_link_protocol".into());
                }
                mark.attrs = Some(Map::from_iter([("href".into(), json!(href))]));
                MarkMode::Add
            };
            apply_link(
                session,
                mark,
                if mode == MarkMode::Remove {
                    mode
                } else {
                    MarkMode::Patch
                },
            )?;
            return Ok(());
        }
        if self.kind == Kind::Code
            && let Some(target) = &self.target
        {
            return self.apply_code(session, target);
        }
        let node = self.build_node(workspace, project_id)?;
        commit_node(
            session,
            self.target.as_ref(),
            node,
            self.kind == Kind::InlineMath,
        )?;
        Ok(())
    }
    /// Use the same granular text commands as native typing, including their
    /// sealed position maps. Rebuilding the whole code node would detach notes
    /// and discard untouched imported leaf metadata.
    fn apply_code(&self, session: &mut EditorSession, target: &NodeRef) -> AppResult<()> {
        use craft_editor_model::{Command, Editor, TextRange, Transaction};
        if session.document().node(&target.path)? != target.expected.as_ref() {
            return Err(craft_editor_model::Error::NodeReferenceConflict.into());
        }
        let id = target.expected.block_id().ok_or("missing_code_identity")?;
        let previous = target.expected.inline_text();
        let old_len = previous.chars().count();
        let new_len = self.source.chars().count();
        let prefix = previous
            .chars()
            .zip(self.source.chars())
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = previous
            .chars()
            .rev()
            .take(old_len - prefix)
            .zip(self.source.chars().rev().take(new_len - prefix))
            .take_while(|(a, b)| a == b)
            .count();
        let end = old_len - suffix;
        let inserted = new_len - prefix - suffix;
        let mut commands = Vec::new();
        if previous != self.source {
            commands.push(Command::ReplaceText {
                range: TextRange {
                    block_id: id.into(),
                    start: ScalarOffset(prefix),
                    end: ScalarOffset(end),
                },
                expected: previous.chars().skip(prefix).take(end - prefix).collect(),
                replacement: self.source.chars().skip(prefix).take(inserted).collect(),
            });
        }
        if target.expected.string_attr("language").unwrap_or_default() != self.language {
            commands.push(Command::SetBlockAttrs {
                block_id: id.into(),
                attrs: Map::from_iter([(
                    "language".into(),
                    if self.language.is_empty() {
                        serde_json::Value::Null
                    } else {
                        json!(self.language)
                    },
                )]),
            });
        }
        if commands.is_empty() {
            return Ok(());
        }
        if let Some(selection) = session.selection() {
            let map = |point: &DocPoint| {
                let mut mapped = point.clone();
                if point.block_id == id {
                    mapped.offset = ScalarOffset(if point.offset.0 < prefix {
                        point.offset.0
                    } else if point.offset.0 > end {
                        prefix + inserted + (point.offset.0 - end)
                    } else {
                        prefix + inserted
                    });
                }
                mapped
            };
            let selection = Selection {
                anchor: map(&selection.anchor),
                head: map(&selection.head),
            };
            session.apply_commands_with_selection(commands, selection)?;
        } else if let Some(cells) = session.cell_selection().cloned() {
            session.apply_commands_with_cell_selection(commands, cells)?;
        } else {
            let mut candidate = Editor::new(session.document().clone(), session.revision());
            candidate.apply(Transaction {
                base_revision: candidate.revision(),
                commands: commands.clone(),
            })?;
            let selected = candidate.document().node_ref(&target.path)?;
            session.apply_commands_with_node_selection(commands, selected)?;
        }
        session.request_scroll_to_caret();
        Ok(())
    }

    fn build_node(&self, workspace: &Workspace, project_id: &str) -> AppResult<Node> {
        let mut node = self.target.as_ref().map_or_else(
            || Node::block(self.kind.node_type(), Map::new(), vec![]),
            |target| target.expected.as_ref().clone(),
        );
        if self.kind == Kind::Image
            && let Some(path) = &self.image_path
        {
            let asset = workspace.import_image_file(project_id, path)?;
            let id = node
                .block_id()
                .map(str::to_owned)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let replacement =
                workspace.image_node(project_id, &asset.id, &id, &self.alt, &self.caption)?;
            node.attrs
                .get_or_insert_default()
                .extend(replacement.attrs.unwrap_or_default());
        }
        let attrs = node.attrs.get_or_insert_default();
        if self.kind != Kind::InlineMath {
            attrs
                .entry("blockId")
                .or_insert_with(|| json!(uuid::Uuid::new_v4().to_string()));
        }
        match self.kind {
            Kind::Mermaid => {
                attrs.insert("code".into(), json!(self.source));
                attrs.insert("caption".into(), json!(self.caption));
            }
            Kind::Math | Kind::InlineMath => {
                attrs.insert("latex".into(), json!(self.source));
                attrs.insert("description".into(), json!(self.caption));
            }
            Kind::Code => {
                attrs.insert(
                    "language".into(),
                    if self.language.is_empty() {
                        serde_json::Value::Null
                    } else {
                        json!(self.language)
                    },
                );
                let previous: String = node
                    .children()
                    .iter()
                    .filter_map(|child| child.text.as_deref())
                    .collect();
                if previous != self.source {
                    // Preserve an existing leaf's opaque metadata on a direct source edit.
                    // Multiple differently attributed leaves need the normal granular editor
                    // transaction rather than an ambiguous full-source rewrite.
                    if node.children().len() > 1 {
                        return Err("ambiguous_code_source_rewrite".into());
                    }
                    node.content = Some(if self.source.is_empty() {
                        if node.children().first().is_some_and(|leaf| {
                            !leaf.extra.is_empty() || leaf.attrs.is_some() || leaf.marks.is_some()
                        }) {
                            return Err("opaque_code_source_requires_granular_edit".into());
                        }
                        vec![]
                    } else {
                        let mut leaf = node
                            .children()
                            .first()
                            .cloned()
                            .unwrap_or_else(|| Node::text(&self.source));
                        leaf.text = Some(self.source.clone());
                        vec![leaf]
                    });
                }
            }
            Kind::Image => {
                attrs.insert("alt".into(), json!(self.alt));
                attrs.insert("caption".into(), json!(self.caption));
            }
            Kind::Link => {}
        }
        Ok(node)
    }
}

fn commit_node(
    session: &mut EditorSession,
    target: Option<&NodeRef>,
    node: Node,
    inline: bool,
) -> AppResult<()> {
    if let Some(target) = target {
        let command = session.document().replace_node(target, node)?;
        let mut candidate =
            craft_editor_model::Editor::new(session.document().clone(), session.revision());
        candidate.apply(craft_editor_model::Transaction {
            base_revision: candidate.revision(),
            commands: vec![command.clone()],
        })?;
        let selected = candidate.document().node_ref(&target.path)?;
        session.apply_commands_with_node_selection(vec![command], selected)?;
    } else if inline {
        session.insert_content(ContentSlice::Inline(vec![node]))?;
    } else if node.node_type == "codeBlock" {
        let id = node.block_id().ok_or("missing_code_identity")?.to_owned();
        let point = DocPoint {
            block_id: id,
            offset: ScalarOffset(0),
        };
        session.insert_content_with_selection(
            ContentSlice::Blocks(vec![node]),
            Selection {
                anchor: point.clone(),
                head: point,
            },
        )?;
    } else {
        let id = node.block_id().ok_or("missing_atom_identity")?.to_owned();
        session.insert_content_selecting_node(ContentSlice::Blocks(vec![node]), &id)?;
    }
    session.request_scroll_to_caret();
    Ok(())
}

fn editable_target(session: &EditorSession) -> Option<NodeRef> {
    if let Some(node) = session.node_selection() {
        return Kind::from_node(&node.expected).map(|_| node.clone());
    }
    let id = &session.selection()?.head.block_id;
    let path = find_path(session.document().root(), id, NodePath::root())?;
    let node = session.document().node_ref(&path).ok()?;
    (node.expected.node_type == "codeBlock").then_some(node)
}
fn apply_link(session: &mut EditorSession, mark: Mark, mode: MarkMode) -> AppResult<()> {
    session.format_mark_range(mark, mode)?;
    Ok(())
}

fn link_at_selection(session: &EditorSession) -> Option<String> {
    // Reuse the engine's original right-marked/left-marked boundary semantics
    // without changing the author's actual selection before Apply.
    let mut probe = EditorSession::new(session.document().clone(), session.revision());
    probe.set_selection(session.selection()?.clone()).ok()?;
    probe.extend_mark_range("link").ok()?;
    let selection = probe.selection()?;
    let block = probe.document().block(&selection.anchor.block_id).ok()?;
    let mut offset = 0;
    for child in block.children() {
        let length = child.text.as_ref().map_or(1, |text| text.chars().count());
        if selection.anchor.offset.0 >= offset && selection.anchor.offset.0 < offset + length {
            return child
                .marks
                .as_ref()?
                .iter()
                .find(|mark| mark.mark_type == "link")?
                .attrs
                .as_ref()?
                .get("href")?
                .as_str()
                .map(str::to_owned);
        }
        offset += length;
    }
    None
}

fn open_dialog(rich: &mut RichState, kind: Kind, target: Option<NodeRef>) -> AppResult<()> {
    rich.dialog = Some(BlockDialog::new(&rich.session, kind, target)?);
    Ok(())
}

fn capture(result: AppResult<()>, error: &mut Option<Box<dyn std::error::Error + Send + Sync>>) {
    if let Err(value) = result
        && error.is_none()
    {
        *error = Some(value);
    }
}

#[derive(Clone, Copy)]
enum Format {
    Mark(&'static str),
    Quote,
    Bullet,
    Ordered,
    Indent,
    Outdent,
}
fn format(session: &mut EditorSession, action: Format) -> AppResult<()> {
    match action {
        Format::Mark(mark) => session.toggle_mark(Mark::new(mark))?,
        Format::Quote => {
            session.toggle_blockquote()?;
        }
        Format::Bullet => {
            session.toggle_list(ListKind::Bullet)?;
        }
        Format::Ordered => {
            session.toggle_list(ListKind::Ordered)?;
        }
        Format::Indent => {
            session.indent_list()?;
        }
        Format::Outdent => {
            session.outdent_list()?;
        }
    }
    Ok(())
}
fn active(session: &EditorSession, action: Format) -> MarkState {
    match action {
        Format::Mark(mark) => session.mark_state(mark),
        Format::Quote => session.blockquote_state(),
        Format::Bullet => session.list_state(ListKind::Bullet),
        Format::Ordered => session.list_state(ListKind::Ordered),
        _ => return MarkState::Off,
    }
    .unwrap_or(MarkState::Off)
}
fn mark_button(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    action: Format,
    msgid: &str,
    icon: Icon,
    l: Language,
    t: Tokens,
) -> AppResult<()> {
    let state = active(session, action);
    let label = l.text(msgid);
    let tooltip = if state == MarkState::Mixed {
        format!("{} · {}", label, l.text("Mixed formatting"))
    } else {
        label.into()
    };
    let response = ui
        .add_enabled(
            session.selection().is_some()
                || session.cell_selection().is_some()
                || (matches!(action, Format::Mark(_))
                    && session
                        .node_selection()
                        .is_some_and(|node| node.expected.node_type == "inlineMath")),
            egui::Button::new("")
                .selected(state == MarkState::On)
                .frame_when_inactive(state != MarkState::Off)
                .stroke(Stroke::new(
                    if state == MarkState::Mixed { 1.0 } else { 0.0 },
                    t.accent,
                ))
                .min_size(egui::vec2(32.0, 32.0)),
        )
        .on_hover_text(&tooltip);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled() && session.selection().is_some(),
            state == MarkState::On,
            &tooltip,
        )
    });
    if ui.is_rect_visible(response.rect) {
        let color = if !ui.is_enabled() || session.selection().is_none() {
            ui.visuals().disable(t.text)
        } else if state != MarkState::Off {
            t.accent
        } else {
            t.text
        };
        icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(20.0)),
            icon,
            color,
        );
    }
    if response.clicked() {
        format(session, action)?;
        session.request_canvas_focus(ui.ctx());
    }
    Ok(())
}
fn format_icon(action: Format) -> Icon {
    match action {
        Format::Mark("bold") => Icon::Bold,
        Format::Mark("italic") => Icon::Italic,
        Format::Mark("underline") => Icon::Underline,
        Format::Mark("strike") => Icon::Strike,
        Format::Mark("code") => Icon::Code,
        Format::Quote => Icon::Quote,
        Format::Bullet => Icon::BulletList,
        Format::Ordered => Icon::OrderedList,
        Format::Indent => Icon::Indent,
        Format::Outdent => Icon::Outdent,
        Format::Mark(_) => Icon::TextCursor,
    }
}

fn block_style_label(session: &EditorSession, language: Language) -> AppResult<&'static str> {
    use craft_editor_egui::TextBlockState;
    Ok(language.text(match session.text_block_state()? {
        TextBlockState::Heading { level: 1 } => "Heading 1",
        TextBlockState::Heading { level: 2 } => "Heading 2",
        TextBlockState::Heading { level: 3 } => "Heading 3",
        TextBlockState::Heading { level: 4 } => "Heading 4",
        TextBlockState::Heading { level: 5 } => "Heading 5",
        TextBlockState::Heading { level: 6 } => "Heading 6",
        TextBlockState::Heading { .. } => "Heading",
        TextBlockState::Code => "Code block",
        TextBlockState::Mixed => "Mixed formatting",
        TextBlockState::Paragraph | TextBlockState::Unavailable => "Paragraph",
    }))
}

pub(super) fn toolbar(
    ui: &mut egui::Ui,
    rich: &mut RichState,
    l: Language,
    t: Tokens,
    mode: crate::editor_preferences::ToolbarMode,
) -> AppResult<()> {
    let mut error = None;
    let label = block_style_label(&rich.session, l)?;
    // Expanded remains bounded by physical space; all hidden actions stay in More.
    let expanded =
        mode != crate::editor_preferences::ToolbarMode::Compact && ui.available_width() >= 790.0;
    let minimal = ui.available_width() < 560.0;
    ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
    ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
    ui.spacing_mut().item_spacing.x = 3.0;
    ui.add_enabled_ui(
        !rich.read_only && !rich.session.is_composing() && rich.dialog.is_none(),
        |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_enabled_ui(
                    rich.session.selection().is_some() || rich.session.cell_selection().is_some(),
                    |ui| {
                        let width = if minimal { 128.0 } else { 164.0 };
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, 32.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_max_width(width);
                                let (button, _) = egui::menu::MenuButton::from_button(
                                    egui::Button::new(label)
                                        .truncate()
                                        .min_size(egui::vec2(width, 32.0)),
                                )
                                .ui(ui, |ui| {
                                    for (msgid, level) in [
                                        ("Paragraph", 0),
                                        ("Heading 1", 1),
                                        ("Heading 2", 2),
                                        ("Heading 3", 3),
                                        ("Heading 4", 4),
                                        ("Heading 5", 5),
                                        ("Heading 6", 6),
                                    ] {
                                        if ui.button(l.text(msgid)).clicked() {
                                            capture(
                                                set_block_kind(&mut rich.session, level),
                                                &mut error,
                                            );
                                            rich.session.request_canvas_focus(ui.ctx());
                                            ui.close();
                                        }
                                    }
                                });
                                button.on_hover_text(label);
                            },
                        );
                    },
                );
                ui.separator();
                for (mark, icon, msgid) in [
                    ("bold", Icon::Bold, "Bold"),
                    ("italic", Icon::Italic, "Italic"),
                    ("underline", Icon::Underline, "Underline"),
                ]
                .into_iter()
                .take(if minimal { 1 } else { 3 })
                {
                    capture(
                        mark_button(ui, &mut rich.session, Format::Mark(mark), msgid, icon, l, t),
                        &mut error,
                    );
                }
                if expanded {
                    for (action, icon, msgid) in [
                        (Format::Mark("strike"), Icon::Strike, "Strikethrough"),
                        (Format::Mark("code"), Icon::Code, "Inline code"),
                        (Format::Quote, Icon::Quote, "Block quote"),
                        (Format::Bullet, Icon::BulletList, "Bullet list"),
                        (Format::Ordered, Icon::OrderedList, "Numbered list"),
                    ] {
                        capture(
                            mark_button(ui, &mut rich.session, action, msgid, icon, l, t),
                            &mut error,
                        );
                    }
                }
                ui.separator();
                let insert = icons::icon_button(ui, Icon::Plus, l.text("Insert block"));
                egui::Popup::menu(&insert).show(|ui| {
                    if icons::labeled_button(ui, Icon::Table, l.text("Insert table · 3 × 3"), false)
                        .clicked()
                    {
                        capture(insert_table(&mut rich.session), &mut error);
                        rich.session.request_canvas_focus(ui.ctx());
                        ui.close();
                    }
                    for kind in [Kind::Math, Kind::Code, Kind::Image, Kind::Mermaid] {
                        if icons::labeled_button(ui, kind.icon(), l.text(kind.title()), false)
                            .clicked()
                        {
                            capture(open_dialog(rich, kind, None), &mut error);
                            ui.close();
                        }
                    }
                    ui.separator();
                    for (kind, icon, msgid) in [
                        ("info", Icon::Info, "Information callout"),
                        ("idea", Icon::Idea, "Idea callout"),
                        ("warning", Icon::Warning, "Warning callout"),
                        ("todo", Icon::Todo, "Todo callout"),
                    ] {
                        if icons::labeled_button(ui, icon, l.text(msgid), false).clicked() {
                            capture(insert_callout(&mut rich.session, kind), &mut error);
                            rich.session.request_canvas_focus(ui.ctx());
                            ui.close();
                        }
                    }
                    ui.separator();
                    for (typ, msgid) in [
                        ("horizontalRule", "Horizontal rule"),
                        ("sceneBreak", "Scene break"),
                    ] {
                        if icons::labeled_button(ui, Icon::Minus, l.text(msgid), false).clicked() {
                            capture(insert_separator(&mut rich.session, typ), &mut error);
                            rich.session.request_canvas_focus(ui.ctx());
                            ui.close();
                        }
                    }
                });
                ui.separator();
                for (undo, msgid, icon) in [(true, "Undo", Icon::Undo), (false, "Redo", Icon::Redo)]
                {
                    if ui
                        .add_enabled_ui(
                            if undo {
                                rich.session.can_undo()
                            } else {
                                rich.session.can_redo()
                            },
                            |ui| icons::icon_button(ui, icon, l.text(msgid)),
                        )
                        .inner
                        .clicked()
                    {
                        capture(
                            if undo {
                                rich.session.undo()
                            } else {
                                rich.session.redo()
                            }
                            .map(|_| ())
                            .map_err(Into::into),
                            &mut error,
                        );
                        rich.session.request_canvas_focus(ui.ctx());
                    }
                }
                if !minimal
                    && icons::icon_button(ui, Icon::Search, l.text("Find in document")).clicked()
                {
                    rich.find_requested = true;
                }
                let more = icons::icon_button(ui, Icon::More, l.text("More formatting"));
                egui::Popup::menu(&more).show(|ui| {
                    egui::ScrollArea::vertical()
                        .max_height((ui.ctx().content_rect().height() - 120.0).max(160.0))
                        .show(ui, |ui| {
                            if icons::labeled_button(
                                ui,
                                Icon::Search,
                                l.text("Find in document"),
                                false,
                            )
                            .clicked()
                            {
                                rich.find_requested = true;
                                ui.close();
                            }
                            ui.separator();
                            if minimal {
                                for (mark, label) in [
                                    ("bold", "Bold"),
                                    ("italic", "Italic"),
                                    ("underline", "Underline"),
                                ] {
                                    let action = Format::Mark(mark);
                                    if icons::labeled_button(
                                        ui,
                                        format_icon(action),
                                        l.text(label),
                                        active(&rich.session, action) == MarkState::On,
                                    )
                                    .clicked()
                                    {
                                        capture(format(&mut rich.session, action), &mut error);
                                        rich.session.request_canvas_focus(ui.ctx());
                                        ui.close();
                                    }
                                }
                                ui.separator();
                            }
                            for (action, msgid) in [
                                (Format::Mark("strike"), "Strikethrough"),
                                (Format::Mark("code"), "Inline code"),
                                (Format::Quote, "Block quote"),
                                (Format::Bullet, "Bullet list"),
                                (Format::Ordered, "Numbered list"),
                                (Format::Indent, "Increase indent"),
                                (Format::Outdent, "Decrease indent"),
                            ] {
                                if ui
                                    .add_enabled_ui(
                                        rich.session.selection().is_some()
                                            || rich.session.cell_selection().is_some(),
                                        |ui| {
                                            icons::labeled_button(
                                                ui,
                                                format_icon(action),
                                                l.text(msgid),
                                                active(&rich.session, action) == MarkState::On,
                                            )
                                        },
                                    )
                                    .inner
                                    .clicked()
                                {
                                    capture(format(&mut rich.session, action), &mut error);
                                    rich.session.request_canvas_focus(ui.ctx());
                                    ui.close();
                                }
                            }
                            ui.separator();
                            for kind in [Kind::InlineMath, Kind::Link] {
                                if ui
                                    .add_enabled_ui(
                                        rich.session.selection().is_some()
                                            || rich.session.cell_selection().is_some(),
                                        |ui| {
                                            icons::labeled_button(
                                                ui,
                                                kind.icon(),
                                                l.text(kind.title()),
                                                false,
                                            )
                                        },
                                    )
                                    .inner
                                    .clicked()
                                {
                                    capture(open_dialog(rich, kind, None), &mut error);
                                    ui.close();
                                }
                            }
                            if let Some(target) = editable_target(&rich.session)
                                && icons::labeled_button(
                                    ui,
                                    Icon::TextCursor,
                                    l.text("Edit block"),
                                    false,
                                )
                                .clicked()
                            {
                                if let Some(kind) = Kind::from_node(&target.expected) {
                                    capture(open_dialog(rich, kind, Some(target)), &mut error);
                                }
                                ui.close();
                            }
                            if rich.session.node_selection().is_some()
                                && icons::labeled_button(
                                    ui,
                                    Icon::Trash,
                                    l.text("Delete block"),
                                    false,
                                )
                                .clicked()
                            {
                                capture(
                                    rich.session
                                        .delete_selected_node()
                                        .map(|_| ())
                                        .map_err(Into::into),
                                    &mut error,
                                );
                                rich.session.request_canvas_focus(ui.ctx());
                                ui.close();
                            }
                            if let Some(context) = TableContext::current(&rich.session) {
                                ui.separator();
                                for (msgid, action) in [
                                    ("Add row below", TableAction::AddRow),
                                    ("Add column right", TableAction::AddColumn),
                                    ("Delete row", TableAction::DeleteRow),
                                    ("Delete column", TableAction::DeleteColumn),
                                    ("Merge selected cells", TableAction::Merge),
                                    ("Split cell", TableAction::Split),
                                    ("Delete table", TableAction::DeleteTable),
                                ] {
                                    if ui
                                        .add_enabled(
                                            context.allows(action),
                                            egui::Button::new(l.text(msgid)),
                                        )
                                        .clicked()
                                    {
                                        capture(
                                            context.apply(&mut rich.session, action),
                                            &mut error,
                                        );
                                        rich.session.request_canvas_focus(ui.ctx());
                                        ui.close();
                                    }
                                }
                            }
                        });
                });
            });
        },
    );
    error.map_or(Ok(()), Err)
}
fn insert_callout(session: &mut EditorSession, kind: &str) -> AppResult<()> {
    let text = Node::paragraph(uuid::Uuid::new_v4().to_string(), "");
    let point = DocPoint {
        block_id: text.block_id().ok_or("missing_callout_identity")?.into(),
        offset: ScalarOffset(0),
    };
    let node = Node::block(
        "callout",
        Map::from_iter([
            ("blockId".into(), json!(uuid::Uuid::new_v4().to_string())),
            ("kind".into(), json!(kind)),
        ]),
        vec![text],
    );
    session.insert_content_with_selection(
        ContentSlice::Blocks(vec![node]),
        Selection {
            anchor: point.clone(),
            head: point,
        },
    )?;
    Ok(())
}
fn insert_separator(session: &mut EditorSession, kind: &str) -> AppResult<()> {
    session.insert_content(ContentSlice::Blocks(vec![Node::block(
        kind,
        Map::new(),
        vec![],
    )]))?;
    Ok(())
}

pub(super) fn show_dialog(
    ctx: &egui::Context,
    rich: &mut RichState,
    workspace: &Workspace,
    project_id: &str,
    l: Language,
    t: Tokens,
) {
    let Some(mut dialog) = rich.dialog.take() else {
        return;
    };
    let mut apply = false;
    let mut cancel = false;
    let response = components::modal(
        ctx,
        l,
        "rich-block-properties",
        l.text(dialog.kind.title()),
        620.0,
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("block-properties-body")
                .max_height((ctx.content_rect().height() - 260.0).max(80.0))
                .show(ui, |ui| {
                    if dialog.kind == Kind::Code {
                        ui.label(l.text("Language"));
                        egui::ComboBox::from_id_salt("code-language")
                            .selected_text(if dialog.language.is_empty() {
                                l.text("Plain text")
                            } else {
                                &dialog.language
                            })
                            .show_ui(ui, |ui| {
                                for (language, label) in CODE_LANGUAGES {
                                    ui.selectable_value(
                                        &mut dialog.language,
                                        (*language).into(),
                                        if language.is_empty() {
                                            l.text("Plain text")
                                        } else {
                                            label
                                        },
                                    );
                                }
                            });
                        ui.add_space(12.0);
                    }
                    if dialog.kind == Kind::Image {
                        if components::action(ui, l.text("Choose image…"), true, None).clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter(l.text("Image"), &["png", "jpg", "jpeg", "gif", "webp"])
                                .pick_file()
                        {
                            dialog.image_path = Some(path);
                        }
                        if let Some(path) = &dialog.image_path {
                            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                        }
                        ui.add_space(12.0);
                        ui.label(l.text("Alternative text"));
                        components::singleline(ui, &mut dialog.alt, "");
                    } else if dialog.kind == Kind::Link {
                        ui.label(l.text("Link address"));
                        let field = components::singleline(ui, &mut dialog.source, "https://");
                        if dialog.focus {
                            field.request_focus();
                            dialog.focus = false;
                        }
                        ui.label(l.text("Leave empty to remove the link."));
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(l.text(
                                if matches!(dialog.kind, Kind::Math | Kind::InlineMath) {
                                    "LaTeX source"
                                } else {
                                    "Source"
                                },
                            ));
                            if dialog.kind == Kind::Code {
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icons::icon_button(ui, Icon::Copy, l.text("Copy code"))
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(dialog.source.clone());
                                    }
                                });
                            }
                        });
                        let syntax_language = match dialog.kind {
                            Kind::Code => dialog.language.as_str(),
                            Kind::Mermaid => "mermaid",
                            Kind::Math | Kind::InlineMath => "latex",
                            _ => "",
                        };
                        let field = components::source(
                            ui,
                            &mut dialog.source,
                            if dialog.kind == Kind::Mermaid { 10 } else { 6 },
                            syntax_language,
                            l,
                        );
                        if dialog.focus {
                            field.response.request_focus();
                            dialog.focus = false;
                        }
                    }
                    if dialog.kind != Kind::Link && dialog.kind != Kind::Code {
                        ui.add_space(12.0);
                        ui.label(
                            l.text(if matches!(dialog.kind, Kind::Math | Kind::InlineMath) {
                                "Description"
                            } else {
                                "Caption"
                            }),
                        );
                        components::singleline(ui, &mut dialog.caption, "");
                    }
                    if let Some(error) = &dialog.error {
                        ui.add_space(12.0);
                        ui.colored_label(t.warning, l.text("The block could not be updated."));
                        ui.label(error);
                    }
                });
            components::footer(ui, |ui| {
                apply = components::action(
                    ui,
                    l.text(if dialog.target.is_some() {
                        "Apply"
                    } else {
                        "Insert"
                    }),
                    dialog.can_apply(),
                    Some(t),
                )
                .clicked();
                cancel = components::action(ui, l.text("Cancel"), true, None).clicked();
            });
        },
    );
    if apply {
        match dialog.apply(&mut rich.session, workspace, project_id) {
            Ok(()) => {
                rich.session.request_canvas_focus(ctx);
                return;
            }
            Err(error) => dialog.error = Some(error.to_string()),
        }
    }
    if !cancel && !response.should_close() {
        rich.dialog = Some(dialog);
    } else {
        if rich.session.revision() == dialog.revision
            && let Err(error) = dialog.restore_target(&mut rich.session)
        {
            dialog.error = Some(error.to_string());
            rich.dialog = Some(dialog);
            return;
        }
        rich.session.request_canvas_focus(ctx);
    }
}

// Source option values are preserved verbatim in canonical JSON and Markdown fences.
const CODE_LANGUAGES: &[(&str, &str)] = &[
    ("", "Plain text"),
    ("go", "Go / Golang"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("html", "HTML"),
    ("xml", "XML"),
    ("css", "CSS"),
    ("scss", "SCSS"),
    ("sass", "Sass"),
    ("less", "Less"),
    ("json", "JSON"),
    ("yaml", "YAML"),
    ("markdown", "Markdown"),
    ("bash", "Bash / Shell"),
    ("powershell", "PowerShell"),
    ("python", "Python"),
    ("java", "Java"),
    ("kotlin", "Kotlin"),
    ("c", "C"),
    ("cpp", "C++"),
    ("csharp", "C#"),
    ("rust", "Rust"),
    ("swift", "Swift"),
    ("objectivec", "Objective-C"),
    ("php", "PHP"),
    ("ruby", "Ruby"),
    ("lua", "Lua"),
    ("sql", "SQL"),
    ("r", "R"),
    ("perl", "Perl"),
    ("vbnet", "Visual Basic .NET"),
    ("basic", "BASIC"),
    ("arduino", "Arduino"),
    ("makefile", "Makefile"),
    ("ini", "INI"),
    ("diff", "Diff"),
    ("jsx", "JSX / React"),
    ("tsx", "TSX / React"),
    ("graphql", "GraphQL"),
    ("dockerfile", "Dockerfile"),
    ("hcl", "HCL"),
    ("terraform", "Terraform"),
    ("toml", "TOML"),
    ("protobuf", "Protocol Buffers"),
    ("dart", "Dart"),
    ("scala", "Scala"),
    ("clojure", "Clojure"),
    ("elixir", "Elixir"),
    ("erlang", "Erlang"),
    ("haskell", "Haskell"),
    ("fsharp", "F#"),
    ("ocaml", "OCaml"),
    ("groovy", "Groovy"),
    ("gradle", "Gradle"),
    ("julia", "Julia"),
    ("matlab", "MATLAB"),
    ("fortran", "Fortran"),
    ("pascal", "Pascal"),
    ("cobol", "COBOL"),
    ("lisp", "Lisp"),
    ("scheme", "Scheme"),
    ("solidity", "Solidity"),
    ("nix", "Nix"),
    ("gdscript", "GDScript"),
    ("glsl", "GLSL"),
    ("nasm", "NASM"),
    ("asm", "Assembly"),
    ("cmake", "CMake"),
    ("nginx", "Nginx"),
    ("http", "HTTP"),
    ("promql", "PromQL"),
    ("jq", "jq"),
    ("json5", "JSON5"),
    ("properties", "Java Properties"),
    ("latex", "LaTeX"),
    ("mermaid", "Mermaid"),
    ("plantuml", "PlantUML"),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn select_seed(session: &mut EditorSession) -> AppResult<()> {
        let point = DocPoint {
            block_id: "seed".into(),
            offset: ScalarOffset(0),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        Ok(())
    }

    #[test]
    fn every_heading_level_keeps_the_exact_unicode_range_and_undo() -> AppResult<()> {
        for level in 1..=6 {
            let tree = RichDocument::plain("seed", "Я😀 echo echo")?;
            let mut session = EditorSession::new(tree, Revision(0));
            let selection = Selection {
                anchor: DocPoint {
                    block_id: "seed".into(),
                    offset: ScalarOffset(8),
                },
                head: DocPoint {
                    block_id: "seed".into(),
                    offset: ScalarOffset(12),
                },
            };
            session.set_selection(selection.clone())?;
            let before = session.document().to_json()?;
            set_block_kind(&mut session, level)?;
            let after = session.document().to_json()?;
            assert_eq!(after["content"][0]["attrs"]["level"], json!(level));
            assert_eq!(session.selection(), Some(&selection));
            session.undo()?;
            assert_eq!(session.document().to_json()?, before);
            session.redo()?;
            assert_eq!(session.document().to_json()?, after);
        }
        Ok(())
    }

    #[test]
    fn real_authoring_preserves_all_sources_and_assets_after_save_and_reopen() -> AppResult<()> {
        let temporary = tempfile::tempdir()?;
        let database = temporary.path().join("workspace.sqlite");
        let workspace = Workspace::open(&database)?;
        let project = workspace.create_project("Synthetic authoring")?;
        let initial = RichDocument::plain("seed", "Text remains")?;
        let document =
            workspace.create_rich_document(&project.id, "All authoring", initial.to_json()?)?;
        let mut session = EditorSession::new(initial, Revision(0));
        for kind in [Kind::Mermaid, Kind::Math, Kind::InlineMath, Kind::Code] {
            select_seed(&mut session)?;
            let mut dialog = BlockDialog::new(&session, kind, None)?;
            if kind == Kind::Code {
                dialog.source = "// Unicode α\nlet value = 42;\n".into();
                dialog.language = "rust".into();
            }
            dialog.caption = "Synthetic α caption".into();
            dialog.apply(&mut session, &workspace, &project.id)?;
        }
        for kind in ["info", "idea", "warning", "todo"] {
            select_seed(&mut session)?;
            insert_callout(&mut session, kind)?;
        }
        select_seed(&mut session)?;
        insert_table(&mut session)?;
        select_seed(&mut session)?;
        insert_separator(&mut session, "horizontalRule")?;
        select_seed(&mut session)?;
        insert_separator(&mut session, "sceneBreak")?;
        let image_path = temporary.path().join("original.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 80, 160, 255])).save(&image_path)?;
        select_seed(&mut session)?;
        let mut image_dialog = BlockDialog::new(&session, Kind::Image, None)?;
        image_dialog.image_path = Some(image_path.clone());
        image_dialog.alt = "Synthetic image".into();
        image_dialog.caption = "Durable image".into();
        image_dialog.apply(&mut session, &workspace, &project.id)?;
        let expected = session.document().to_json()?;
        let proofs: Vec<_> = session
            .take_pending_changes()
            .into_iter()
            .map(|change| change.replay_proof())
            .collect();
        workspace.save_rich_editor_changes(
            &document.id,
            document.revision,
            &proofs,
            expected.clone(),
        )?;
        std::fs::remove_file(image_path)?;
        drop(session);
        drop(workspace);
        let reopened = Workspace::open(database)?;
        let saved = reopened.document(&document.id)?;
        assert_eq!(saved.rich_json, Some(expected));
        let tree = RichDocument::from_json(saved.rich_json.ok_or("missing rich content")?)?;
        let image = tree
            .blocks()
            .iter()
            .find(|node| node.node_type == "imageBlock")
            .ok_or("missing image")?;
        let snapshot = reopened.image_asset_snapshot(&project.id, &tree)?;
        assert!(!snapshot.load(image)?.is_empty());
        assert!(tree.plain_text().contains("Text remains"));
        Ok(())
    }

    #[test]
    fn editing_diagram_preserves_identity_opaque_metadata_and_single_history_entry() -> AppResult<()>
    {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Properties")?;
        let tree = RichDocument::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"mermaidBlock","attrs":{"blockId":"diagram","code":"flowchart LR\nA-->B","caption":"Before","custom":{"value":7}},"opaqueRoot":["retained"]},
                {"type":"paragraph","attrs":{"blockId":"seed"},"content":[]}
            ]}),
        )?;
        let original = tree.to_json()?;
        let mut session = EditorSession::new(tree, Revision(0));
        let target = session.document().node_ref(&NodePath(vec![0]))?;
        session.select_node(target.clone())?;
        let mut dialog = BlockDialog::new(&session, Kind::Mermaid, Some(target))?;
        dialog.source = "flowchart TB\nA-->C".into();
        dialog.caption = "After α".into();
        dialog.apply(&mut session, &workspace, &project.id)?;
        let after = session.document().to_json()?;
        assert_eq!(after["content"][0]["attrs"]["custom"], json!({"value":7}));
        assert_eq!(after["content"][0]["opaqueRoot"], json!(["retained"]));
        session.undo()?;
        assert_eq!(session.document().to_json()?, original);
        session.redo()?;
        assert_eq!(session.document().to_json()?, after);
        assert_eq!(
            session
                .node_selection()
                .map(|node| node.expected.string_attr("caption")),
            Some(Some("After α"))
        );
        Ok(())
    }

    #[test]
    fn stale_source_dialog_cannot_overwrite_newer_writing() -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Conflict")?;
        let mut session = EditorSession::new(RichDocument::plain("seed", "Before")?, Revision(0));
        select_seed(&mut session)?;
        let dialog = BlockDialog::new(&session, Kind::Mermaid, None)?;
        session.insert_plain_text("New ")?;
        let before = session.document().to_json()?;
        assert!(dialog.apply(&mut session, &workspace, &project.id).is_err());
        assert_eq!(session.document().to_json()?, before);
        Ok(())
    }

    #[test]
    fn canceling_a_source_dialog_does_not_mutate_the_document_or_history() -> AppResult<()> {
        let mut session = EditorSession::new(RichDocument::plain("seed", "Before")?, Revision(0));
        select_seed(&mut session)?;
        let before = session.document().to_json()?;
        let mut dialog = BlockDialog::new(&session, Kind::Math, None)?;
        dialog.source = "\\frac{1}{2}".into();
        drop(dialog);
        assert_eq!(session.document().to_json()?, before);
        assert!(!session.can_undo());
        Ok(())
    }
    #[test]
    fn inline_formula_property_edit_keeps_the_exact_node_selected_through_history() -> AppResult<()>
    {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Inline")?;
        let tree = RichDocument::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"paragraph","attrs":{"blockId":"seed"},"content":[{"type":"text","text":"Before "},{"type":"inlineMath","attrs":{"latex":"x^2","description":"Before"}},{"type":"text","text":" after"}]}
            ]}),
        )?;
        let mut session = EditorSession::new(tree, Revision(0));
        let target = session.document().node_ref(&NodePath(vec![0, 1]))?;
        session.select_node(target.clone())?;
        let mut dialog = BlockDialog::new(&session, Kind::InlineMath, Some(target))?;
        dialog.source = "y^3".into();
        dialog.apply(&mut session, &workspace, &project.id)?;
        assert_eq!(
            session
                .node_selection()
                .map(|node| node.expected.string_attr("latex")),
            Some(Some("y^3"))
        );
        session.undo()?;
        assert_eq!(
            session
                .node_selection()
                .map(|node| node.expected.string_attr("latex")),
            Some(Some("x^2"))
        );
        session.redo()?;
        assert_eq!(
            session
                .node_selection()
                .map(|node| node.expected.string_attr("latex")),
            Some(Some("y^3"))
        );
        Ok(())
    }

    #[test]
    fn shortening_code_source_clamps_the_caret_in_that_same_block() -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Code")?;
        let tree = RichDocument::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"paragraph","attrs":{"blockId":"seed"},"content":[]},
                {"type":"codeBlock","attrs":{"blockId":"code","language":"rust","custom":7},"content":[{"type":"text","text":"0123456789","customLeaf":9}]}
            ]}),
        )?;
        let mut session = EditorSession::new(tree, Revision(0));
        let point = DocPoint {
            block_id: "code".into(),
            offset: ScalarOffset(10),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        let target = session.document().node_ref(&NodePath(vec![1]))?;
        let mut dialog = BlockDialog::new(&session, Kind::Code, Some(target))?;
        dialog.source = "α".into();
        dialog.apply(&mut session, &workspace, &project.id)?;
        assert_eq!(
            session
                .selection()
                .map(|s| (&s.head.block_id, s.head.offset)),
            Some((&"code".to_string(), ScalarOffset(1)))
        );
        assert_eq!(
            session.document().to_json()?["content"][1]["content"][0]["customLeaf"],
            json!(9)
        );
        session.undo()?;
        assert_eq!(
            session.selection().map(|s| s.head.offset),
            Some(ScalarOffset(10))
        );
        session.redo()?;
        assert_eq!(
            session.selection().map(|s| s.head.offset),
            Some(ScalarOffset(1))
        );
        Ok(())
    }

    #[test]
    fn link_boundary_prefill_does_not_expand_selection_until_apply_and_cancel_restores_it()
    -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Link")?;
        let tree = RichDocument::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"paragraph","attrs":{"blockId":"seed"},"content":[{"type":"text","text":"link","marks":[{"type":"link","attrs":{"href":"https://example.org"}},{"type":"bold"}]},{"type":"text","text":" after"}]}
            ]}),
        )?;
        let mut session = EditorSession::new(tree, Revision(0));
        let point = DocPoint {
            block_id: "seed".into(),
            offset: ScalarOffset(4),
        };
        let original = Selection {
            anchor: point.clone(),
            head: point,
        };
        session.set_selection(original.clone())?;
        let dialog = BlockDialog::new(&session, Kind::Link, None)?;
        assert_eq!(dialog.source, "https://example.org");
        assert_eq!(session.selection(), Some(&original));
        session.extend_mark_range("link")?;
        dialog.restore_target(&mut session)?;
        assert_eq!(session.selection(), Some(&original));
        let mut dialog = BlockDialog::new(&session, Kind::Link, None)?;
        dialog.source = "https://example.net".into();
        dialog.apply(&mut session, &workspace, &project.id)?;
        let value = session.document().to_json()?;
        let marks = value["content"][0]["content"][0]["marks"]
            .as_array()
            .ok_or("missing marks")?;
        assert!(marks.iter().any(|mark| mark["type"] == "bold"));
        assert!(
            marks
                .iter()
                .any(|mark| mark["attrs"]["href"] == "https://example.net")
        );
        session.undo()?;
        assert_eq!(session.selection(), Some(&original));
        Ok(())
    }

    #[test]
    fn insert_dialog_applies_to_its_captured_selection_even_if_the_caret_moves() -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Target")?;
        let mut session = EditorSession::new(RichDocument::plain("seed", "Writing")?, Revision(0));
        select_seed(&mut session)?;
        let dialog = BlockDialog::new(&session, Kind::Mermaid, None)?;
        let point = DocPoint {
            block_id: "seed".into(),
            offset: ScalarOffset(7),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        dialog.apply(&mut session, &workspace, &project.id)?;
        assert_eq!(
            session
                .document()
                .blocks()
                .first()
                .map(|node| node.node_type.as_str()),
            Some("mermaidBlock")
        );
        Ok(())
    }
    #[test]
    fn imported_multileaf_code_edits_keep_surviving_metadata_and_single_step_undo() -> AppResult<()>
    {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Imported code")?;
        let tree = RichDocument::from_json(
            json!({"type":"doc","attrs":{"schemaVersion":4},"content":[
                {"type":"codeBlock","attrs":{"blockId":"code","customBlock":7},"content":[
                    {"type":"text","text":"α prefix ","leafSource":"first"},
                    {"type":"text","text":"old","leafSource":"middle"},
                    {"type":"text","text":" suffix 界","leafSource":"last"}
                ]},
                {"type":"paragraph","attrs":{"blockId":"after"},"content":[{"type":"text","text":"Unchanged"}]}
            ]}),
        )?;
        let original = tree.to_json()?;
        let mut session = EditorSession::new(tree, Revision(0));
        let point = DocPoint {
            block_id: "code".into(),
            offset: ScalarOffset(session.document().block_text("code")?.chars().count()),
        };
        session.set_selection(Selection {
            anchor: point.clone(),
            head: point,
        })?;
        let target = session.document().node_ref(&NodePath(vec![0]))?;
        let mut dialog = BlockDialog::new(&session, Kind::Code, Some(target))?;
        dialog.source = "α prefix NEW😀 suffix 界".into();
        dialog.apply(&mut session, &workspace, &project.id)?;
        let content = session.document().to_json()?;
        assert_eq!(session.document().block_text("code")?, dialog.source);
        assert_eq!(content["content"][0]["content"][0]["leafSource"], "first");
        assert_eq!(
            content["content"][0]["content"]
                .as_array()
                .and_then(|rows| rows.last())
                .and_then(|leaf| leaf.get("leafSource")),
            Some(&json!("last"))
        );
        assert!(content["content"][0]["attrs"].get("language").is_none());
        assert_eq!(content["content"][1], original["content"][1]);
        let changes = session.take_pending_changes();
        assert_eq!(changes.len(), 1);
        assert!(
            changes[0]
                .mappings
                .iter()
                .any(|mapping| matches!(mapping, craft_editor_model::PositionMap::Text { .. }))
        );
        session.undo()?;
        assert_eq!(session.document().to_json()?, original);
        assert!(!session.can_undo());
        session.redo()?;
        assert_eq!(session.document().to_json()?, content);
        Ok(())
    }

    #[test]
    fn source_dialog_keeps_cells_and_rejects_foreign_same_revision_sessions() -> AppResult<()> {
        let workspace = Workspace::memory()?;
        let project = workspace.create_project("Dialog scope")?;
        let tree = RichDocument::from_json(json!({
            "type":"doc", "attrs":{"schemaVersion":4}, "content":[{
                "type":"table", "attrs":{"blockId":"table"}, "content":[{
                    "type":"tableRow", "content":[
                        {"type":"tableCell", "content":[{"type":"paragraph", "attrs":{"blockId":"a"}, "content":[{"type":"text","text":"First"}]}]},
                        {"type":"tableCell", "content":[{"type":"paragraph", "attrs":{"blockId":"b"}, "content":[{"type":"text","text":"Second"}]}]}
                    ]
                }]
            }]
        }))?;
        let mut session = EditorSession::new(tree.clone(), Revision(0));
        let cells = craft_editor_egui::TableSelection {
            table_path: NodePath(vec![0]),
            anchor: craft_editor_egui::CellPoint { row: 0, column: 1 },
            head: craft_editor_egui::CellPoint { row: 0, column: 0 },
        };
        session.set_cell_selection(cells.clone())?;
        let dialog = BlockDialog::new(&session, Kind::Mermaid, None)?;
        let caret = DocPoint {
            block_id: "b".into(),
            offset: ScalarOffset(2),
        };
        session.set_selection(Selection {
            anchor: caret.clone(),
            head: caret,
        })?;
        dialog.restore_target(&mut session)?;
        assert_eq!(session.cell_selection(), Some(&cells));
        assert!(session.selection().is_none());

        let mut foreign = EditorSession::new(tree, Revision(0));
        let caret = DocPoint {
            block_id: "b".into(),
            offset: ScalarOffset(3),
        };
        foreign.set_selection(Selection {
            anchor: caret.clone(),
            head: caret,
        })?;
        let before = foreign.document().to_json()?;
        let selection = foreign.selection_state()?;
        assert!(dialog.apply(&mut foreign, &workspace, &project.id).is_err());
        assert_eq!(foreign.document().to_json()?, before);
        assert_eq!(foreign.selection_state()?, selection);
        assert!(!foreign.can_undo());
        Ok(())
    }
}
