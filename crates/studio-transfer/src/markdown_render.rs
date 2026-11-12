//! Markdown is a reading representation; canonical source retains every unsupported semantic.
use std::collections::{BTreeSet, HashMap};

use serde_json::Value;

use super::portable_paths::relative_link;
use super::{Result, TransferError};

pub(super) struct MarkdownRenderer<'a> {
    pub(super) current: &'a str,
    pub(super) project: &'a str,
    pub(super) nodes: &'a HashMap<String, String>,
    pub(super) anchors: &'a HashMap<String, String>,
    pub(super) heading_offset: u8,
    /// Folder exports keep per-block HTML anchors; a reader manuscript omits that invisible markup.
    pub(super) block_anchors: bool,
    pub(super) assets: &'a HashMap<String, String>,
    pub(super) conversions: BTreeSet<String>,
}
impl MarkdownRenderer<'_> {
    pub(super) fn render(&mut self, node: &Value) -> Result<String> {
        let kind = node["type"]
            .as_str()
            .ok_or_else(|| TransferError::Invalid("Rich export node has no type".into()))?;
        let children = node["content"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        let attrs = &node["attrs"];
        let rendered = match kind {
            "doc" => self.blocks(children),
            "text" => self.marked_text(node),
            "paragraph" => self.inline(children),
            "heading" => Ok(format!(
                "{} {}",
                "#".repeat(
                    u8::try_from(attrs["level"].as_u64().unwrap_or(1).clamp(1, 6))
                        .unwrap_or(1)
                        .saturating_add(self.heading_offset)
                        .min(6) as usize
                ),
                self.inline(children)?
            )),
            "hardBreak" => Ok("  \n".into()),
            "blockquote" => Ok(quote(&self.blocks(children)?)),
            "bulletList" => self.list(children, None),
            "orderedList" => {
                if attrs.get("type").is_some_and(|v| !v.is_null()) {
                    self.conversions
                        .insert("ordered-list-numbering-style-in-companion".into());
                }
                self.list(children, Some(attrs["start"].as_u64().unwrap_or(1)))
            }
            "listItem" => self.blocks(children),
            "horizontalRule" => Ok("---".into()),
            "sceneBreak" => {
                self.conversions
                    .insert("scene-break-reading-separator".into());
                Ok("* * *".into())
            }
            "codeBlock" => {
                let language = attrs["language"].as_str().unwrap_or("");
                if language.contains(['\n', '\r', '`']) {
                    self.conversions
                        .insert("code-language-label-in-companion".into());
                }
                Ok(fenced(&literal(node), language))
            }
            "mermaidBlock" => {
                self.conversions
                    .insert("mermaid-reader-support-required".into());
                let mut out = fenced(attrs["code"].as_str().unwrap_or(""), "mermaid");
                if let Some(caption) = attrs["caption"].as_str().filter(|s| !s.is_empty()) {
                    out.push_str("\n\n");
                    out.push_str(&escape_text(caption));
                }
                Ok(out)
            }
            "inlineMath" => {
                self.conversions
                    .insert("formula-reader-support-required".into());
                if attrs["description"].as_str().is_some_and(|s| !s.is_empty()) {
                    self.conversions
                        .insert("formula-description-in-companion".into());
                }
                let source = attrs["latex"].as_str().unwrap_or("");
                if source.contains(['$', '\n', '\r']) {
                    self.conversions
                        .insert("inline-formula-delimiter-source-reading-view".into());
                    Ok(code_span(source))
                } else {
                    Ok(format!("${source}$"))
                }
            }
            "mathBlock" => {
                self.conversions
                    .insert("formula-reader-support-required".into());
                let source = attrs["latex"].as_str().unwrap_or("");
                let mut out = if source.lines().any(|line| line.trim() == "$$") {
                    self.conversions
                        .insert("formula-delimiter-fenced-source".into());
                    fenced(source, "math")
                } else {
                    format!("$$\n{source}\n$$")
                };
                if let Some(description) = attrs["description"].as_str().filter(|s| !s.is_empty()) {
                    out.push_str("\n\n");
                    out.push_str(&escape_text(description));
                }
                Ok(out)
            }
            "imageBlock" => {
                let mut out = match self.image_target(attrs)? {
                    Some(target) => format!(
                        "![{}](<{}>)",
                        escape_text(attrs["alt"].as_str().unwrap_or("")),
                        target
                    ),
                    None => format!(
                        "[Image source retained in canonical companion](<{}>)",
                        relative_link(self.current, ".writercraft/project.json")?
                    ),
                };
                if let Some(caption) = attrs["caption"].as_str().filter(|s| !s.is_empty()) {
                    out.push_str("\n\n");
                    out.push_str(&escape_text(caption));
                }
                if attrs.get("width").is_some_and(|v| !v.is_null())
                    || attrs.get("height").is_some_and(|v| !v.is_null())
                {
                    self.conversions
                        .insert("image-display-size-in-companion".into());
                }
                Ok(out)
            }
            "callout" => {
                self.conversions.insert("callout-reader-extension".into());
                Ok(format!(
                    "> [!{}]\n{}",
                    attrs["kind"]
                        .as_str()
                        .unwrap_or("info")
                        .to_ascii_uppercase(),
                    quote(&self.blocks(children)?)
                ))
            }
            "table" => self.table(children),
            "tableRow" | "tableHeader" | "tableCell" => self.blocks(children),
            _ => {
                self.conversions
                    .insert(format!("unsupported-rich-node:{kind}"));
                Ok(format!(
                    "[Preserved rich content](<{}>)",
                    relative_link(self.current, ".writercraft/project.json")?
                ))
            }
        }?;
        if self.block_anchors
            && !matches!(kind, "text" | "hardBreak" | "inlineMath")
            && let Some(id) = attrs["blockId"].as_str().filter(|id| !id.is_empty())
        {
            self.conversions.insert("block-identity-html-anchor".into());
            return Ok(format!("<a id=\"{}\"></a>\n\n{rendered}", html_escape(id)));
        }
        Ok(rendered)
    }
    fn blocks(&mut self, nodes: &[Value]) -> Result<String> {
        Ok(nodes
            .iter()
            .map(|node| self.render(node))
            .collect::<Result<Vec<_>>>()?
            .join("\n\n"))
    }
    fn inline(&mut self, nodes: &[Value]) -> Result<String> {
        Ok(nodes
            .iter()
            .map(|node| self.render(node))
            .collect::<Result<Vec<_>>>()?
            .concat())
    }
    fn marked_text(&mut self, node: &Value) -> Result<String> {
        let text = node["text"].as_str().unwrap_or("");
        let marks = node["marks"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        let code = marks
            .iter()
            .any(|mark| mark["type"].as_str() == Some("code"));
        let mut out = if code {
            if text.contains(['\n', '\r']) {
                self.conversions
                    .insert("inline-code-linebreak-reading-normalization".into());
            }
            code_span(text)
        } else {
            escape_text(text)
        };
        for mark in marks {
            match mark["type"].as_str().unwrap_or("") {
                "code" => {}
                "bold" => out = wrap_mark(&out, "**"),
                "italic" => out = wrap_mark(&out, "*"),
                "strike" => out = wrap_mark(&out, "~~"),
                "underline" => {
                    self.conversions
                        .insert("underline-html-reader-support".into());
                    out = format!("<u>{out}</u>");
                }
                "link" => {
                    let attrs = &mark["attrs"];
                    let href = attrs["href"].as_str().unwrap_or("");
                    let href = self.link_target(href)?;
                    if let Some(href) = href {
                        let title = attrs["title"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| {
                                format!(
                                    " \"{}\"",
                                    s.replace('\\', "\\\\")
                                        .replace('"', "\\\"")
                                        .replace(['\n', '\r'], " ")
                                )
                            })
                            .unwrap_or_default();
                        out = format!("[{out}](<{}>{title})", escape_url(&href));
                    }
                    if ["target", "rel", "class"]
                        .iter()
                        .any(|key| attrs.get(key).is_some_and(|v| !v.is_null()))
                    {
                        self.conversions
                            .insert("extended-link-attributes-in-companion".into());
                    }
                }
                unknown => {
                    self.conversions
                        .insert(format!("unsupported-rich-mark:{unknown}"));
                }
            }
        }
        Ok(out)
    }
    fn link_target(&mut self, href: &str) -> Result<Option<String>> {
        let (target, fragment) = href.split_once('#').map_or((href, ""), |(a, b)| (a, b));
        let document = target
            .strip_prefix("writers-studio://documents/")
            .or_else(|| target.strip_prefix(&format!("/projects/{}/studio/", self.project)));
        if let Some(id) = document {
            if let Some(anchor) = self.anchors.get(id) {
                return Ok(Some(anchor.clone()));
            }
            if let Some(path) = self.nodes.get(id) {
                let mut relative = relative_link(self.current, path)?;
                if !fragment.is_empty() {
                    relative.push('#');
                    relative.push_str(&super::portable_paths::percent_encode(fragment));
                }
                return Ok(Some(relative));
            }
            self.conversions
                .insert("document-link-outside-export-scope".into());
            return Ok(None);
        }
        if let Some(id) = target.strip_prefix("asset://").or_else(|| {
            target
                .strip_prefix("/api/media/")
                .and_then(|s| s.strip_suffix("/content"))
        }) {
            let path = self.assets.get(id).ok_or_else(|| {
                TransferError::Invalid(
                    "A linked project artifact is missing from the export snapshot".into(),
                )
            })?;
            return Ok(Some(relative_link(self.current, path)?));
        }
        if href.starts_with('#')
            || ["https://", "http://", "mailto:", "tel:"]
                .iter()
                .any(|prefix| href.starts_with(prefix))
        {
            return Ok(Some(href.to_owned()));
        }
        self.conversions
            .insert("unresolved-or-application-link-in-companion".into());
        Ok(None)
    }
    fn image_target(&mut self, attrs: &Value) -> Result<Option<String>> {
        let source = attrs["src"].as_str().unwrap_or("");
        let identity = attrs["assetId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .or_else(|| source.strip_prefix("asset://"))
            .or_else(|| {
                source
                    .strip_prefix("/api/media/")
                    .and_then(|s| s.strip_suffix("/content"))
            });
        if let Some(id) = identity {
            let path = self.assets.get(id).ok_or_else(|| {
                TransferError::Invalid(
                    "A referenced project image is missing from the export snapshot".into(),
                )
            })?;
            return Ok(Some(relative_link(self.current, path)?));
        }
        self.conversions
            .insert("external-or-opaque-image-source-retained".into());
        if source.starts_with("https://") || source.starts_with("http://") {
            Ok(Some(escape_url(source)))
        } else {
            Ok(None)
        }
    }
    fn list(&mut self, items: &[Value], start: Option<u64>) -> Result<String> {
        let mut out = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let marker = start
                .map(|n| format!("{}.", n.saturating_add(index as u64)))
                .unwrap_or_else(|| "-".into());
            let body = self.render(item)?;
            let indent = " ".repeat(marker.len() + 1);
            let mut lines = body.split('\n');
            let first = lines.next().unwrap_or("");
            let mut entry = format!("{marker} {first}");
            for line in lines {
                entry.push('\n');
                entry.push_str(&indent);
                entry.push_str(line);
            }
            out.push(entry);
        }
        Ok(out.join("\n"))
    }
    fn table(&mut self, rows: &[Value]) -> Result<String> {
        let width = rows
            .first()
            .and_then(|row| row["content"].as_array())
            .map(Vec::len)
            .unwrap_or(0);
        let simple = width > 0
            && rows.iter().all(|row| {
                row["content"].as_array().is_some_and(|cells| {
                    cells.len() == width
                        && cells.iter().all(|cell| {
                            cell["attrs"]["colspan"].as_u64().unwrap_or(1) == 1
                                && cell["attrs"]["rowspan"].as_u64().unwrap_or(1) == 1
                                && cell["content"].as_array().is_some_and(|blocks| {
                                    blocks.len() == 1
                                        && blocks[0]["type"].as_str() == Some("paragraph")
                                })
                        })
                })
            });
        if !simple {
            self.conversions
                .insert("complex-table-html-reading-view".into());
            let mut out = "<table>\n".to_owned();
            for row in rows {
                out.push_str("<tr>");
                for cell in row["content"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                    let tag = if cell["type"].as_str() == Some("tableHeader") {
                        "th"
                    } else {
                        "td"
                    };
                    out.push_str(&format!(
                        "<{tag} colspan=\"{}\" rowspan=\"{}\">",
                        cell["attrs"]["colspan"].as_u64().unwrap_or(1),
                        cell["attrs"]["rowspan"].as_u64().unwrap_or(1)
                    ));
                    for child in cell["content"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                        out.push_str(&self.html(child)?);
                    }
                    out.push_str(&format!("</{tag}>"));
                }
                out.push_str("</tr>\n");
            }
            out.push_str("</table>");
            return Ok(out);
        }
        self.conversions
            .insert("table-widths-and-cell-metadata-in-companion".into());
        let mut out = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let mut cells = Vec::new();
            for cell in row["content"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                cells.push(table_cell(&self.render(&cell["content"][0])?));
            }
            out.push(format!("| {} |", cells.join(" | ")));
            if index == 0 {
                let align = row["content"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
                    .iter()
                    .map(|cell| match cell["attrs"]["align"].as_str() {
                        Some("left") => ":---",
                        Some("right") => "---:",
                        Some("center") => ":---:",
                        _ => "---",
                    })
                    .collect::<Vec<_>>();
                out.push(format!("| {} |", align.join(" | ")));
                if row["content"].as_array().is_some_and(|cells| {
                    cells
                        .iter()
                        .any(|cell| cell["type"].as_str() != Some("tableHeader"))
                }) {
                    self.conversions
                        .insert("markdown-table-first-row-as-header".into());
                }
            }
        }
        Ok(out.join("\n"))
    }
    fn html(&mut self, node: &Value) -> Result<String> {
        let children = node["content"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        let mut inner = String::new();
        for child in children {
            inner.push_str(&self.html(child)?);
        }
        let attrs = &node["attrs"];
        Ok(match node["type"].as_str().unwrap_or("") {
            "text" => {
                let mut text = html_escape(node["text"].as_str().unwrap_or(""));
                for mark in node["marks"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                    text = match mark["type"].as_str().unwrap_or("") {
                        "bold" => format!("<strong>{text}</strong>"),
                        "italic" => format!("<em>{text}</em>"),
                        "underline" => format!("<u>{text}</u>"),
                        "strike" => format!("<s>{text}</s>"),
                        "code" => format!("<code>{text}</code>"),
                        "link" => {
                            match self.link_target(mark["attrs"]["href"].as_str().unwrap_or(""))? {
                                Some(target) => {
                                    format!("<a href=\"{}\">{text}</a>", html_escape(&target))
                                }
                                None => text,
                            }
                        }
                        _ => text,
                    };
                }
                text
            }
            "table" => format!("<table>{inner}</table>"),
            "tableRow" => format!("<tr>{inner}</tr>"),
            "tableHeader" | "tableCell" => {
                let tag = if node["type"].as_str() == Some("tableHeader") {
                    "th"
                } else {
                    "td"
                };
                format!(
                    "<{tag} colspan=\"{}\" rowspan=\"{}\">{inner}</{tag}>",
                    attrs["colspan"].as_u64().unwrap_or(1),
                    attrs["rowspan"].as_u64().unwrap_or(1)
                )
            }
            "paragraph" => format!("<p>{inner}</p>"),
            "heading" => {
                let level = attrs["level"].as_u64().unwrap_or(1).clamp(1, 6);
                format!("<h{level}>{inner}</h{level}>")
            }
            "blockquote" => format!("<blockquote>{inner}</blockquote>"),
            "bulletList" => format!("<ul>{inner}</ul>"),
            "orderedList" => format!(
                "<ol start=\"{}\">{inner}</ol>",
                attrs["start"].as_u64().unwrap_or(1)
            ),
            "listItem" => format!("<li>{inner}</li>"),
            "hardBreak" => "<br>".into(),
            "codeBlock" => format!("<pre><code>{}</code></pre>", html_escape(&literal(node))),
            "imageBlock" => match self.image_target(attrs)? {
                Some(target) => format!(
                    "<img src=\"{}\" alt=\"{}\">",
                    html_escape(&target),
                    html_escape(attrs["alt"].as_str().unwrap_or(""))
                ),
                None => "<span>Image source retained in canonical companion</span>".into(),
            },
            "horizontalRule" | "sceneBreak" => "<hr>".into(),
            "inlineMath" | "mathBlock" => format!(
                "<code>{}</code>",
                html_escape(attrs["latex"].as_str().unwrap_or(""))
            ),
            "mermaidBlock" => format!(
                "<pre><code>{}</code></pre>",
                html_escape(attrs["code"].as_str().unwrap_or(""))
            ),
            "callout" => format!("<blockquote>{inner}</blockquote>"),
            _ => inner,
        })
    }
}
pub(super) fn escape_text(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if "\\`*_{}[]<>#+-.!|~".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
pub(super) fn fenced(source: &str, language: &str) -> String {
    let longest = source.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(3.max(longest + 1));
    let language = if language.contains(['\n', '\r', '`']) {
        ""
    } else {
        language
    };
    format!(
        "{fence}{language}\n{source}{}{fence}",
        if source.ends_with('\n') { "" } else { "\n" }
    )
}
fn code_span(source: &str) -> String {
    let longest = source.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat((longest + 1).max(1));
    let pad = source.starts_with('`')
        || source.ends_with('`')
        || (source.starts_with(' ') && source.ends_with(' ') && source.chars().any(|c| c != ' '));
    if pad {
        format!("{fence} {source} {fence}")
    } else {
        format!("{fence}{source}{fence}")
    }
}
fn quote(source: &str) -> String {
    source
        .split('\n')
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
fn literal(node: &Value) -> String {
    if let Some(text) = node["text"].as_str() {
        return text.to_owned();
    }
    node["content"]
        .as_array()
        .map(|children| children.iter().map(literal).collect::<Vec<_>>().concat())
        .unwrap_or_default()
}
fn escape_url(value: &str) -> String {
    value
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace(' ', "%20")
        .replace('\n', "%0A")
        .replace('\r', "%0D")
}
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn wrap_mark(source: &str, delimiter: &str) -> String {
    let content = source.trim_matches(char::is_whitespace);
    if content.is_empty() {
        return source.to_owned();
    }
    let start = source.len() - source.trim_start_matches(char::is_whitespace).len();
    let end = start + content.len();
    format!(
        "{}{delimiter}{content}{delimiter}{}",
        &source[..start],
        &source[end..]
    )
}

fn table_cell(source: &str) -> String {
    let mut output = String::new();
    let mut slashes = 0_usize;
    for character in source.chars() {
        if character == '|' && slashes.is_multiple_of(2) {
            output.push('\\');
        }
        if character == '\n' {
            output.push_str("<br>");
        } else {
            output.push(character);
        }
        slashes = if character == '\\' { slashes + 1 } else { 0 };
    }
    output
}
#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Synthetic Markdown conversion contracts fail immediately"
)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn complex_tables_should_keep_nested_blocks_spans_and_unicode_visible() {
        let nodes = HashMap::new();
        let assets = HashMap::new();
        let anchors = HashMap::new();
        let mut renderer = MarkdownRenderer {
            current: "documents/a.md",
            project: "project",
            nodes: &nodes,
            anchors: &anchors,
            heading_offset: 0,
            block_anchors: true,
            assets: &assets,
            conversions: BTreeSet::new(),
        };
        let table = json!({"type":"table","content":[{"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":2,"rowspan":1},"content":[
            {"type":"paragraph","content":[{"type":"text","text":"Автор <safe>","marks":[{"type":"bold"}]}]},
            {"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Nested list"}]}]}]}
        ]}]}]});
        let output = renderer.render(&table).unwrap();
        for text in [
            "<table>",
            "colspan=\"2\"",
            "<strong>Автор &lt;safe&gt;</strong>",
            "<ul><li><p>Nested list</p></li></ul>",
        ] {
            assert!(output.contains(text), "{output}");
        }
        assert!(
            renderer
                .conversions
                .contains("complex-table-html-reading-view")
        );
    }

    #[test]
    fn document_links_should_rewrite_and_outside_selection_links_should_be_reported() {
        let nodes = HashMap::from([("selected".into(), "documents/Автор.md".into())]);
        let assets = HashMap::new();
        let anchors = HashMap::new();
        let mut renderer = MarkdownRenderer {
            current: "documents/a.md",
            project: "project",
            nodes: &nodes,
            anchors: &anchors,
            heading_offset: 0,
            block_anchors: true,
            assets: &assets,
            conversions: BTreeSet::new(),
        };
        assert_eq!(
            renderer
                .link_target("/projects/project/studio/selected#passage 😀")
                .unwrap(),
            Some("%D0%90%D0%B2%D1%82%D0%BE%D1%80.md#passage%20%F0%9F%98%80".into())
        );
        assert_eq!(
            renderer
                .link_target("writers-studio://documents/unselected")
                .unwrap(),
            None
        );
        assert!(
            renderer
                .conversions
                .contains("document-link-outside-export-scope")
        );
        assert_eq!(renderer.link_target("javascript:alert(1)").unwrap(), None);
    }

    #[test]
    fn opaque_image_locations_should_have_readable_fallbacks_without_invented_local_links() {
        let nodes = HashMap::new();
        let assets = HashMap::new();
        let anchors = HashMap::new();
        let mut renderer = MarkdownRenderer {
            current: "documents/a.md",
            project: "project",
            nodes: &nodes,
            anchors: &anchors,
            heading_offset: 0,
            block_anchors: true,
            assets: &assets,
            conversions: BTreeSet::new(),
        };
        let image = serde_json::json!({"type":"imageBlock","attrs":{"src":"file:///unrelated/local.png","caption":"Original caption"}});
        let output = renderer.render(&image).unwrap();
        assert!(output.contains("canonical companion"));
        assert!(output.contains("Original caption"));
        assert!(!output.contains("file:///"));
        assert!(
            renderer
                .conversions
                .contains("external-or-opaque-image-source-retained")
        );
    }

    #[test]
    fn table_pipes_whitespace_and_embedded_fences_should_not_break_reading_syntax() {
        assert_eq!(table_cell("Cell \\| value"), "Cell \\| value");
        assert_eq!(table_cell("raw|value"), "raw\\|value");
        assert_eq!(wrap_mark(" bold ", "**"), " **bold** ");
        let tick = char::from(96);
        let source = format!("{tick}{tick}{tick}\nbody");
        assert!(fenced(&source, "rust").starts_with(&format!("{tick}{tick}{tick}{tick}rust\n")));
        assert!(fenced("body", "bad\nlabel").starts_with(&format!("{tick}{tick}{tick}\n")));
    }
}
