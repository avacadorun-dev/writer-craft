//! Complete interface catalogs and appearance-independent language preferences.
//!
//! Catalog lookup never translates manuscript text, project titles or imported data.

use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

const MISSING_MESSAGE: &str = "⟦missing translation⟧";

/// Interface languages, kept in the same order as every translation row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Language {
    /// Russian.
    #[default]
    #[serde(rename = "ru")]
    Russian,
    /// English.
    #[serde(rename = "en")]
    English,
    /// German.
    #[serde(rename = "de")]
    German,
    /// French.
    #[serde(rename = "fr")]
    French,
    /// Spanish.
    #[serde(rename = "es")]
    Spanish,
    /// Portuguese.
    #[serde(rename = "pt")]
    Portuguese,
    /// Simplified Chinese.
    #[serde(rename = "zh")]
    Chinese,
    /// Japanese.
    #[serde(rename = "ja")]
    Japanese,
}

impl Language {
    /// Every supported language, in catalog column order.
    pub const ALL: [Self; 8] = [
        Self::Russian,
        Self::English,
        Self::German,
        Self::French,
        Self::Spanish,
        Self::Portuguese,
        Self::Chinese,
        Self::Japanese,
    ];

    /// Stable persisted language tag.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Russian => "ru",
            Self::English => "en",
            Self::German => "de",
            Self::French => "fr",
            Self::Spanish => "es",
            Self::Portuguese => "pt",
            Self::Chinese => "zh",
            Self::Japanese => "ja",
        }
    }

    /// A language's own name, so its setting remains discoverable in every interface.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::Russian => "Русский",
            Self::English => "English",
            Self::German => "Deutsch",
            Self::French => "Français",
            Self::Spanish => "Español",
            Self::Portuguese => "Português",
            Self::Chinese => "简体中文",
            Self::Japanese => "日本語",
        }
    }

    /// Resolve a stable message ID without falling back to Russian.
    pub fn text(self, msgid: &str) -> &'static str {
        CATALOG
            .iter()
            .find(|(key, _)| *key == msgid)
            .map(|(_, translations)| translations[self as usize])
            .unwrap_or(MISSING_MESSAGE)
    }

    /// Interpolate named placeholders once, leaving user-provided text untouched.
    /// Unknown parameters remain visible as `{name}` instead of disappearing.
    pub fn format(self, msgid: &str, parameters: &[(&str, String)]) -> String {
        let mut remaining = self.text(msgid);
        let mut output = String::with_capacity(remaining.len());
        while let Some(start) = remaining.find('{') {
            output.push_str(&remaining[..start]);
            let after = &remaining[start + 1..];
            let Some(end) = after.find('}') else {
                output.push_str(&remaining[start..]);
                return output;
            };
            let key = &after[..end];
            if let Some((_, value)) = parameters.iter().find(|(name, _)| *name == key) {
                output.push_str(value);
            } else {
                output.push_str(&remaining[start..start + end + 2]);
            }
            remaining = &after[end + 1..];
        }
        output.push_str(remaining);
        output
    }
}

/// Preferences stored separately from the existing appearance/palette file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfacePreferences {
    /// Language of interface labels; never a manuscript translation preference.
    pub language: Language,
    /// Manuscript font size in logical pixels.
    pub font_size: f32,
}

impl Default for InterfacePreferences {
    fn default() -> Self {
        Self {
            language: Language::Russian,
            font_size: 21.0,
        }
    }
}

impl InterfacePreferences {
    /// Load preferences, treating a missing file as a first launch.
    ///
    /// # Errors
    /// Returns an I/O error for unreadable or invalid JSON preferences, preserving that file.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let preferences: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(preferences.sanitized())
    }

    /// Atomically persist language and font size without touching appearance/palette settings.
    ///
    /// # Errors
    /// Returns an I/O error if staging, flushing or replacing the preferences file fails.
    pub fn save(self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec_pretty(&self.sanitized()).map_err(io::Error::other)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|error| error.error)?;
        Ok(())
    }

    fn sanitized(mut self) -> Self {
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(16.0, 28.0)
        } else {
            21.0
        };
        self
    }
}

