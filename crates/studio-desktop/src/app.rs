//! Bounded native writer workspace: navigation, manuscript and review inspector.
use crate::{
    icons::{self, Icon},
    theme::{self, Appearance, Palette, ThemeSettings, Tokens},
};
use egui::{Align, Color32, FontFamily, FontId, Layout, RichText, Stroke, Vec2};
use std::{
    io::Write,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use studio_core::{Document, Project, Proposal, ProposalStatus, Workspace};

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, PartialEq)]
enum Inspector {
    Review,
    Versions,
}

pub struct StudioApp {
    workspace: Workspace,
    database: PathBuf,
    projects: Vec<Project>,
    project: Option<Project>,
    documents: Vec<Document>,
    document: Option<Document>,
    body: String,
    dirty: Option<Instant>,
    library: bool,
    navigator: bool,
    inspector_visible: bool,
    inspector: Inspector,
    search: String,
    status: String,
    error: Option<String>,
    settings: ThemeSettings,
    settings_open: bool,
    font_size: f32,
    project_dialog: bool,
    project_name: String,
    document_dialog: bool,
    document_name: String,
    expected: String,
    replacement: String,
    reason: String,
    proposals: Vec<Proposal>,
    import_job: Option<mpsc::Receiver<Result<studio_transfer::ImportPreview, String>>>,
    import_preview: Option<studio_transfer::ImportPreview>,
    capture: Option<PathBuf>,
    frames: u32,
}

impl StudioApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        database: PathBuf,
        demo: bool,
        capture: Option<PathBuf>,
    ) -> AppResult<Self> {
        let workspace = Workspace::open(&database)?;
        let settings = std::fs::read(database.with_file_name("appearance.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, &settings);
        let mut app = Self {
            workspace,
            database,
            projects: vec![],
            project: None,
            documents: vec![],
            document: None,
            body: String::new(),
            dirty: None,
            library: true,
            navigator: true,
            inspector_visible: true,
            inspector: Inspector::Review,
            search: String::new(),
            status: "Все изменения сохранены".into(),
            error: None,
            settings,
            settings_open: false,
            font_size: 21.0,
            project_dialog: false,
            project_name: String::new(),
            document_dialog: false,
            document_name: String::new(),
            expected: String::new(),
            replacement: String::new(),
            reason: String::new(),
            proposals: vec![],
            import_job: None,
            import_preview: None,
            capture,
            frames: 0,
        };
        app.projects = app.workspace.list_projects()?;
        if std::env::args().any(|argument| argument == "--settings") {
            app.settings_open = true;
        }
        if std::env::args().any(|argument| argument == "--light") {
            app.settings.appearance = Appearance::Light;
            theme::apply(&cc.egui_ctx, &app.settings);
        }
        if demo {
            if app.projects.is_empty() {
                app.create_example()?;
            }
            if let Some(project) = app.projects.first().cloned() {
                app.open_project(project)?;
            }
        }
        Ok(app)
    }

    fn create_example(&mut self) -> AppResult<()> {
        let project = self.workspace.create_project("Там, где начинается море")?;
        self.workspace
            .create_document(&project.id, "01. Дом у маяка", SAMPLE)?;
        self.workspace.create_document(&project.id, "02. Письмо без адреса", "На обороте конверта было только одно слово: «Вернись».\n\nАся прочитала его дважды и положила письмо на подоконник. За стеклом начинался дождь.")?;
        self.workspace.create_document(&project.id, "Заметки к истории", "Ася — реставратор, возвращается в приморский город детства.\n\nМаяк перестал работать девятнадцать лет назад.\n\nГлавная тема: иногда вернуться — значит впервые выбрать свой путь.")?;
        self.projects = self.workspace.list_projects()?;
        Ok(())
    }

    fn open_project(&mut self, project: Project) -> AppResult<()> {
        self.save()?;
        self.documents = self.workspace.list_documents(&project.id)?;
        self.project = Some(project);
        self.document = None;
        self.body.clear();
        self.library = false;
        if let Some(document) = self.documents.first().cloned() {
            self.open_document(document)?;
        }
        Ok(())
    }

    fn open_document(&mut self, document: Document) -> AppResult<()> {
        self.save()?;
        let document = self.workspace.document(&document.id)?;
        self.body.clone_from(&document.body);
        self.dirty = None;
        if let Some(draft) = self.workspace.draft(&document.id)? {
            if draft.base_revision == document.revision && draft.body != document.body {
                self.body = draft.body;
                self.dirty = Some(Instant::now());
                self.status = "Восстановлен несохранённый текст".into();
            } else if draft.base_revision != document.revision {
                self.workspace.recover_draft_as_document(&document.id)?;
                self.documents = self.workspace.list_documents(&document.project_id)?;
                self.status = "Несохранённый текст восстановлен отдельным документом".into();
            }
        }
        self.proposals = self.workspace.proposals(&document.id)?;
        self.document = Some(document);
        self.expected.clear();
        self.replacement.clear();
        Ok(())
    }

    fn save(&mut self) -> AppResult<()> {
        if self.dirty.is_none() {
            return Ok(());
        }
        if let Some(document) = self.document.as_ref() {
            let saved =
                self.workspace
                    .save_document(&document.id, document.revision, &self.body)?;
            self.document = Some(saved);
            if let Some(project) = &self.project {
                self.documents = self.workspace.list_documents(&project.id)?;
            }
            self.dirty = None;
            self.status = "Все изменения сохранены".into();
        }
        Ok(())
    }

    fn record_error(&mut self, result: AppResult<()>) {
        if let Err(error) = result {
            self.error = Some(error.to_string());
            self.status = "Требуется внимание".into();
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, t: Tokens) {
        egui::Panel::top("commands")
            .exact_size(48.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ui, |ui| {
                let drag = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("titlebar-drag"),
                    egui::Sense::drag(),
                );
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.horizontal_centered(|ui| {
                    if cfg!(target_os = "macos") {
                        ui.add_space(76.0);
                    }
                    icons::icon(ui, Icon::Book, 22.0, t.accent);
                    ui.label(RichText::new("WriterCraft").size(17.0).strong());
                    ui.add_space(10.0);
                    if icons::icon_button(ui, Icon::Home, "Библиотека проектов").clicked()
                    {
                        let result = self.save();
                        if result.is_ok() {
                            self.library = true;
                        }
                        self.record_error(result);
                    }
                    ui.separator();
                    ui.menu_button("Файл", |ui| {
                        if ui.button("Новый проект…     ⌘N").clicked() {
                            self.project_dialog = true;
                            ui.close();
                        }
                        if ui.button("Открыть .wstudio…     ⌘O").clicked() {
                            self.choose_archive();
                            ui.close();
                        }
                        if ui.button("Экспорт текста…").clicked() {
                            self.export_text();
                            ui.close();
                        }
                    });
                    if icons::icon_button(ui, Icon::Save, "Сохранить · ⌘S").clicked() {
                        let result = self.save();
                        self.record_error(result);
                    }
                    if !self.library {
                        ui.separator();
                        if let Some(project) = &self.project {
                            ui.label(RichText::new(&project.title).size(14.0).color(t.muted));
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(ui, Icon::Settings, "Настройки оформления · ⌘,")
                            .clicked()
                        {
                            self.settings_open = true;
                        }
                        if !self.library {
                            if icons::icon_button(ui, Icon::Assistant, "Панель правок · ⌘⇧A")
                                .clicked()
                            {
                                self.inspector_visible = !self.inspector_visible;
                            }
                            if icons::icon_button(ui, Icon::Sidebar, "Структура проекта").clicked()
                            {
                                self.navigator = !self.navigator;
                            }
                        }
                    });
                });
            });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui, t: Tokens) {
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(16, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon(ui, Icon::Shield, 13.0, t.good);
                    ui.label(
                        RichText::new("На этом компьютере")
                            .size(12.0)
                            .color(t.muted),
                    );
                    ui.separator();
                    ui.label(RichText::new(&self.status).size(12.0).color(
                        if self.error.is_some() {
                            t.warning
                        } else {
                            t.muted
                        },
                    ));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(document) = &self.document {
                            if !self.library {
                                ui.label(
                                    RichText::new(format!(
                                        "{} слов   ·   Версия {}",
                                        self.body.split_whitespace().count(),
                                        document.revision
                                    ))
                                    .size(12.0)
                                    .color(t.muted),
                                );
                            }
                        }
                    });
                });
            });
    }

    fn navigation(&mut self, ui: &mut egui::Ui, t: Tokens) {
        egui::Panel::left("navigator")
            .default_size(248.0)
            .min_size(210.0)
            .max_size(340.0)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(14, 18)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.library {
                            "БИБЛИОТЕКА"
                        } else {
                            "СТРУКТУРА"
                        })
                        .size(11.0)
                        .strong()
                        .color(t.muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(
                            ui,
                            Icon::Plus,
                            if self.library {
                                "Создать проект"
                            } else {
                                "Добавить документ"
                            },
                        )
                        .clicked()
                        {
                            if self.library {
                                self.project_dialog = true;
                            } else {
                                self.document_dialog = true;
                            }
                        }
                    });
                });
                ui.add_space(12.0);
                if self.library {
                    nav_row(ui, Icon::Book, "Все проекты", true, t);
                    ui.add_space(6.0);
                    if nav_row(ui, Icon::Import, "Открыть .wstudio", false, t).clicked() {
                        self.choose_archive();
                    }
                    ui.add_space(24.0);
                    ui.label(
                        RichText::new("РАБОЧЕЕ ПРОСТРАНСТВО")
                            .size(10.5)
                            .strong()
                            .color(t.muted),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Рукопись, материалы и история — вместе с вашим проектом.")
                            .size(13.0)
                            .color(t.muted),
                    );
                } else {
                    if let Some(project) = &self.project {
                        ui.label(RichText::new(&project.title).size(17.0).strong());
                        ui.label(
                            RichText::new(format!("{} документов", self.documents.len()))
                                .size(12.0)
                                .color(t.muted),
                        );
                    }
                    ui.add_space(18.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Найти документ…")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        icons::icon(ui, Icon::Folder, 16.0, t.muted);
                        ui.label(RichText::new("Рукопись").size(13.0).strong());
                    });
                    ui.add_space(6.0);
                    let mut selected = None;
                    egui::ScrollArea::vertical()
                        .id_salt("document-tree")
                        .auto_shrink([false, false])
                        .max_height((ui.available_height() - 90.0).max(80.0))
                        .show(ui, |ui| {
                            for document in &self.documents {
                                if !self.search.is_empty()
                                    && !document
                                        .title
                                        .to_lowercase()
                                        .contains(&self.search.to_lowercase())
                                {
                                    continue;
                                }
                                let active = self
                                    .document
                                    .as_ref()
                                    .is_some_and(|current| current.id == document.id);
                                if nav_row(ui, Icon::Document, &document.title, active, t).clicked()
                                {
                                    selected = Some(document.clone());
                                }
                                ui.add_space(3.0);
                            }
                        });
                    if let Some(document) = selected {
                        let result = self.open_document(document);
                        self.record_error(result);
                    }
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);
                    if nav_row(
                        ui,
                        Icon::History,
                        "История документа",
                        self.inspector_visible && self.inspector == Inspector::Versions,
                        t,
                    )
                    .clicked()
                    {
                        self.inspector = Inspector::Versions;
                        self.inspector_visible = true;
                    }
                }
            });
    }

    fn library(&mut self, ui: &mut egui::Ui, t: Tokens) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin::symmetric(44, 34)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Ваше место для больших историй").size(29.0).strong());
                        ui.add_space(8.0);
                        ui.label(RichText::new("Откройте рукопись и продолжите с того места, где остановились.").size(15.0).color(t.muted));
                    });
                });
                ui.add_space(28.0);
                ui.horizontal(|ui| {
                    if primary(ui, "Новый проект", t).clicked() { self.project_dialog = true; }
                    if ui.button("Открыть .wstudio…").clicked() { self.choose_archive(); }
                    if self.projects.is_empty() && ui.button("Попробовать на примере").clicked() {
                        let result = self.create_example(); self.record_error(result);
                    }
                });
                ui.add_space(34.0);
                ui.label(RichText::new("ПРОЕКТЫ").size(11.0).strong().color(t.muted));
                ui.add_space(12.0); ui.separator();
                let mut selected = None;
                egui::ScrollArea::vertical().id_salt("library-projects").show(ui, |ui| {
                    for project in &self.projects {
                        let row = egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 20)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                icons::icon(ui, Icon::Book, 27.0, t.accent); ui.add_space(14.0);
                                if ui.add(egui::Button::new(RichText::new(&project.title).size(19.0).strong()).frame(false)).clicked() { selected = Some(project.clone()); }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icons::icon_button(ui, Icon::ArrowRight, "Открыть проект").clicked() { selected = Some(project.clone()); }
                                    ui.label(RichText::new("Локальный проект").size(12.0).color(t.muted));
                                });
                            });
                        });
                        ui.painter().hline(row.response.rect.x_range(), row.response.rect.bottom(), Stroke::new(1.0,t.border));
                    }
                    if self.projects.is_empty() {
                        ui.add_space(60.0);
                        ui.label(RichText::new("Первая глава начинается здесь").size(23.0));
                        ui.add_space(8.0);
                        ui.label(RichText::new("Создайте проект. Для письма не нужны аккаунт, сервер или подключение к сети.").color(t.muted));
                    }
                });
                if let Some(project) = selected { let result = self.open_project(project); self.record_error(result); }
            });
    }

    fn editor(&mut self, ui: &mut egui::Ui, t: Tokens) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(t.editor).inner_margin(0))
            .show(ui, |ui| {
                let Some(document) = self.document.clone() else {
                    ui.centered_and_justified(|ui| {
                        if ui.button("Добавить первый документ").clicked() {
                            self.document_dialog = true;
                        }
                    });
                    return;
                };
                egui::Panel::top("document-heading")
                    .exact_size(110.0)
                    .resizable(false)
                    .frame(
                        egui::Frame::new()
                            .fill(t.editor)
                            .inner_margin(egui::Margin::symmetric(34, 22)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("РУКОПИСЬ").size(10.5).strong().color(t.muted));
                            if document.read_only {
                                ui.label(
                                    RichText::new("Исходный формат · только чтение")
                                        .size(11.0)
                                        .color(t.warning),
                                );
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icons::icon_button(ui, Icon::Export, "Экспортировать текст")
                                    .clicked()
                                {
                                    self.export_text();
                                }
                                if ui
                                    .button("Фокус")
                                    .on_hover_text("Скрыть боковые панели · ⌘⇧F")
                                    .clicked()
                                {
                                    self.navigator = false;
                                    self.inspector_visible = false;
                                }
                            });
                        });
                        ui.add_space(10.0);
                        ui.label(RichText::new(&document.title).size(28.0).strong());
                    });
                egui::ScrollArea::vertical()
                    .id_salt((&document.id, "manuscript"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = (ui.available_width() - 72.0).clamp(240.0, 710.0);
                        let offset = ((ui.available_width() - width) * 0.5).max(24.0);
                        ui.horizontal(|ui| {
                            ui.add_space(offset);
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                ui.add_space(24.0);
                                let output = egui::TextEdit::multiline(&mut self.body)
                                    .id_salt((&document.id, "body"))
                                    .font(FontId::new(
                                        self.font_size,
                                        FontFamily::Name("Manuscript".into()),
                                    ))
                                    .text_color(t.text)
                                    .frame(egui::Frame::NONE)
                                    .margin(0)
                                    .desired_width(width)
                                    .desired_rows(24)
                                    .interactive(!document.read_only)
                                    .hint_text("Начните писать…")
                                    .show(ui);
                                if output.response.changed() {
                                    self.dirty = Some(Instant::now());
                                    self.status = "Сохраняется…".into();
                                    let result = self
                                        .workspace
                                        .save_draft(&document.id, document.revision, &self.body)
                                        .map_err(Into::into);
                                    self.record_error(result);
                                }
                                if let Some(range) =
                                    output.cursor_range.filter(|range| !range.is_empty())
                                {
                                    let range = range.as_sorted_char_range();
                                    self.expected = self
                                        .body
                                        .chars()
                                        .skip(usize::from(range.start))
                                        .take(usize::from(range.end) - usize::from(range.start))
                                        .collect();
                                }
                                ui.add_space(80.0);
                            });
                        });
                    });
            });
    }

    fn inspector(&mut self, ui: &mut egui::Ui, t: Tokens) {
        egui::Panel::right("inspector")
            .default_size(320.0)
            .min_size(280.0)
            .max_size(460.0)
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(20, 18)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.inspector == Inspector::Review, "Правки")
                        .clicked()
                    {
                        self.inspector = Inspector::Review;
                    }
                    if ui
                        .selectable_label(self.inspector == Inspector::Versions, "История")
                        .clicked()
                    {
                        self.inspector = Inspector::Versions;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(ui, Icon::Close, "Скрыть панель").clicked()
                        {
                            self.inspector_visible = false;
                        }
                    });
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(16.0);
                egui::ScrollArea::vertical()
                    .id_salt("inspector-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.inspector == Inspector::Versions {
                            self.versions(ui, t);
                        } else {
                            self.review(ui, t);
                        }
                    });
            });
    }

    fn review(&mut self, ui: &mut egui::Ui, t: Tokens) {
        ui.horizontal(|ui| {
            icons::icon(ui, Icon::Assistant, 22.0, t.accent);
            ui.label(RichText::new("Под вашим контролем").size(17.0).strong());
        });
        ui.add_space(10.0);
        ui.label(RichText::new("Проверьте предложение перед изменением рукописи. Оригинал остаётся нетронутым до применения.").size(13.0).color(t.muted));
        ui.add_space(16.0);
        let editable = self
            .document
            .as_ref()
            .is_some_and(|document| !document.read_only);
        egui::CollapsingHeader::new("Подготовить точную правку")
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Исходный фрагмент").size(12.0).color(t.muted));
                ui.add(
                    egui::TextEdit::multiline(&mut self.expected)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .hint_text("Выделите текст в рукописи или вставьте точный фрагмент"),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Предлагаемый текст")
                        .size(12.0)
                        .color(t.muted),
                );
                ui.add(
                    egui::TextEdit::multiline(&mut self.replacement)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY)
                        .hint_text("Новая формулировка"),
                );
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.reason)
                        .desired_width(f32::INFINITY)
                        .hint_text("Причина правки (необязательно)"),
                );
                ui.add_space(12.0);
                let ready =
                    editable && !self.expected.is_empty() && self.expected != self.replacement;
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new("Подготовить предложение")
                            .min_size(Vec2::new(ui.available_width(), 34.0)),
                    )
                    .clicked()
                {
                    let result = self.stage();
                    self.record_error(result);
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Локальный инструмент правок. AI-провайдер пока не подключён.")
                        .size(11.5)
                        .color(t.muted),
                );
            });
        if !self.proposals.is_empty() {
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);
        }
        for proposal in self.proposals.clone() {
            match proposal.status {
                ProposalStatus::Staged => {
                    ui.label(
                        RichText::new("Предлагаемые изменения · не применены")
                            .size(12.0)
                            .strong()
                            .color(t.warning),
                    );
                    if !proposal.reason.is_empty() {
                        ui.add_space(6.0);
                        ui.label(&proposal.reason);
                    }
                    ui.add_space(10.0);
                    diff_block(ui, "Исходный текст", &proposal.before, t.surface, t);
                    ui.add_space(8.0);
                    diff_block(ui, "Предложение", &proposal.after, t.accent_soft, t);
                    ui.add_space(12.0);
                    if primary(ui, "Применить изменения", t).clicked() {
                        let result = self.apply(&proposal.id);
                        self.record_error(result);
                    }
                    if ui.button("Отклонить").clicked() {
                        let result = self
                            .workspace
                            .reject_proposal(&proposal.id)
                            .map_err(Into::into)
                            .and_then(|()| self.refresh_proposals());
                        self.record_error(result);
                    }
                }
                ProposalStatus::Applied => {
                    ui.label(RichText::new("Изменения применены").color(t.good));
                    if ui.button("Отменить применение").clicked() {
                        let result = self.undo(&proposal.id);
                        self.record_error(result);
                    }
                }
                ProposalStatus::Rejected => {
                    ui.label(
                        RichText::new("Предложение отклонено")
                            .size(12.0)
                            .color(t.muted),
                    );
                }
                ProposalStatus::Undone => {
                    ui.label(
                        RichText::new("Применение отменено")
                            .size(12.0)
                            .color(t.muted),
                    );
                }
            }
            ui.add_space(18.0);
            ui.separator();
            ui.add_space(12.0);
        }
    }

    fn versions(&self, ui: &mut egui::Ui, t: Tokens) {
        ui.label(RichText::new("История документа").size(17.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("Сохранённые версии остаются в локальном проекте.")
                .size(13.0)
                .color(t.muted),
        );
        ui.add_space(18.0);
        if let Some(document) = &self.document {
            match self.workspace.revisions(&document.id) {
                Ok(revisions) => {
                    for revision in revisions {
                        egui::CollapsingHeader::new(format!(
                            "Версия {} · {} слов",
                            revision.revision,
                            revision.body.split_whitespace().count()
                        ))
                        .id_salt(revision.revision)
                        .show(ui, |ui| {
                            ui.label(RichText::new(&revision.body).size(13.0));
                        });
                        ui.add_space(6.0);
                    }
                }
                Err(error) => {
                    ui.colored_label(t.warning, error.to_string());
                }
            }
        }
    }

    fn stage(&mut self) -> AppResult<()> {
        self.save()?;
        if let Some(document) = &self.document {
            self.workspace.stage_replace(
                &document.id,
                document.revision,
                &self.expected,
                &self.replacement,
                if self.reason.trim().is_empty() {
                    "Локальная правка автора"
                } else {
                    &self.reason
                },
            )?;
            self.refresh_proposals()?;
            self.status = "Предложение подготовлено; рукопись не изменена".into();
        }
        Ok(())
    }
    fn refresh_proposals(&mut self) -> AppResult<()> {
        if let Some(document) = &self.document {
            self.proposals = self.workspace.proposals(&document.id)?;
        }
        Ok(())
    }
    fn apply(&mut self, id: &str) -> AppResult<()> {
        self.save()?;
        let document = self.workspace.apply_proposal(id)?;
        self.open_document(document)?;
        self.status = "Изменения применены; доступна отмена".into();
        Ok(())
    }
    fn undo(&mut self, id: &str) -> AppResult<()> {
        self.save()?;
        let document = self.workspace.undo_proposal(id)?;
        self.open_document(document)?;
        self.status = "Применение отменено".into();
        Ok(())
    }

    fn choose_archive(&mut self) {
        if self.import_job.is_some() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Проект Writers Studio", &["wstudio"])
            .pick_file()
        {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result =
                    studio_transfer::inspect_archive(path).map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            self.import_job = Some(receiver);
            self.status = "Проверяется архив проекта…".into();
        }
    }
    fn export_text(&mut self) {
        let Some(document) = &self.document else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.txt", document.title))
            .add_filter("Обычный текст", &["txt"])
            .save_file()
        {
            let result = atomic_write(&path, self.body.as_bytes());
            if result.is_ok() {
                self.status = "Текст экспортирован".into();
            }
            self.record_error(result);
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context, t: Tokens) {
        if self.project_dialog {
            let mut open = true;
            egui::Window::new("Новый проект")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label("Название проекта");
                    ui.add_space(6.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.project_name)
                            .desired_width(f32::INFINITY)
                            .hint_text("Моя новая история"),
                    );
                    if response.gained_focus() {
                        response.request_focus();
                    }
                    ui.add_space(16.0);
                    if ui
                        .add_enabled(
                            !self.project_name.trim().is_empty(),
                            egui::Button::new("Создать проект"),
                        )
                        .clicked()
                    {
                        let result = (|| -> AppResult<()> {
                            let project =
                                self.workspace.create_project(self.project_name.trim())?;
                            self.workspace
                                .create_document(&project.id, "Первая глава", "")?;
                            self.projects = self.workspace.list_projects()?;
                            self.open_project(project)?;
                            Ok(())
                        })();
                        if result.is_ok() {
                            self.project_dialog = false;
                            self.project_name.clear();
                        }
                        self.record_error(result);
                    }
                });
            if !open {
                self.project_dialog = false;
            }
        }
        if self.document_dialog {
            let mut open = true;
            egui::Window::new("Новый документ")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label("Название документа");
                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.document_name)
                            .desired_width(f32::INFINITY)
                            .hint_text("Новая глава"),
                    );
                    ui.add_space(16.0);
                    if ui
                        .add_enabled(
                            !self.document_name.trim().is_empty(),
                            egui::Button::new("Добавить документ"),
                        )
                        .clicked()
                    {
                        let result = (|| -> AppResult<()> {
                            self.save()?;
                            if let Some(project) = &self.project {
                                let document = self.workspace.create_document(
                                    &project.id,
                                    self.document_name.trim(),
                                    "",
                                )?;
                                self.documents = self.workspace.list_documents(&project.id)?;
                                self.open_document(document)?;
                            }
                            Ok(())
                        })();
                        if result.is_ok() {
                            self.document_dialog = false;
                            self.document_name.clear();
                        }
                        self.record_error(result);
                    }
                });
            if !open {
                self.document_dialog = false;
            }
        }
        if self.settings_open {
            self.settings_dialog(ctx, t);
        }
        if let Some(preview) = &self.import_preview {
            let mut open = true;
            egui::Window::new("Проверка проекта .wstudio").open(&mut open).default_width(480.0).collapsible(false).show(ctx,|ui| {
                ui.heading(&preview.title); ui.add_space(10.0);
                ui.label(format!("Версия формата: {} · Документов: {}",preview.format_version,preview.documents.len()));
                ui.add_space(12.0);
                ui.label("Архив успешно проверен. В этом MVP доступен просмотр структуры; перенос полного проекта будет добавлен после проверки совместимости rich-редактора.");
                ui.add_space(12.0);
                egui::ScrollArea::vertical().max_height(300.0).show(ui,|ui| {
                    for document in &preview.documents {
                        egui::CollapsingHeader::new(&document.title).id_salt(&document.id).show(ui,|ui| {ui.label(&document.plain_text);});
                    }
                });
            });
            if !open {
                self.import_preview = None;
            }
        }
        if let Some(message) = self.error.clone() {
            let mut open = true;
            egui::Window::new("Требуется внимание")
                .open(&mut open)
                .default_width(440.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&message);
                    ui.add_space(12.0);
                    if ui.button("Понятно").clicked() {
                        self.error = None;
                    }
                });
            if !open {
                self.error = None;
            }
        }
    }

    fn settings_dialog(&mut self, ctx: &egui::Context, t: Tokens) {
        let mut open = true;
        let previous = self.settings.clone();
        egui::Window::new("Настройки · WriterCraft").open(&mut open).collapsible(false).resizable(false).default_width(590.0).show(ctx,|ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {icons::icon(ui,Icon::Settings,23.0,t.accent);ui.heading("Оформление");});
            ui.add_space(8.0); ui.label(RichText::new("Спокойная рабочая среда, которая подходит именно вам.").color(t.muted)); ui.add_space(22.0);
            ui.label(RichText::new("Режим").strong()); ui.add_space(8.0);
            ui.horizontal(|ui| {for appearance in Appearance::ALL {ui.selectable_value(&mut self.settings.appearance,appearance,appearance.label());}});
            ui.add_space(24.0); ui.label(RichText::new("Палитра").strong()); ui.add_space(8.0);
            egui::Grid::new("palette-options").num_columns(2).spacing([12.0,8.0]).show(ui,|ui| {
                for (index,palette) in Palette::ALL.iter().copied().enumerate() {
                    ui.selectable_value(&mut self.settings.palette,palette,palette.label());
                    if index%2==1 {ui.end_row();}
                }
            });
            ui.add_space(24.0); ui.separator(); ui.add_space(18.0);
            ui.horizontal(|ui| {ui.label("Размер текста рукописи");ui.add(egui::Slider::new(&mut self.font_size,16.0..=28.0).suffix(" px"));});
            ui.add_space(16.0);
            ui.label(RichText::new("Сочетания клавиш").strong()); ui.add_space(8.0);
            ui.label(RichText::new("⌘S — сохранить     ⌘N — новый проект     ⌘O — открыть архив\n⌘⇧A — панель правок     ⌘⇧F — режим фокуса     ⌘, — настройки").size(12.0).color(t.muted));
            ui.add_space(20.0);
            ui.with_layout(Layout::right_to_left(Align::Center),|ui| {if primary(ui,"Готово",t).clicked() {self.settings_open=false;}});
        });
        if self.settings != previous {
            theme::apply(ctx, &self.settings);
            let result = serde_json::to_vec_pretty(&self.settings)
                .map_err(Into::into)
                .and_then(|bytes| {
                    atomic_write(&self.database.with_file_name("appearance.json"), &bytes)
                });
            self.record_error(result);
        }
        if !open {
            self.settings_open = false;
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            let result = self.save();
            self.record_error(result);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.project_dialog = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.choose_archive();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.settings_open = true;
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::A,
            )
        }) {
            self.inspector_visible = !self.inspector_visible;
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::F,
            )
        }) {
            let show = !(self.navigator || self.inspector_visible);
            self.navigator = show;
            self.inspector_visible = show;
        }
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let t = theme::tokens(&ctx);
        self.shortcuts(&ctx);
        if self
            .dirty
            .is_some_and(|instant| instant.elapsed() > Duration::from_millis(650))
        {
            let result = self.save();
            self.record_error(result);
        }
        if self.dirty.is_some() || self.import_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            let result = self.save();
            if result.is_err() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            self.record_error(result);
        }
        if let Some(receiver) = &self.import_job {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(preview) => {
                        self.import_preview = Some(preview);
                        self.status = "Архив проверен".into();
                    }
                    Err(error) => {
                        self.error = Some(error);
                    }
                }
                self.import_job = None;
            }
        }
        self.toolbar(ui, t);
        self.status_bar(ui, t);
        if self.library || self.navigator {
            self.navigation(ui, t);
        }
        if self.library {
            self.library(ui, t);
        } else {
            if self.inspector_visible {
                self.inspector(ui, t);
            }
            self.editor(ui, t);
        }
        self.dialogs(&ctx, t);
        self.frames += 1;
        if self.capture.is_some() && self.frames == 8 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshots = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        });
        if !screenshots.is_empty()
            && let Some(path) = self.capture.take()
        {
            if let Some(image) = screenshots.first() {
                let result = image::save_buffer(
                    &path,
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .map_err(Into::into);
                self.record_error(result);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.capture.is_some() {
            ctx.request_repaint();
        }
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Source Sans".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceSans3-Regular.ttf"
        ))
        .into(),
    );
    fonts.font_data.insert(
        "Source Serif".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceSerif4-Regular.ttf"
        ))
        .into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Source Sans".into());
    fonts.families.insert(
        FontFamily::Name("Manuscript".into()),
        vec!["Source Serif".into(), "Source Sans".into()],
    );
    ctx.set_fonts(fonts);
}
fn primary(ui: &mut egui::Ui, label: &str, t: Tokens) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(t.on_accent).strong())
            .fill(t.accent)
            .min_size(Vec2::new(128.0, 34.0)),
    )
}
fn nav_row(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    t: Tokens,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, if selected { t.accent_soft } else { t.surface });
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(10.0, 3.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    icons::icon(
        &mut child,
        icon,
        17.0,
        if selected { t.accent } else { t.muted },
    );
    child.add_space(4.0);
    child.add(
        egui::Label::new(RichText::new(label).size(14.0).color(if selected {
            t.text
        } else {
            t.muted
        }))
        .truncate(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response.on_hover_text(label)
}
fn diff_block(ui: &mut egui::Ui, label: &str, body: &str, background: Color32, t: Tokens) {
    ui.label(RichText::new(label).size(11.0).color(t.muted));
    ui.add_space(4.0);
    egui::Frame::new()
        .fill(background)
        .corner_radius(5)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(body).size(13.0));
        });
}
fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or("Не удалось определить папку файла")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