// Each row is msgid + Russian, English, German, French, Spanish, Portuguese, Chinese, Japanese.
const CATALOG: &[(&str, [&str; 8])] = &[
    (
        "Rich-text proposals are not available in this preview. Your document remains unchanged.",
        [
            "Работа с предложениями правок ещё не подключена к новому редактору в этой тестовой сборке. Документ остаётся без изменений.",
            "Rich-text proposals are not available in this preview. Your document remains unchanged.",
            "Änderungsvorschläge für formatierten Text sind in dieser Vorschau noch nicht verfügbar. Ihr Dokument bleibt unverändert.",
            "Les propositions de modification du texte enrichi ne sont pas disponibles dans cet aperçu. Votre document reste inchangé.",
            "Las propuestas para texto enriquecido aún no están disponibles en esta vista previa. Su documento no cambia.",
            "As propostas para texto formatado ainda não estão disponíveis nesta prévia. Seu documento permanece inalterado.",
            "此预览版尚未接入富文本修改建议。文档保持不变。",
            "このプレビューではリッチテキストの変更提案をまだ利用できません。文書は変更されません。",
        ],
    ),
    (
        "Mixed formatting",
        [
            "Смешанное форматирование",
            "Mixed formatting",
            "Gemischte Formatierung",
            "Mise en forme mixte",
            "Formato mixto",
            "Formatação mista",
            "混合格式",
            "書式が混在",
        ],
    ),
    (
        "More formatting",
        [
            "Другие параметры текста",
            "More formatting",
            "Weitere Formatierung",
            "Autres options de texte",
            "Más opciones de formato",
            "Mais opções de formatação",
            "更多格式",
            "その他の書式",
        ],
    ),
    (
        "Editor Preview",
        [
            "Предпросмотр редактора",
            "Editor Preview",
            "Editor-Vorschau",
            "Aperçu de l’éditeur",
            "Vista previa del editor",
            "Prévia do editor",
            "编辑器预览",
            "エディタープレビュー",
        ],
    ),
    (
        "Paragraph",
        [
            "Обычный текст",
            "Paragraph",
            "Absatz",
            "Paragraphe",
            "Párrafo",
            "Parágrafo",
            "正文",
            "本文",
        ],
    ),
    (
        "Heading",
        [
            "Заголовок",
            "Heading",
            "Überschrift",
            "Titre",
            "Título",
            "Título",
            "标题",
            "見出し",
        ],
    ),
    (
        "Heading 1",
        [
            "Заголовок 1",
            "Heading 1",
            "Überschrift 1",
            "Titre 1",
            "Título 1",
            "Título 1",
            "标题 1",
            "見出し 1",
        ],
    ),
    (
        "Heading 2",
        [
            "Заголовок 2",
            "Heading 2",
            "Überschrift 2",
            "Titre 2",
            "Título 2",
            "Título 2",
            "标题 2",
            "見出し 2",
        ],
    ),
    (
        "Heading 3",
        [
            "Заголовок 3",
            "Heading 3",
            "Überschrift 3",
            "Titre 3",
            "Título 3",
            "Título 3",
            "标题 3",
            "見出し 3",
        ],
    ),
    (
        "Bold",
        [
            "Жирный · ⌘B",
            "Bold · ⌘B",
            "Fett · ⌘B",
            "Gras · ⌘B",
            "Negrita · ⌘B",
            "Negrito · ⌘B",
            "粗体 · ⌘B",
            "太字 · ⌘B",
        ],
    ),
    (
        "Italic",
        [
            "Курсив · ⌘I",
            "Italic · ⌘I",
            "Kursiv · ⌘I",
            "Italique · ⌘I",
            "Cursiva · ⌘I",
            "Itálico · ⌘I",
            "斜体 · ⌘I",
            "斜体 · ⌘I",
        ],
    ),
    (
        "Underline",
        [
            "Подчёркнутый · ⌘U",
            "Underline · ⌘U",
            "Unterstrichen · ⌘U",
            "Souligné · ⌘U",
            "Subrayado · ⌘U",
            "Sublinhado · ⌘U",
            "下划线 · ⌘U",
            "下線 · ⌘U",
        ],
    ),
    (
        "Table",
        [
            "Таблица",
            "Table",
            "Tabelle",
            "Tableau",
            "Tabla",
            "Tabela",
            "表格",
            "表",
        ],
    ),
    (
        "Insert table · 3 × 3",
        [
            "Вставить таблицу · 3 × 3",
            "Insert table · 3 × 3",
            "Tabelle einfügen · 3 × 3",
            "Insérer un tableau · 3 × 3",
            "Insertar tabla · 3 × 3",
            "Inserir tabela · 3 × 3",
            "插入表格 · 3 × 3",
            "表を挿入 · 3 × 3",
        ],
    ),
    (
        "Add row below",
        [
            "Добавить строку снизу",
            "Add row below",
            "Zeile darunter einfügen",
            "Ajouter une ligne en dessous",
            "Añadir fila debajo",
            "Adicionar linha abaixo",
            "在下方插入行",
            "下に行を追加",
        ],
    ),
    (
        "Add column right",
        [
            "Добавить столбец справа",
            "Add column right",
            "Spalte rechts einfügen",
            "Ajouter une colonne à droite",
            "Añadir columna a la derecha",
            "Adicionar coluna à direita",
            "在右侧插入列",
            "右に列を追加",
        ],
    ),
    (
        "Delete row",
        [
            "Удалить строку",
            "Delete row",
            "Zeile löschen",
            "Supprimer la ligne",
            "Eliminar fila",
            "Excluir linha",
            "删除行",
            "行を削除",
        ],
    ),
    (
        "Delete column",
        [
            "Удалить столбец",
            "Delete column",
            "Spalte löschen",
            "Supprimer la colonne",
            "Eliminar columna",
            "Excluir coluna",
            "删除列",
            "列を削除",
        ],
    ),
    (
        "Merge selected cells",
        [
            "Объединить выбранные ячейки",
            "Merge selected cells",
            "Ausgewählte Zellen verbinden",
            "Fusionner les cellules sélectionnées",
            "Combinar celdas seleccionadas",
            "Mesclar células selecionadas",
            "合并选定单元格",
            "選択したセルを結合",
        ],
    ),
    (
        "Split cell",
        [
            "Разделить ячейку",
            "Split cell",
            "Zelle teilen",
            "Fractionner la cellule",
            "Dividir celda",
            "Dividir célula",
            "拆分单元格",
            "セルを分割",
        ],
    ),
    (
        "Undo",
        [
            "Отменить · ⌘Z",
            "Undo · ⌘Z",
            "Rückgängig · ⌘Z",
            "Annuler · ⌘Z",
            "Deshacer · ⌘Z",
            "Desfazer · ⌘Z",
            "撤销 · ⌘Z",
            "元に戻す · ⌘Z",
        ],
    ),
    (
        "Redo",
        [
            "Повторить · ⌘⇧Z",
            "Redo · ⌘⇧Z",
            "Wiederholen · ⌘⇧Z",
            "Rétablir · ⌘⇧Z",
            "Rehacer · ⌘⇧Z",
            "Refazer · ⌘⇧Z",
            "重做 · ⌘⇧Z",
            "やり直す · ⌘⇧Z",
        ],
    ),
    (
        "Strikethrough",
        [
            "Зачёркнутый",
            "Strikethrough",
            "Durchgestrichen",
            "Barré",
            "Tachado",
            "Tachado",
            "删除线",
            "取り消し線",
        ],
    ),
    (
        "Inline code",
        [
            "Код в строке",
            "Inline code",
            "Inline-Code",
            "Code en ligne",
            "Código en línea",
            "Código em linha",
            "行内代码",
            "インラインコード",
        ],
    ),
    (
        "The working draft changed. Preserve your changes before retrying.",
        [
            "Рабочий черновик изменился. Сохраните свою работу отдельно перед повторной попыткой.",
            "The working draft changed. Preserve your changes before retrying.",
            "Der Arbeitsentwurf wurde geändert. Sichern Sie Ihre Änderungen vor einem erneuten Versuch.",
            "Le brouillon a changé. Conservez vos modifications avant de réessayer.",
            "El borrador ha cambiado. Conserve sus cambios antes de volver a intentarlo.",
            "O rascunho foi alterado. Preserve suas alterações antes de tentar novamente.",
            "工作草稿已更改。请先保留您的修改，再重试。",
            "作業中の下書きが変更されました。変更内容を保全してから再試行してください。",
        ],
    ),
    (
        "Close",
        [
            "Закрыть",
            "Close",
            "Schließen",
            "Fermer",
            "Cerrar",
            "Fechar",
            "关闭",
            "閉じる",
        ],
    ),
    (
        "Workspace",
        [
            "Рабочая среда",
            "Workspace",
            "Arbeitsbereich",
            "Espace de travail",
            "Espacio de trabajo",
            "Área de trabalho",
            "工作区",
            "ワークスペース",
        ],
    ),
    (
        "Connections",
        [
            "Подключения",
            "Connections",
            "Verbindungen",
            "Connexions",
            "Conexiones",
            "Conexões",
            "连接",
            "接続",
        ],
    ),
    (
        "Application",
        [
            "Приложение",
            "Application",
            "Anwendung",
            "Application",
            "Aplicación",
            "Aplicativo",
            "应用",
            "アプリ",
        ],
    ),
    (
        "General",
        [
            "Общие",
            "General",
            "Allgemein",
            "Général",
            "General",
            "Geral",
            "常规",
            "一般",
        ],
    ),
    (
        "Editor",
        [
            "Редактор",
            "Editor",
            "Editor",
            "Éditeur",
            "Editor",
            "Editor",
            "编辑器",
            "エディター",
        ],
    ),
    (
        "AI & providers",
        [
            "ИИ и провайдеры",
            "AI & providers",
            "KI & Anbieter",
            "IA et fournisseurs",
            "IA y proveedores",
            "IA e provedores",
            "AI 与服务商",
            "AI とプロバイダー",
        ],
    ),
    (
        "Agents & MCP",
        [
            "Агенты и MCP",
            "Agents & MCP",
            "Agenten & MCP",
            "Agents et MCP",
            "Agentes y MCP",
            "Agentes e MCP",
            "智能体与 MCP",
            "エージェントと MCP",
        ],
    ),
    (
        "Voice",
        [
            "Голос",
            "Voice",
            "Spracheingabe",
            "Voix",
            "Voz",
            "Voz",
            "语音",
            "音声",
        ],
    ),
    (
        "Files & recovery",
        [
            "Файлы и восстановление",
            "Files & recovery",
            "Dateien & Wiederherstellung",
            "Fichiers et restauration",
            "Archivos y recuperación",
            "Arquivos e recuperação",
            "文件与恢复",
            "ファイルと復元",
        ],
    ),
    (
        "This section is not available in this alpha.",
        [
            "Этот раздел пока недоступен в альфа-версии.",
            "This section is not available in this alpha.",
            "Dieser Bereich ist in dieser Alpha noch nicht verfügbar.",
            "Cette section est indisponible dans cette version alpha.",
            "Esta sección aún no está disponible en esta alfa.",
            "Esta seção ainda não está disponível nesta versão alfa.",
            "此部分在当前 Alpha 版本中尚不可用。",
            "このセクションは、このアルファ版ではまだ利用できません。",
        ],
    ),
    (
        "Provider connections, models and assistant behavior.",
        [
            "Подключения провайдеров, модели и поведение ассистента.",
            "Provider connections, models and assistant behavior.",
            "Anbieter-Verbindungen, Modelle und Assistentenverhalten.",
            "Connexions aux fournisseurs, modèles et comportement de l’assistant.",
            "Conexiones de proveedores, modelos y comportamiento del asistente.",
            "Conexões de provedores, modelos e comportamento do assistente.",
            "服务商连接、模型和助手行为。",
            "プロバイダー接続、モデル、アシスタントの動作。",
        ],
    ),
    (
        "Agent connections and MCP server settings.",
        [
            "Подключения агентов и настройки MCP-серверов.",
            "Agent connections and MCP server settings.",
            "Agenten-Verbindungen und MCP-Server-Einstellungen.",
            "Connexions aux agents et paramètres des serveurs MCP.",
            "Conexiones de agentes y ajustes de servidores MCP.",
            "Conexões de agentes e configurações de servidores MCP.",
            "智能体连接和 MCP 服务器设置。",
            "エージェント接続と MCP サーバーの設定。",
        ],
    ),
    (
        "Microphone, transcription language and voice commands.",
        [
            "Микрофон, язык распознавания и голосовые команды.",
            "Microphone, transcription language and voice commands.",
            "Mikrofon, Transkriptionssprache und Sprachbefehle.",
            "Microphone, langue de transcription et commandes vocales.",
            "Micrófono, idioma de transcripción y comandos de voz.",
            "Microfone, idioma de transcrição e comandos de voz.",
            "麦克风、转写语言和语音命令。",
            "マイク、文字起こしの言語、音声コマンド。",
        ],
    ),
    (
        "Project storage, backups, recovery and audio retention.",
        [
            "Хранение проектов, резервные копии, восстановление и хранение аудио.",
            "Project storage, backups, recovery and audio retention.",
            "Projektspeicher, Sicherungen, Wiederherstellung und Audioaufbewahrung.",
            "Stockage des projets, sauvegardes, restauration et conservation audio.",
            "Almacenamiento de proyectos, copias, recuperación y conservación de audio.",
            "Armazenamento de projetos, backups, recuperação e retenção de áudio.",
            "项目存储、备份、恢复和音频保留。",
            "プロジェクトの保存、バックアップ、復元、音声の保持。",
        ],
    ),
    (
        "Exit focus",
        [
            "Вернуться к работе",
            "Exit focus",
            "Fokus beenden",
            "Quitter le mode concentration",
            "Salir del enfoque",
            "Sair do foco",
            "退出专注模式",
            "集中モードを終了",
        ],
    ),
    (
        "Outdated proposal — the text has changed",
        [
            "Предложение устарело — текст изменён",
            "Outdated proposal — the text has changed",
            "Veralteter Vorschlag — der Text wurde geändert",
            "Proposition obsolète — le texte a changé",
            "Propuesta desactualizada: el texto ha cambiado",
            "Proposta desatualizada: o texto mudou",
            "建议已过期：文本已更改",
            "提案は古くなっています：本文が変更されました",
        ],
    ),
    (
        "Review all changes",
        [
            "Полный просмотр изменений",
            "Review all changes",
            "Alle Änderungen prüfen",
            "Examiner toutes les modifications",
            "Revisar todos los cambios",
            "Revisar todas as alterações",
            "查看全部更改",
            "すべての変更を確認",
        ],
    ),
    (
        "Copy proposal",
        [
            "Скопировать предложение",
            "Copy proposal",
            "Vorschlag kopieren",
            "Copier la proposition",
            "Copiar propuesta",
            "Copiar proposta",
            "复制建议",
            "提案をコピー",
        ],
    ),
    (
        "Clear selection scope",
        [
            "Очистить выделение",
            "Clear selection scope",
            "Auswahlbereich löschen",
            "Effacer la portée de sélection",
            "Borrar alcance de selección",
            "Limpar escopo da seleção",
            "清除选区范围",
            "選択範囲を解除",
        ],
    ),
    (
        "Selection: {start}–{end}",
        [
            "Выделение: {start}–{end}",
            "Selection: {start}–{end}",
            "Auswahl: {start}–{end}",
            "Sélection : {start}–{end}",
            "Selección: {start}–{end}",
            "Seleção: {start}–{end}",
            "选区：{start}–{end}",
            "選択範囲：{start}–{end}",
        ],
    ),
    (
        "All changes saved",
        [
            "Все изменения сохранены",
            "All changes saved",
            "Alle Änderungen gespeichert",
            "Toutes les modifications sont enregistrées",
            "Todos los cambios guardados",
            "Todas as alterações salvas",
            "所有更改已保存",
            "すべての変更を保存しました",
        ],
    ),
    (
        "Unsaved text recovered",
        [
            "Восстановлен несохранённый текст",
            "Unsaved text recovered",
            "Ungespeicherter Text wiederhergestellt",
            "Texte non enregistré récupéré",
            "Texto sin guardar recuperado",
            "Texto não salvo recuperado",
            "已恢复未保存的文本",
            "未保存のテキストを復元しました",
        ],
    ),
    (
        "Unsaved text recovered as a separate document",
        [
            "Несохранённый текст восстановлен отдельным документом",
            "Unsaved text recovered as a separate document",
            "Ungespeicherter Text als separates Dokument wiederhergestellt",
            "Texte non enregistré récupéré dans un document distinct",
            "Texto sin guardar recuperado en un documento aparte",
            "Texto não salvo recuperado em um documento separado",
            "未保存的文本已恢复为单独的文档",
            "未保存のテキストを別の文書として復元しました",
        ],
    ),
    (
        "Attention needed",
        [
            "Требуется внимание",
            "Attention needed",
            "Aktion erforderlich",
            "Une intervention est nécessaire",
            "Se requiere atención",
            "Atenção necessária",
            "需要处理",
            "確認が必要です",
        ],
    ),
    (
        "Project library",
        [
            "Библиотека проектов",
            "Project library",
            "Projektbibliothek",
            "Bibliothèque de projets",
            "Biblioteca de proyectos",
            "Biblioteca de projetos",
            "项目库",
            "プロジェクトライブラリ",
        ],
    ),
    (
        "File",
        [
            "Файл",
            "File",
            "Datei",
            "Fichier",
            "Archivo",
            "Arquivo",
            "文件",
            "ファイル",
        ],
    ),
    (
        "New project…     ⌘N",
        [
            "Новый проект…     ⌘N",
            "New project…     ⌘N",
            "Neues Projekt…     ⌘N",
            "Nouveau projet…     ⌘N",
            "Nuevo proyecto…     ⌘N",
            "Novo projeto…     ⌘N",
            "新建项目…     ⌘N",
            "新規プロジェクト…     ⌘N",
        ],
    ),
    (
        "Open .wstudio…     ⌘O",
        [
            "Открыть .wstudio…     ⌘O",
            "Open .wstudio…     ⌘O",
            ".wstudio öffnen…     ⌘O",
            "Ouvrir un .wstudio…     ⌘O",
            "Abrir .wstudio…     ⌘O",
            "Abrir .wstudio…     ⌘O",
            "打开 .wstudio…     ⌘O",
            ".wstudio を開く…     ⌘O",
        ],
    ),
    (
        "Export text…",
        [
            "Экспорт текста…",
            "Export text…",
            "Text exportieren…",
            "Exporter le texte…",
            "Exportar texto…",
            "Exportar texto…",
            "导出文本…",
            "テキストを書き出す…",
        ],
    ),
    (
        "Save · ⌘S",
        [
            "Сохранить · ⌘S",
            "Save · ⌘S",
            "Speichern · ⌘S",
            "Enregistrer · ⌘S",
            "Guardar · ⌘S",
            "Salvar · ⌘S",
            "保存 · ⌘S",
            "保存 · ⌘S",
        ],
    ),
    (
        "Settings · ⌘,",
        [
            "Настройки · ⌘,",
            "Settings · ⌘,",
            "Einstellungen · ⌘,",
            "Paramètres · ⌘,",
            "Ajustes · ⌘,",
            "Configurações · ⌘,",
            "设置 · ⌘,",
            "設定 · ⌘,",
        ],
    ),
    (
        "Review panel · ⌘⇧A",
        [
            "Панель правок · ⌘⇧A",
            "Review panel · ⌘⇧A",
            "Überarbeitung · ⌘⇧A",
            "Panneau de révision · ⌘⇧A",
            "Panel de revisión · ⌘⇧A",
            "Painel de revisão · ⌘⇧A",
            "审阅面板 · ⌘⇧A",
            "校正パネル · ⌘⇧A",
        ],
    ),
    (
        "Project structure",
        [
            "Структура проекта",
            "Project structure",
            "Projektstruktur",
            "Structure du projet",
            "Estructura del proyecto",
            "Estrutura do projeto",
            "项目结构",
            "プロジェクト構成",
        ],
    ),
    (
        "On this computer",
        [
            "На этом компьютере",
            "On this computer",
            "Auf diesem Computer",
            "Sur cet ordinateur",
            "En este equipo",
            "Neste computador",
            "在此电脑上",
            "このコンピュータ上",
        ],
    ),
    (
        "Words: {count}   ·   Revision {revision}",
        [
            "Слов: {count}   ·   Версия {revision}",
            "Words: {count}   ·   Revision {revision}",
            "Wörter: {count}   ·   Version {revision}",
            "Mots : {count}   ·   Version {revision}",
            "Palabras: {count}   ·   Versión {revision}",
            "Palavras: {count}   ·   Versão {revision}",
            "字词数：{count}   ·   版本 {revision}",
            "単語数：{count}   ·   版 {revision}",
        ],
    ),
    (
        "LIBRARY",
        [
            "БИБЛИОТЕКА",
            "LIBRARY",
            "BIBLIOTHEK",
            "BIBLIOTHÈQUE",
            "BIBLIOTECA",
            "BIBLIOTECA",
            "项目库",
            "ライブラリ",
        ],
    ),
    (
        "STRUCTURE",
        [
            "СТРУКТУРА",
            "STRUCTURE",
            "STRUKTUR",
            "STRUCTURE",
            "ESTRUCTURA",
            "ESTRUTURA",
            "结构",
            "構成",
        ],
    ),
    (
        "Create project",
        [
            "Создать проект",
            "Create project",
            "Projekt erstellen",
            "Créer un projet",
            "Crear proyecto",
            "Criar projeto",
            "创建项目",
            "プロジェクトを作成",
        ],
    ),
    (
        "Add document",
        [
            "Добавить документ",
            "Add document",
            "Dokument hinzufügen",
            "Ajouter un document",
            "Añadir documento",
            "Adicionar documento",
            "添加文档",
            "文書を追加",
        ],
    ),
    (
        "All projects",
        [
            "Все проекты",
            "All projects",
            "Alle Projekte",
            "Tous les projets",
            "Todos los proyectos",
            "Todos os projetos",
            "所有项目",
            "すべてのプロジェクト",
        ],
    ),
    (
        "Open .wstudio",
        [
            "Открыть .wstudio",
            "Open .wstudio",
            ".wstudio öffnen",
            "Ouvrir un .wstudio",
            "Abrir .wstudio",
            "Abrir .wstudio",
            "打开 .wstudio",
            ".wstudio を開く",
        ],
    ),
    (
        "WORKSPACE",
        [
            "РАБОЧЕЕ ПРОСТРАНСТВО",
            "WORKSPACE",
            "ARBEITSBEREICH",
            "ESPACE DE TRAVAIL",
            "ESPACIO DE TRABAJO",
            "ESPAÇO DE TRABALHO",
            "工作区",
            "ワークスペース",
        ],
    ),
    (
        "Your manuscript, materials and history, together in your project.",
        [
            "Рукопись, материалы и история — вместе с вашим проектом.",
            "Your manuscript, materials and history, together in your project.",
            "Manuskript, Materialien und Verlauf – gemeinsam in Ihrem Projekt.",
            "Votre manuscrit, vos ressources et leur historique, réunis dans votre projet.",
            "Tu manuscrito, materiales e historial, juntos en tu proyecto.",
            "Seu manuscrito, materiais e histórico, juntos no seu projeto.",
            "手稿、素材和历史记录，尽在项目之中。",
            "原稿、資料、履歴をプロジェクトにまとめて管理。",
        ],
    ),
    (
        "Documents: {count}",
        [
            "Документов: {count}",
            "Documents: {count}",
            "Dokumente: {count}",
            "Documents : {count}",
            "Documentos: {count}",
            "Documentos: {count}",
            "文档数：{count}",
            "文書数：{count}",
        ],
    ),
    (
        "Find a document…",
        [
            "Найти документ…",
            "Find a document…",
            "Dokument suchen…",
            "Rechercher un document…",
            "Buscar documento…",
            "Buscar documento…",
            "查找文档…",
            "文書を検索…",
        ],
    ),
    (
        "Manuscript",
        [
            "Рукопись",
            "Manuscript",
            "Manuskript",
            "Manuscrit",
            "Manuscrito",
            "Manuscrito",
            "手稿",
            "原稿",
        ],
    ),
    (
        "Document history",
        [
            "История документа",
            "Document history",
            "Dokumentverlauf",
            "Historique du document",
            "Historial del documento",
            "Histórico do documento",
            "文档历史",
            "文書の履歴",
        ],
    ),
    (
        "A place for your great stories",
        [
            "Ваше место для больших историй",
            "A place for your great stories",
            "Ein Ort für Ihre großen Geschichten",
            "Un lieu pour vos grandes histoires",
            "Un lugar para tus grandes historias",
            "Um lugar para suas grandes histórias",
            "为你的精彩故事留一片天地",
            "大きな物語を紡ぐ場所",
        ],
    ),
    (
        "Open a manuscript and continue where you left off.",
        [
            "Откройте рукопись и продолжите с того места, где остановились.",
            "Open a manuscript and continue where you left off.",
            "Öffnen Sie ein Manuskript und machen Sie dort weiter, wo Sie aufgehört haben.",
            "Ouvrez un manuscrit et reprenez là où vous vous étiez arrêté.",
            "Abre un manuscrito y continúa donde lo dejaste.",
            "Abra um manuscrito e continue de onde parou.",
            "打开手稿，从上次停下的地方继续。",
            "原稿を開いて、前回の続きから書き始めましょう。",
        ],
    ),
    (
        "New project",
        [
            "Новый проект",
            "New project",
            "Neues Projekt",
            "Nouveau projet",
            "Nuevo proyecto",
            "Novo projeto",
            "新建项目",
            "新規プロジェクト",
        ],
    ),
    (
        "Open .wstudio…",
        [
            "Открыть .wstudio…",
            "Open .wstudio…",
            ".wstudio öffnen…",
            "Ouvrir un .wstudio…",
            "Abrir .wstudio…",
            "Abrir .wstudio…",
            "打开 .wstudio…",
            ".wstudio を開く…",
        ],
    ),
    (
        "Try a sample",
        [
            "Попробовать на примере",
            "Try a sample",
            "Beispiel ausprobieren",
            "Essayer un exemple",
            "Probar un ejemplo",
            "Experimentar um exemplo",
            "试用示例",
            "サンプルを試す",
        ],
    ),
    (
        "PROJECTS",
        [
            "ПРОЕКТЫ",
            "PROJECTS",
            "PROJEKTE",
            "PROJETS",
            "PROYECTOS",
            "PROJETOS",
            "项目",
            "プロジェクト",
        ],
    ),
    (
        "Open project",
        [
            "Открыть проект",
            "Open project",
            "Projekt öffnen",
            "Ouvrir le projet",
            "Abrir proyecto",
            "Abrir projeto",
            "打开项目",
            "プロジェクトを開く",
        ],
    ),
    (
        "Local project",
        [
            "Локальный проект",
            "Local project",
            "Lokales Projekt",
            "Projet local",
            "Proyecto local",
            "Projeto local",
            "本地项目",
            "ローカルプロジェクト",
        ],
    ),
    (
        "Your first chapter starts here",
        [
            "Первая глава начинается здесь",
            "Your first chapter starts here",
            "Ihr erstes Kapitel beginnt hier",
            "Votre premier chapitre commence ici",
            "Tu primer capítulo empieza aquí",
            "Seu primeiro capítulo começa aqui",
            "第一章，从这里开始",
            "最初の章はここから",
        ],
    ),
    (
        "Create a project. Writing needs no account, server or internet connection.",
        [
            "Создайте проект. Для письма не нужны аккаунт, сервер или подключение к сети.",
            "Create a project. Writing needs no account, server or internet connection.",
            "Erstellen Sie ein Projekt. Zum Schreiben brauchen Sie weder Konto noch Server oder Internetverbindung.",
            "Créez un projet. Écrire ne nécessite ni compte, ni serveur, ni connexion Internet.",
            "Crea un proyecto. Para escribir no necesitas una cuenta, un servidor ni conexión a internet.",
            "Crie um projeto. Para escrever, não é preciso conta, servidor ou conexão com a internet.",
            "创建项目即可写作，无需账号、服务器或网络连接。",
            "プロジェクトを作成しましょう。執筆にアカウント、サーバー、インターネット接続は必要ありません。",
        ],
    ),
    (
        "Add your first document",
        [
            "Добавить первый документ",
            "Add your first document",
            "Erstes Dokument hinzufügen",
            "Ajouter votre premier document",
            "Añadir tu primer documento",
            "Adicionar seu primeiro documento",
            "添加第一份文档",
            "最初の文書を追加",
        ],
    ),
    (
        "MANUSCRIPT",
        [
            "РУКОПИСЬ",
            "MANUSCRIPT",
            "MANUSKRIPT",
            "MANUSCRIT",
            "MANUSCRITO",
            "MANUSCRITO",
            "手稿",
            "原稿",
        ],
    ),
    (
        "Original format · read only",
        [
            "Исходный формат · только чтение",
            "Original format · read only",
            "Originalformat · schreibgeschützt",
            "Format d’origine · lecture seule",
            "Formato original · solo lectura",
            "Formato original · somente leitura",
            "原始格式 · 只读",
            "元の形式 · 読み取り専用",
        ],
    ),
    (
        "Export text",
        [
            "Экспортировать текст",
            "Export text",
            "Text exportieren",
            "Exporter le texte",
            "Exportar texto",
            "Exportar texto",
            "导出文本",
            "テキストを書き出す",
        ],
    ),
    (
        "Focus",
        [
            "Фокус",
            "Focus",
            "Fokus",
            "Concentration",
            "Concentración",
            "Foco",
            "专注",
            "集中モード",
        ],
    ),
    (
        "Hide side panels · ⌘⇧F",
        [
            "Скрыть боковые панели · ⌘⇧F",
            "Hide side panels · ⌘⇧F",
            "Seitenleisten ausblenden · ⌘⇧F",
            "Masquer les panneaux latéraux · ⌘⇧F",
            "Ocultar paneles laterales · ⌘⇧F",
            "Ocultar painéis laterais · ⌘⇧F",
            "隐藏侧边面板 · ⌘⇧F",
            "サイドパネルを隠す · ⌘⇧F",
        ],
    ),
    (
        "Start writing…",
        [
            "Начните писать…",
            "Start writing…",
            "Beginnen Sie zu schreiben…",
            "Commencez à écrire…",
            "Empieza a escribir…",
            "Comece a escrever…",
            "开始写作…",
            "ここから書き始める…",
        ],
    ),
    (
        "Saving…",
        [
            "Сохраняется…",
            "Saving…",
            "Wird gespeichert…",
            "Enregistrement…",
            "Guardando…",
            "Salvando…",
            "正在保存…",
            "保存中…",
        ],
    ),
    (
        "Review",
        [
            "Правки",
            "Review",
            "Überarbeitung",
            "Révision",
            "Revisión",
            "Revisão",
            "审阅",
            "校正",
        ],
    ),
    (
        "History",
        [
            "История",
            "History",
            "Verlauf",
            "Historique",
            "Historial",
            "Histórico",
            "历史",
            "履歴",
        ],
    ),
    (
        "Hide panel",
        [
            "Скрыть панель",
            "Hide panel",
            "Leiste ausblenden",
            "Masquer le panneau",
            "Ocultar panel",
            "Ocultar painel",
            "隐藏面板",
            "パネルを隠す",
        ],
    ),
    (
        "You are in control",
        [
            "Под вашим контролем",
            "You are in control",
            "Sie behalten die Kontrolle",
            "Vous gardez le contrôle",
            "Tú tienes el control",
            "Você está no controle",
            "由你掌控",
            "変更はあなたの判断で",
        ],
    ),
    (
        "Review the proposal before changing your manuscript. The original stays unchanged until you apply it.",
        [
            "Проверьте предложение перед изменением рукописи. Оригинал остаётся нетронутым до применения.",
            "Review the proposal before changing your manuscript. The original stays unchanged until you apply it.",
            "Prüfen Sie den Vorschlag, bevor Sie das Manuskript ändern. Das Original bleibt bis zur Anwendung unverändert.",
            "Vérifiez la proposition avant de modifier votre manuscrit. L’original reste inchangé jusqu’à son application.",
            "Revisa la propuesta antes de cambiar el manuscrito. El original no cambia hasta que la apliques.",
            "Revise a proposta antes de alterar o manuscrito. O original permanece intacto até a aplicação.",
            "更改手稿前请审阅建议。应用建议前，原文保持不变。",
            "原稿を変更する前に提案を確認してください。適用するまで原文は変更されません。",
        ],
    ),
    (
        "Prepare an exact edit",
        [
            "Подготовить точную правку",
            "Prepare an exact edit",
            "Exakte Änderung vorbereiten",
            "Préparer une modification précise",
            "Preparar una edición exacta",
            "Preparar uma edição exata",
            "准备精确修改",
            "正確な編集を準備",
        ],
    ),
    (
        "Original excerpt",
        [
            "Исходный фрагмент",
            "Original excerpt",
            "Originalauszug",
            "Extrait d’origine",
            "Fragmento original",
            "Trecho original",
            "原始片段",
            "元の抜粋",
        ],
    ),
    (
        "Select text in the manuscript or paste an exact excerpt",
        [
            "Выделите текст в рукописи или вставьте точный фрагмент",
            "Select text in the manuscript or paste an exact excerpt",
            "Text im Manuskript markieren oder einen exakten Auszug einfügen",
            "Sélectionnez du texte dans le manuscrit ou collez un extrait exact",
            "Selecciona texto en el manuscrito o pega un fragmento exacto",
            "Selecione texto no manuscrito ou cole um trecho exato",
            "在手稿中选择文本，或粘贴完全一致的片段",
            "原稿のテキストを選択するか、完全に一致する抜粋を貼り付けてください",
        ],
    ),
    (
        "Proposed text",
        [
            "Предлагаемый текст",
            "Proposed text",
            "Vorgeschlagener Text",
            "Texte proposé",
            "Texto propuesto",
            "Texto proposto",
            "建议文本",
            "提案テキスト",
        ],
    ),
    (
        "New wording",
        [
            "Новая формулировка",
            "New wording",
            "Neue Formulierung",
            "Nouvelle formulation",
            "Nueva redacción",
            "Nova redação",
            "新的表述",
            "新しい表現",
        ],
    ),
    (
        "Reason for the edit (optional)",
        [
            "Причина правки (необязательно)",
            "Reason for the edit (optional)",
            "Grund der Änderung (optional)",
            "Motif de la modification (facultatif)",
            "Motivo de la edición (opcional)",
            "Motivo da edição (opcional)",
            "修改原因（可选）",
            "編集理由（任意）",
        ],
    ),
    (
        "Prepare proposal",
        [
            "Подготовить предложение",
            "Prepare proposal",
            "Vorschlag vorbereiten",
            "Préparer la proposition",
            "Preparar propuesta",
            "Preparar proposta",
            "准备建议",
            "提案を準備",
        ],
    ),
    (
        "Local editing tool. No AI provider is connected yet.",
        [
            "Локальный инструмент правок. AI-провайдер пока не подключён.",
            "Local editing tool. No AI provider is connected yet.",
            "Lokales Überarbeitungswerkzeug. Es ist noch kein KI-Anbieter verbunden.",
            "Outil de révision local. Aucun fournisseur d’IA n’est encore connecté.",
            "Herramienta de edición local. Aún no hay un proveedor de IA conectado.",
            "Ferramenta de edição local. Nenhum provedor de IA está conectado ainda.",
            "本地编辑工具，尚未连接 AI 服务。",
            "ローカルの編集ツールです。AI プロバイダーはまだ接続されていません。",
        ],
    ),
    (
        "Proposed changes · not applied",
        [
            "Предлагаемые изменения · не применены",
            "Proposed changes · not applied",
            "Vorgeschlagene Änderungen · nicht angewendet",
            "Modifications proposées · non appliquées",
            "Cambios propuestos · sin aplicar",
            "Alterações propostas · não aplicadas",
            "建议的更改 · 尚未应用",
            "提案された変更 · 未適用",
        ],
    ),
    (
        "Original text",
        [
            "Исходный текст",
            "Original text",
            "Originaltext",
            "Texte d’origine",
            "Texto original",
            "Texto original",
            "原文",
            "元のテキスト",
        ],
    ),
    (
        "Proposal",
        [
            "Предложение",
            "Proposal",
            "Vorschlag",
            "Proposition",
            "Propuesta",
            "Proposta",
            "建议",
            "提案",
        ],
    ),
    (
        "Apply changes",
        [
            "Применить изменения",
            "Apply changes",
            "Änderungen anwenden",
            "Appliquer les modifications",
            "Aplicar cambios",
            "Aplicar alterações",
            "应用更改",
            "変更を適用",
        ],
    ),
    (
        "Reject",
        [
            "Отклонить",
            "Reject",
            "Ablehnen",
            "Rejeter",
            "Rechazar",
            "Rejeitar",
            "拒绝",
            "却下",
        ],
    ),
    (
        "Changes applied",
        [
            "Изменения применены",
            "Changes applied",
            "Änderungen angewendet",
            "Modifications appliquées",
            "Cambios aplicados",
            "Alterações aplicadas",
            "已应用更改",
            "変更を適用しました",
        ],
    ),
    (
        "Undo apply",
        [
            "Отменить применение",
            "Undo apply",
            "Anwendung rückgängig machen",
            "Annuler l’application des modifications",
            "Deshacer aplicación",
            "Desfazer aplicação",
            "撤销应用",
            "適用を取り消す",
        ],
    ),
    (
        "Proposal rejected",
        [
            "Предложение отклонено",
            "Proposal rejected",
            "Vorschlag abgelehnt",
            "Proposition rejetée",
            "Propuesta rechazada",
            "Proposta rejeitada",
            "已拒绝建议",
            "提案を却下しました",
        ],
    ),
    (
        "Apply undone",
        [
            "Применение отменено",
            "Apply undone",
            "Anwendung rückgängig gemacht",
            "Application des modifications annulée",
            "Aplicación deshecha",
            "Aplicação desfeita",
            "已撤销应用",
            "適用を取り消しました",
        ],
    ),
    (
        "Saved revisions remain in your local project.",
        [
            "Сохранённые версии остаются в локальном проекте.",
            "Saved revisions remain in your local project.",
            "Gespeicherte Versionen bleiben in Ihrem lokalen Projekt erhalten.",
            "Les versions enregistrées restent dans votre projet local.",
            "Las versiones guardadas permanecen en tu proyecto local.",
            "As versões salvas permanecem no seu projeto local.",
            "已保存的版本保留在本地项目中。",
            "保存した版はローカルプロジェクトに保持されます。",
        ],
    ),
    (
        "Revision {revision} · Words: {count}",
        [
            "Версия {revision} · Слов: {count}",
            "Revision {revision} · Words: {count}",
            "Version {revision} · Wörter: {count}",
            "Version {revision} · Mots : {count}",
            "Versión {revision} · Palabras: {count}",
            "Versão {revision} · Palavras: {count}",
            "版本 {revision} · 字词数：{count}",
            "版 {revision} · 単語数：{count}",
        ],
    ),
    (
        "Local author edit",
        [
            "Локальная правка автора",
            "Local author edit",
            "Lokale Änderung des Autors",
            "Modification locale de l’auteur",
            "Edición local del autor",
            "Edição local do autor",
            "作者的本地修改",
            "執筆者によるローカル編集",
        ],
    ),
    (
        "Proposal prepared; manuscript unchanged",
        [
            "Предложение подготовлено; рукопись не изменена",
            "Proposal prepared; manuscript unchanged",
            "Vorschlag vorbereitet; Manuskript unverändert",
            "Proposition préparée ; manuscrit inchangé",
            "Propuesta preparada; manuscrito sin cambios",
            "Proposta preparada; manuscrito inalterado",
            "建议已准备好；手稿未更改",
            "提案を準備しました。原稿は変更されていません",
        ],
    ),
    (
        "Changes applied; undo is available",
        [
            "Изменения применены; доступна отмена",
            "Changes applied; undo is available",
            "Änderungen angewendet; Rückgängig ist verfügbar",
            "Modifications appliquées ; annulation possible",
            "Cambios aplicados; se pueden deshacer",
            "Alterações aplicadas; é possível desfazer",
            "已应用更改；可以撤销",
            "変更を適用しました。取り消しが可能です",
        ],
    ),
    (
        "Writers Studio project",
        [
            "Проект Writers Studio",
            "Writers Studio project",
            "Writers-Studio-Projekt",
            "Projet Writers Studio",
            "Proyecto de Writers Studio",
            "Projeto do Writers Studio",
            "Writers Studio 项目",
            "Writers Studio プロジェクト",
        ],
    ),
    (
        "Checking project archive…",
        [
            "Проверяется архив проекта…",
            "Checking project archive…",
            "Projektarchiv wird geprüft…",
            "Vérification de l’archive du projet…",
            "Comprobando archivo del proyecto…",
            "Verificando arquivo do projeto…",
            "正在检查项目归档…",
            "プロジェクトのアーカイブを確認中…",
        ],
    ),
    (
        "Plain text",
        [
            "Обычный текст",
            "Plain text",
            "Nur Text",
            "Texte brut",
            "Texto sin formato",
            "Texto simples",
            "纯文本",
            "プレーンテキスト",
        ],
    ),
    (
        "Text exported",
        [
            "Текст экспортирован",
            "Text exported",
            "Text exportiert",
            "Texte exporté",
            "Texto exportado",
            "Texto exportado",
            "已导出文本",
            "テキストを書き出しました",
        ],
    ),
    (
        "Project title",
        [
            "Название проекта",
            "Project title",
            "Projekttitel",
            "Titre du projet",
            "Título del proyecto",
            "Título do projeto",
            "项目标题",
            "プロジェクト名",
        ],
    ),
    (
        "My new story",
        [
            "Моя новая история",
            "My new story",
            "Meine neue Geschichte",
            "Ma nouvelle histoire",
            "Mi nueva historia",
            "Minha nova história",
            "我的新故事",
            "新しい物語",
        ],
    ),
    (
        "First chapter",
        [
            "Первая глава",
            "First chapter",
            "Erstes Kapitel",
            "Premier chapitre",
            "Primer capítulo",
            "Primeiro capítulo",
            "第一章",
            "最初の章",
        ],
    ),
    (
        "New document",
        [
            "Новый документ",
            "New document",
            "Neues Dokument",
            "Nouveau document",
            "Nuevo documento",
            "Novo documento",
            "新建文档",
            "新規文書",
        ],
    ),
    (
        "Document title",
        [
            "Название документа",
            "Document title",
            "Dokumenttitel",
            "Titre du document",
            "Título del documento",
            "Título do documento",
            "文档标题",
            "文書名",
        ],
    ),
    (
        "New chapter",
        [
            "Новая глава",
            "New chapter",
            "Neues Kapitel",
            "Nouveau chapitre",
            "Nuevo capítulo",
            "Novo capítulo",
            "新的章节",
            "新しい章",
        ],
    ),
    (
        ".wstudio project preview",
        [
            "Проверка проекта .wstudio",
            ".wstudio project preview",
            ".wstudio-Projektvorschau",
            "Aperçu du projet .wstudio",
            "Vista previa del proyecto .wstudio",
            "Prévia do projeto .wstudio",
            ".wstudio 项目预览",
            ".wstudio プロジェクトのプレビュー",
        ],
    ),
    (
        "Format version: {version} · Documents: {count}",
        [
            "Версия формата: {version} · Документов: {count}",
            "Format version: {version} · Documents: {count}",
            "Formatversion: {version} · Dokumente: {count}",
            "Version du format : {version} · Documents : {count}",
            "Versión del formato: {version} · Documentos: {count}",
            "Versão do formato: {version} · Documentos: {count}",
            "格式版本：{version} · 文档数：{count}",
            "形式のバージョン：{version} · 文書数：{count}",
        ],
    ),
    (
        "Archive checked. Text preview is available. Full project import and rich-document editing are not implemented yet.",
        [
            "Архив проверен. Сейчас доступен предпросмотр текста. Полный перенос проекта и редактирование rich-документов ещё не реализованы.",
            "Archive checked. Text preview is available. Full project import and rich-document editing are not implemented yet.",
            "Archiv geprüft. Eine Textvorschau ist verfügbar. Vollständiger Projektimport und Bearbeitung strukturierter Dokumente sind noch nicht implementiert.",
            "Archive vérifiée. Un aperçu du texte est disponible. L’importation complète du projet et l’édition des documents enrichis ne sont pas encore implémentées.",
            "Archivo comprobado. La vista previa del texto está disponible. La importación completa del proyecto y la edición de documentos enriquecidos aún no están implementadas.",
            "Arquivo verificado. A prévia do texto está disponível. A importação completa do projeto e a edição de documentos ricos ainda não foram implementadas.",
            "归档已检查，可预览文本。完整项目导入和富文本文档编辑尚未实现。",
            "アーカイブを確認しました。テキストをプレビューできます。プロジェクト全体の読み込みとリッチ文書の編集はまだ実装されていません。",
        ],
    ),
    (
        "Understood",
        [
            "Понятно",
            "Understood",
            "Verstanden",
            "Compris",
            "Entendido",
            "Entendido",
            "知道了",
            "確認",
        ],
    ),
    (
        "Settings · WriterCraft",
        [
            "Настройки · WriterCraft",
            "Settings · WriterCraft",
            "Einstellungen · WriterCraft",
            "Réglages · WriterCraft",
            "Ajustes · WriterCraft",
            "Configurações · WriterCraft",
            "设置 · WriterCraft",
            "設定 · WriterCraft",
        ],
    ),
    (
        "Appearance",
        [
            "Оформление",
            "Appearance",
            "Darstellung",
            "Apparence",
            "Apariencia",
            "Aparência",
            "外观",
            "外観",
        ],
    ),
    (
        "A calm workspace that suits you.",
        [
            "Спокойная рабочая среда, которая подходит именно вам.",
            "A calm workspace that suits you.",
            "Eine ruhige Arbeitsumgebung, die zu Ihnen passt.",
            "Un espace de travail apaisant, à votre image.",
            "Un espacio de trabajo tranquilo, a tu medida.",
            "Um espaço de trabalho tranquilo, do seu jeito.",
            "适合你的安静创作空间。",
            "あなたに合った、落ち着いた執筆環境。",
        ],
    ),
    (
        "Mode",
        [
            "Режим",
            "Mode",
            "Modus",
            "Mode",
            "Modo",
            "Modo",
            "模式",
            "モード",
        ],
    ),
    (
        "Palette",
        [
            "Палитра",
            "Palette",
            "Farbpalette",
            "Palette",
            "Paleta",
            "Paleta",
            "配色",
            "カラーパレット",
        ],
    ),
    (
        "Manuscript text size",
        [
            "Размер текста рукописи",
            "Manuscript text size",
            "Manuskript-Schriftgröße",
            "Taille du texte du manuscrit",
            "Tamaño del texto del manuscrito",
            "Tamanho do texto do manuscrito",
            "手稿字号",
            "原稿の文字サイズ",
        ],
    ),
    (
        "Keyboard shortcuts",
        [
            "Сочетания клавиш",
            "Keyboard shortcuts",
            "Tastenkürzel",
            "Raccourcis clavier",
            "Atajos de teclado",
            "Atalhos de teclado",
            "键盘快捷键",
            "キーボードショートカット",
        ],
    ),
    (
        "⌘S — save     ⌘N — new project     ⌘O — open archive\n⌘⇧A — review panel     ⌘⇧F — focus mode     ⌘, — settings",
        [
            "⌘S — сохранить     ⌘N — новый проект     ⌘O — открыть архив\n⌘⇧A — панель правок     ⌘⇧F — режим фокуса     ⌘, — настройки",
            "⌘S — save     ⌘N — new project     ⌘O — open archive\n⌘⇧A — review panel     ⌘⇧F — focus mode     ⌘, — settings",
            "⌘S — speichern     ⌘N — neues Projekt     ⌘O — Archiv öffnen\n⌘⇧A — Überarbeitung     ⌘⇧F — Fokusmodus     ⌘, — Einstellungen",
            "⌘S — enregistrer     ⌘N — nouveau projet     ⌘O — ouvrir une archive\n⌘⇧A — révision     ⌘⇧F — concentration     ⌘, — réglages",
            "⌘S — guardar     ⌘N — nuevo proyecto     ⌘O — abrir archivo\n⌘⇧A — revisión     ⌘⇧F — concentración     ⌘, — ajustes",
            "⌘S — salvar     ⌘N — novo projeto     ⌘O — abrir arquivo\n⌘⇧A — revisão     ⌘⇧F — modo de foco     ⌘, — configurações",
            "⌘S — 保存     ⌘N — 新建项目     ⌘O — 打开归档\n⌘⇧A — 审阅面板     ⌘⇧F — 专注模式     ⌘, — 设置",
            "⌘S — 保存     ⌘N — 新規プロジェクト     ⌘O — アーカイブを開く\n⌘⇧A — 校正パネル     ⌘⇧F — 集中モード     ⌘, — 設定",
        ],
    ),
    (
        "Done",
        [
            "Готово",
            "Done",
            "Fertig",
            "Terminé",
            "Listo",
            "Concluído",
            "完成",
            "完了",
        ],
    ),
    (
        "Archive checked",
        [
            "Архив проверен",
            "Archive checked",
            "Archiv geprüft",
            "Archive vérifiée",
            "Archivo comprobado",
            "Arquivo verificado",
            "归档已检查",
            "アーカイブを確認しました",
        ],
    ),
    (
        "Cannot determine the file folder",
        [
            "Не удалось определить папку файла",
            "Cannot determine the file folder",
            "Dateiordner konnte nicht ermittelt werden",
            "Impossible de déterminer le dossier du fichier",
            "No se puede determinar la carpeta del archivo",
            "Não foi possível determinar a pasta do arquivo",
            "无法确定文件所在文件夹",
            "ファイルのフォルダーを特定できません",
        ],
    ),
    (
        "System",
        [
            "Системная",
            "System",
            "System",
            "Système",
            "Sistema",
            "Sistema",
            "跟随系统",
            "システム",
        ],
    ),
    (
        "Light",
        [
            "Светлая",
            "Light",
            "Hell",
            "Clair",
            "Claro",
            "Claro",
            "浅色",
            "ライト",
        ],
    ),
    (
        "Dark",
        [
            "Тёмная",
            "Dark",
            "Dunkel",
            "Sombre",
            "Oscuro",
            "Escuro",
            "深色",
            "ダーク",
        ],
    ),
    (
        "Graphite and blue",
        [
            "Графит и синий",
            "Graphite and blue",
            "Graphit und Blau",
            "Graphite et bleu",
            "Grafito y azul",
            "Grafite e azul",
            "石墨与蓝",
            "グラファイトと青",
        ],
    ),
    (
        "Cedar and copper",
        [
            "Кедр и медь",
            "Cedar and copper",
            "Zeder und Kupfer",
            "Cèdre et cuivre",
            "Cedro y cobre",
            "Cedro e cobre",
            "雪松与铜",
            "シダーと銅",
        ],
    ),
    (
        "Dried rose and wine",
        [
            "Сухая роза и вино",
            "Dried rose and wine",
            "Getrocknete Rose und Wein",
            "Rose séchée et vin",
            "Rosa seca y vino",
            "Rosa seca e vinho",
            "干玫瑰与酒红",
            "ドライローズとワイン",
        ],
    ),
    (
        "Fog and graphite",
        [
            "Туман и графит",
            "Fog and graphite",
            "Nebel und Graphit",
            "Brume et graphite",
            "Niebla y grafito",
            "Névoa e grafite",
            "薄雾与石墨",
            "霧とグラファイト",
        ],
    ),
    (
        "Linen and clay",
        [
            "Лён и глина",
            "Linen and clay",
            "Leinen und Ton",
            "Lin et argile",
            "Lino y arcilla",
            "Linho e argila",
            "亚麻与陶土",
            "リネンとクレイ",
        ],
    ),
    (
        "Dusty lavender",
        [
            "Пыльная лаванда",
            "Dusty lavender",
            "Staubiger Lavendel",
            "Lavande poudrée",
            "Lavanda empolvada",
            "Lavanda suave",
            "灰紫薰衣草",
            "くすんだラベンダー",
        ],
    ),
    (
        "Porcelain and ink",
        [
            "Фарфор и чернила",
            "Porcelain and ink",
            "Porzellan und Tinte",
            "Porcelaine et encre",
            "Porcelana y tinta",
            "Porcelana e tinta",
            "瓷白与墨色",
            "磁器とインク",
        ],
    ),
    (
        "Interface language",
        [
            "Язык интерфейса",
            "Interface language",
            "Sprache der Oberfläche",
            "Langue de l’interface",
            "Idioma de la interfaz",
            "Idioma da interface",
            "界面语言",
            "表示言語",
        ],
    ),
    (
        "Language changes only the interface; your document text stays the same.",
        [
            "Язык меняет только интерфейс; текст ваших документов остаётся прежним.",
            "Language changes only the interface; your document text stays the same.",
            "Die Sprache ändert nur die Oberfläche; Ihr Dokumenttext bleibt unverändert.",
            "La langue ne change que l’interface ; le texte de vos documents reste identique.",
            "El idioma solo cambia la interfaz; el texto de tus documentos sigue igual.",
            "O idioma altera apenas a interface; o texto dos seus documentos permanece igual.",
            "语言设置只影响界面，不会更改文档内容。",
            "言語の変更は画面表示のみに適用され、文書のテキストは変わりません。",
        ],
    ),
    (
        "Prepare an edit from selection",
        [
            "Подготовить правку выделения",
            "Prepare an edit from selection",
            "Änderung aus Auswahl vorbereiten",
            "Préparer une modification de la sélection",
            "Preparar una edición de la selección",
            "Preparar uma edição da seleção",
            "根据选中文本准备修改",
            "選択範囲の編集を準備",
        ],
    ),
    (
        "Copy selection",
        [
            "Скопировать выделение",
            "Copy selection",
            "Auswahl kopieren",
            "Copier la sélection",
            "Copiar selección",
            "Copiar seleção",
            "复制所选内容",
            "選択範囲をコピー",
        ],
    ),
    (
        "Selected excerpt sent to the review panel",
        [
            "Выделенный фрагмент передан в панель правок",
            "Selected excerpt sent to the review panel",
            "Markierter Auszug an die Überarbeitungsleiste übergeben",
            "Extrait sélectionné envoyé au panneau de révision",
            "Fragmento seleccionado enviado al panel de revisión",
            "Trecho selecionado enviado ao painel de revisão",
            "已将所选片段发送到审阅面板",
            "選択した抜粋を校正パネルに送りました",
        ],
    ),
    (
        "Technical details",
        [
            "Технические сведения",
            "Technical details",
            "Technische Details",
            "Détails techniques",
            "Detalles técnicos",
            "Detalhes técnicos",
            "技术详情",
            "技術的な詳細",
        ],
    ),
    (
        "Check the entered values and try again.",
        [
            "Проверьте введённые данные и повторите действие.",
            "Check the entered values and try again.",
            "Prüfen Sie die eingegebenen Werte und versuchen Sie es erneut.",
            "Vérifiez les valeurs saisies et réessayez.",
            "Comprueba los valores introducidos y vuelve a intentarlo.",
            "Verifique os valores inseridos e tente novamente.",
            "请检查输入的内容，然后重试。",
            "入力内容を確認して、もう一度お試しください。",
        ],
    ),
    (
        "The document changed. Reload it before saving.",
        [
            "Документ изменён. Перезагрузите его перед сохранением.",
            "The document changed. Reload it before saving.",
            "Das Dokument wurde geändert. Laden Sie es vor dem Speichern neu.",
            "Le document a changé. Rechargez-le avant d’enregistrer.",
            "El documento ha cambiado. Vuelve a cargarlo antes de guardar.",
            "O documento foi alterado. Recarregue-o antes de salvar.",
            "文档已更改，请重新加载后再保存。",
            "文書が変更されています。保存する前に再読み込みしてください。",
        ],
    ),
    (
        "Recover the unsaved draft from another revision first.",
        [
            "Сначала восстановите несохранённый черновик другой версии.",
            "Recover the unsaved draft from another revision first.",
            "Stellen Sie zuerst den ungespeicherten Entwurf einer anderen Version wieder her.",
            "Récupérez d’abord le brouillon non enregistré d’une autre version.",
            "Primero recupera el borrador sin guardar de otra versión.",
            "Primeiro recupere o rascunho não salvo de outra versão.",
            "请先恢复其他版本中未保存的草稿。",
            "先に別の版の未保存の下書きを復元してください。",
        ],
    ),
    (
        "This document is read only to preserve its original structure.",
        [
            "Этот документ доступен только для чтения, чтобы сохранить исходную структуру.",
            "This document is read only to preserve its original structure.",
            "Dieses Dokument ist schreibgeschützt, damit seine ursprüngliche Struktur erhalten bleibt.",
            "Ce document est en lecture seule afin de préserver sa structure d’origine.",
            "Este documento es de solo lectura para conservar su estructura original.",
            "Este documento é somente leitura para preservar sua estrutura original.",
            "此文档为只读模式，以保留原始结构。",
            "元の構造を保持するため、この文書は読み取り専用です。",
        ],
    ),
    (
        "The exact excerpt was not found. Select existing text.",
        [
            "Точный фрагмент не найден. Выберите существующий текст.",
            "The exact excerpt was not found. Select existing text.",
            "Der exakte Auszug wurde nicht gefunden. Markieren Sie vorhandenen Text.",
            "L’extrait exact est introuvable. Sélectionnez du texte existant.",
            "No se encontró el fragmento exacto. Selecciona texto existente.",
            "O trecho exato não foi encontrado. Selecione um texto existente.",
            "未找到完全一致的片段，请选择现有文本。",
            "完全に一致する抜粋が見つかりません。既存のテキストを選択してください。",
        ],
    ),
    (
        "The excerpt occurs more than once. Select a longer unique excerpt.",
        [
            "Фрагмент встречается несколько раз. Выберите более длинный уникальный текст.",
            "The excerpt occurs more than once. Select a longer unique excerpt.",
            "Der Auszug kommt mehrfach vor. Wählen Sie einen längeren eindeutigen Text.",
            "L’extrait apparaît plusieurs fois. Sélectionnez un texte plus long et unique.",
            "El fragmento aparece varias veces. Selecciona un texto más largo y único.",
            "O trecho aparece mais de uma vez. Selecione um texto mais longo e único.",
            "该片段出现了多次，请选择更长且唯一的文本。",
            "同じ抜粋が複数あります。一意に特定できる、より長いテキストを選択してください。",
        ],
    ),
    (
        "The proposal state changed. Refresh the review panel.",
        [
            "Состояние предложения изменилось. Обновите панель правок.",
            "The proposal state changed. Refresh the review panel.",
            "Der Vorschlagsstatus wurde geändert. Aktualisieren Sie die Überarbeitungsleiste.",
            "L’état de la proposition a changé. Actualisez le panneau de révision.",
            "El estado de la propuesta ha cambiado. Actualiza el panel de revisión.",
            "O estado da proposta foi alterado. Atualize o painel de revisão.",
            "建议状态已更改，请刷新审阅面板。",
            "提案の状態が変わりました。校正パネルを更新してください。",
        ],
    ),
    (
        "The proposal is stale. Prepare it again using the current revision.",
        [
            "Предложение устарело. Подготовьте его заново по текущей версии.",
            "The proposal is stale. Prepare it again using the current revision.",
            "Der Vorschlag ist veraltet. Erstellen Sie ihn anhand der aktuellen Version erneut.",
            "La proposition est obsolète. Préparez-la à nouveau à partir de la version actuelle.",
            "La propuesta está desactualizada. Prepárala de nuevo con la versión actual.",
            "A proposta está desatualizada. Prepare-a novamente com a versão atual.",
            "建议已过期，请根据当前版本重新准备。",
            "提案が古くなっています。現在の版を基に準備し直してください。",
        ],
    ),
    (
        "Undo would overwrite later edits. The text was left unchanged.",
        [
            "Отмена затронет последующие правки. Текст оставлен без изменений.",
            "Undo would overwrite later edits. The text was left unchanged.",
            "Rückgängig würde spätere Änderungen überschreiben. Der Text blieb unverändert.",
            "L’annulation écraserait des modifications ultérieures. Le texte n’a pas été modifié.",
            "Deshacer sobrescribiría cambios posteriores. El texto no se ha modificado.",
            "Desfazer sobrescreveria edições posteriores. O texto não foi alterado.",
            "撤销会覆盖后续修改，因此文本保持不变。",
            "取り消すと後の編集が上書きされるため、テキストは変更していません。",
        ],
    ),
    (
        "The item was not found. Refresh the project list.",
        [
            "Объект больше не найден. Обновите список проектов.",
            "The item was not found. Refresh the project list.",
            "Das Element wurde nicht gefunden. Aktualisieren Sie die Projektliste.",
            "L’élément est introuvable. Actualisez la liste des projets.",
            "No se encontró el elemento. Actualiza la lista de proyectos.",
            "O item não foi encontrado. Atualize a lista de projetos.",
            "未找到该项目，请刷新项目列表。",
            "項目が見つかりません。プロジェクト一覧を更新してください。",
        ],
    ),
    (
        "This workspace requires a newer version of WriterCraft.",
        [
            "Для этого рабочего пространства требуется новая версия WriterCraft.",
            "This workspace requires a newer version of WriterCraft.",
            "Dieser Arbeitsbereich erfordert eine neuere Version von WriterCraft.",
            "Cet espace de travail nécessite une version plus récente de WriterCraft.",
            "Este espacio de trabajo requiere una versión más reciente de WriterCraft.",
            "Este espaço de trabalho requer uma versão mais recente do WriterCraft.",
            "此工作区需要更新版本的 WriterCraft。",
            "このワークスペースには、より新しいバージョンの WriterCraft が必要です。",
        ],
    ),
    (
        "Choose a WriterCraft workspace file.",
        [
            "Выберите файл рабочего пространства WriterCraft.",
            "Choose a WriterCraft workspace file.",
            "Wählen Sie eine WriterCraft-Arbeitsbereichsdatei.",
            "Choisissez un fichier d’espace de travail WriterCraft.",
            "Elige un archivo de espacio de trabajo de WriterCraft.",
            "Escolha um arquivo de espaço de trabalho do WriterCraft.",
            "请选择 WriterCraft 工作区文件。",
            "WriterCraft のワークスペースファイルを選択してください。",
        ],
    ),
    (
        "Workspace data is inconsistent. Restore a backup.",
        [
            "Данные рабочего пространства повреждены. Восстановите резервную копию.",
            "Workspace data is inconsistent. Restore a backup.",
            "Die Arbeitsbereichsdaten sind inkonsistent. Stellen Sie eine Sicherung wieder her.",
            "Les données de l’espace de travail sont incohérentes. Restaurez une sauvegarde.",
            "Los datos del espacio de trabajo son incoherentes. Restaura una copia de seguridad.",
            "Os dados do espaço de trabalho são inconsistentes. Restaure um backup.",
            "工作区数据不一致，请恢复备份。",
            "ワークスペースのデータに不整合があります。バックアップから復元してください。",
        ],
    ),
    (
        "The workspace is unavailable. Close and reopen it.",
        [
            "Рабочее пространство занято. Закройте и откройте его снова.",
            "The workspace is unavailable. Close and reopen it.",
            "Der Arbeitsbereich ist nicht verfügbar. Schließen und öffnen Sie ihn erneut.",
            "L’espace de travail est indisponible. Fermez-le puis rouvrez-le.",
            "El espacio de trabajo no está disponible. Ciérralo y vuelve a abrirlo.",
            "O espaço de trabalho está indisponível. Feche-o e abra-o novamente.",
            "工作区不可用，请关闭后重新打开。",
            "ワークスペースを利用できません。いったん閉じてから開き直してください。",
        ],
    ),
    (
        "The local database operation could not be completed.",
        [
            "Не удалось завершить операцию с локальной базой данных.",
            "The local database operation could not be completed.",
            "Der Vorgang in der lokalen Datenbank konnte nicht abgeschlossen werden.",
            "L’opération sur la base de données locale n’a pas pu aboutir.",
            "No se pudo completar la operación de la base de datos local.",
            "Não foi possível concluir a operação no banco de dados local.",
            "无法完成本地数据库操作。",
            "ローカルデータベースの操作を完了できませんでした。",
        ],
    ),
    (
        "The file could not be read or saved. Check the folder and access permissions.",
        [
            "Не удалось прочитать или сохранить файл. Проверьте папку и права доступа.",
            "The file could not be read or saved. Check the folder and access permissions.",
            "Die Datei konnte nicht gelesen oder gespeichert werden. Prüfen Sie Ordner und Zugriffsrechte.",
            "Le fichier n’a pas pu être lu ou enregistré. Vérifiez le dossier et les droits d’accès.",
            "No se pudo leer o guardar el archivo. Comprueba la carpeta y los permisos de acceso.",
            "Não foi possível ler ou salvar o arquivo. Verifique a pasta e as permissões de acesso.",
            "无法读取或保存文件，请检查文件夹和访问权限。",
            "ファイルを読み込み、または保存できませんでした。フォルダーとアクセス権を確認してください。",
        ],
    ),
    (
        "The saved data could not be decoded.",
        [
            "Не удалось прочитать сохранённые данные.",
            "The saved data could not be decoded.",
            "Die gespeicherten Daten konnten nicht dekodiert werden.",
            "Les données enregistrées n’ont pas pu être décodées.",
            "No se pudieron decodificar los datos guardados.",
            "Não foi possível decodificar os dados salvos.",
            "无法解析已保存的数据。",
            "保存されたデータを読み取れませんでした。",
        ],
    ),
    (
        "The archive could not be checked. The original project was not changed.",
        [
            "Не удалось проверить архив. Исходный проект не изменён.",
            "The archive could not be checked. The original project was not changed.",
            "Das Archiv konnte nicht geprüft werden. Das Originalprojekt wurde nicht geändert.",
            "L’archive n’a pas pu être vérifiée. Le projet d’origine n’a pas été modifié.",
            "No se pudo comprobar el archivo. El proyecto original no se ha modificado.",
            "Não foi possível verificar o arquivo. O projeto original não foi alterado.",
            "无法检查归档，原始项目未更改。",
            "アーカイブを確認できませんでした。元のプロジェクトは変更していません。",
        ],
    ),
    (
        "The action could not be completed. Details are available below.",
        [
            "Не удалось завершить действие. Подробности доступны ниже.",
            "The action could not be completed. Details are available below.",
            "Die Aktion konnte nicht abgeschlossen werden. Einzelheiten stehen unten.",
            "L’action n’a pas pu aboutir. Les détails figurent ci-dessous.",
            "No se pudo completar la acción. Los detalles están disponibles abajo.",
            "Não foi possível concluir a ação. Os detalhes estão disponíveis abaixo.",
            "无法完成操作，详情见下方。",
            "操作を完了できませんでした。詳細は以下をご確認ください。",
        ],
    ),
    (
        "Archive checking was interrupted. Try opening it again.",
        [
            "Проверка архива прервана. Попробуйте открыть его снова.",
            "Archive checking was interrupted. Try opening it again.",
            "Die Archivprüfung wurde unterbrochen. Versuchen Sie, das Archiv erneut zu öffnen.",
            "La vérification de l’archive a été interrompue. Essayez de la rouvrir.",
            "La comprobación del archivo se interrumpió. Intenta abrirlo de nuevo.",
            "A verificação do arquivo foi interrompida. Tente abri-lo novamente.",
            "归档检查已中断，请尝试重新打开。",
            "アーカイブの確認が中断されました。もう一度開いてください。",
        ],
    ),
    (
        "Interface settings could not be saved.",
        [
            "Не удалось сохранить настройки интерфейса.",
            "Interface settings could not be saved.",
            "Die Oberflächeneinstellungen konnten nicht gespeichert werden.",
            "Les réglages de l’interface n’ont pas pu être enregistrés.",
            "No se pudieron guardar los ajustes de la interfaz.",
            "Não foi possível salvar as configurações da interface.",
            "无法保存界面设置。",
            "表示設定を保存できませんでした。",
        ],
    ),
    ("px", ["px", "px", "px", "px", "px", "px", "px", "px"]),
];

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "Test setup failures identify the invalid fixture step"
)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(text: &str) -> BTreeSet<&str> {
        let mut result = BTreeSet::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('}') else { break };
            result.insert(&after[..end]);
            rest = &after[end + 1..];
        }
        result
    }

    fn has_cyrillic(text: &str) -> bool {
        text.chars()
            .any(|ch| ('\u{0400}'..='\u{052f}').contains(&ch))
    }

    #[test]
    fn every_message_has_eight_nonempty_translations_and_unique_id() {
        let mut keys = BTreeSet::new();
        assert_eq!(Language::ALL.len(), 8);
        for (key, translations) in CATALOG {
            assert!(keys.insert(key), "Duplicate catalog ID: {key}");
            for language in Language::ALL {
                let text = language.text(key);
                assert!(
                    !text.trim().is_empty(),
                    "Empty {} translation: {key}",
                    language.code()
                );
                assert_ne!(
                    text,
                    MISSING_MESSAGE,
                    "Missing {} translation: {key}",
                    language.code()
                );
                assert_eq!(text, translations[language as usize]);
                if language != Language::Russian {
                    assert!(
                        !has_cyrillic(text),
                        "Russian leaked into {}: {key}",
                        language.code()
                    );
                }
            }
        }
        assert!(
            keys.len() >= 135,
            "Expected the complete initial desktop catalog"
        );
    }

    #[test]
    fn translations_preserve_named_placeholders_in_every_language() {
        for (key, translations) in CATALOG {
            let expected = placeholders(key);
            for (language, translated) in Language::ALL.iter().zip(translations) {
                assert_eq!(
                    placeholders(translated),
                    expected,
                    "Placeholder mismatch in {}: {key}",
                    language.code()
                );
            }
        }
    }

    #[test]
    fn interpolation_changes_named_values_without_translating_or_reprocessing_them() {
        let text = Language::English.format(
            "Words: {count}   ·   Revision {revision}",
            &[("count", "{revision}".into()), ("revision", "4".into())],
        );
        assert_eq!(text, "Words: {revision}   ·   Revision 4");
    }

    #[test]
    fn missing_message_is_visible_without_russian_fallback() {
        for language in Language::ALL {
            assert_eq!(language.text("unknown_message_key"), MISSING_MESSAGE);
        }
    }

    #[test]
    fn preferences_persist_language_and_font_without_touching_theme_file() {
        let directory = tempfile::tempdir().expect("preferences directory");
        let theme = directory.path().join("appearance.json");
        let original_theme = br#"{"appearance":"dark","palette":"solid-craft-blue"}"#;
        std::fs::write(&theme, original_theme).expect("existing theme preferences");
        let path = directory.path().join("interface.json");
        let preferences = InterfacePreferences {
            language: Language::Japanese,
            font_size: 24.0,
        };
        preferences.save(&path).expect("save interface preferences");
        assert_eq!(
            InterfacePreferences::load(&path).expect("reload interface preferences"),
            preferences
        );
        assert_eq!(
            std::fs::read(theme).expect("unchanged theme file"),
            original_theme
        );
    }

    #[test]
    fn first_launch_and_partial_preferences_have_stable_defaults() {
        let directory = tempfile::tempdir().expect("preferences directory");
        let path = directory.path().join("interface.json");
        assert_eq!(
            InterfacePreferences::load(&path).expect("first launch"),
            InterfacePreferences::default()
        );
        std::fs::write(&path, br#"{"language":"de"}"#).expect("partial preferences");
        assert_eq!(
            InterfacePreferences::load(&path).expect("partial load"),
            InterfacePreferences {
                language: Language::German,
                font_size: 21.0,
            }
        );
    }

    #[test]
    fn invalid_preferences_remain_on_disk_for_recovery() {
        let directory = tempfile::tempdir().expect("preferences directory");
        let path = directory.path().join("interface.json");
        std::fs::write(&path, b"{invalid").expect("invalid preferences fixture");
        assert!(InterfacePreferences::load(&path).is_err());
        assert_eq!(
            std::fs::read(path).expect("original invalid preferences"),
            b"{invalid"
        );
    }

    #[test]
    fn font_size_is_bounded_when_loading_old_or_modified_preferences() {
        let directory = tempfile::tempdir().expect("preferences directory");
        let path = directory.path().join("interface.json");
        std::fs::write(&path, br#"{"language":"fr","font_size":1000}"#).expect("font fixture");
        assert_eq!(
            InterfacePreferences::load(path)
                .expect("bounded font")
                .font_size,
            28.0
        );
    }

    fn string_literals(source: &str) -> Vec<String> {
        let mut values = Vec::new();
        let mut chars = source.char_indices();
        while let Some((start, ch)) = chars.next() {
            if ch != '"' {
                continue;
            }
            let mut escaped = false;
            for (end, ch) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    if let Ok(value) = serde_json::from_str::<String>(&source[start..=end]) {
                        values.push(value);
                    }
                    break;
                }
            }
        }
        values
    }

    #[test]
    fn every_cyrillic_desktop_ui_message_is_in_the_catalog() {
        let mut app = include_str!("app.rs").to_owned();
        // Sample manuscript and its titles are user content, not interface strings.
        if let Some(start) = app.find("    fn create_example(") {
            let end = app[start + 1..]
                .find("\n    fn ")
                .map(|offset| start + 1 + offset)
                .unwrap_or(app.len());
            app.replace_range(start..end, "");
        }
        if let Some(sample) = app.find("const SAMPLE:") {
            app.truncate(sample);
        }
        app.push_str(include_str!("theme.rs"));
        let keys: BTreeSet<_> = CATALOG.iter().map(|(key, _)| *key).collect();
        let missing: Vec<_> = string_literals(&app)
            .into_iter()
            .filter(|text| has_cyrillic(text) && !keys.contains(text.as_str()))
            .collect();
        assert!(missing.is_empty(), "Uncatalogued UI strings: {missing:?}");
    }
}