const SAMPLE: &str = "В тот день море было почти белым. Ася остановилась у калитки, положила ладонь на выцветшую доску и прислушалась: дом молчал так, будто ждал её все эти годы.\n\nОна не собиралась возвращаться. В её жизни всё было устроено — небольшая мастерская, стол у окна, знакомые лица по дороге на работу. Но в понедельник пришло письмо. Без обратного адреса, с фотографией маяка внутри.\n\nТеперь фотография лежала в кармане пальто, а сам маяк стоял за домом, высокий и тёмный на фоне облаков. В детстве ей казалось, что свет в его окне не гаснет никогда.\n\n— Ты всё-таки приехала.\n\nАся обернулась. У соседнего забора стоял человек с жестяной кружкой в руке. Она узнала его не сразу: седина, тяжёлый свитер, та же привычка смотреть чуть в сторону.\n\n— Здравствуй, Миша.\n\nОн кивнул, словно они расстались вчера, и отворил калитку. Петли скрипнули. Этот звук оказался удивительно точным — ни годы, ни солёный ветер не изменили в нём ничего.\n\nНа кухне пахло деревом и чаем. На столе стояли две чашки. Ася сняла пальто, повесила его на спинку стула и только тогда заметила: часы над дверью идут.\n\nОна хорошо помнила, как остановила их перед отъездом.\n\n— Кто здесь был? — спросила она.\n\nМиша поставил кружку на подоконник. За окном ветер медленно поворачивал сухую ветку яблони.\n\n— Сначала выпьем чаю, — сказал он. — У нас много времени.";
