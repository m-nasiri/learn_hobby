use dioxus::document::eval;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct SelectionSnapshot {
    pub html: String,
    pub text: String,
}

const SELECTION_READ_SCRIPT_TEMPLATE: &str = r#"
    const el = document.getElementById("{element_id}");
    if (!el) {{ return {{ html: "", text: "" }}; }}
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || sel.isCollapsed) {{
        return {{ html: "", text: "" }};
    }}
    const range = sel.getRangeAt(0);
    if (!el.contains(range.commonAncestorContainer)) {{
        return {{ html: "", text: "" }};
    }}
    const fragment = range.cloneContents();
    const wrapper = document.createElement("div");
    wrapper.appendChild(fragment);
    return {{ html: wrapper.innerHTML || "", text: sel.toString() || "" }};
"#;

pub async fn read_selection_snapshot(element_id: &str) -> Option<SelectionSnapshot> {
    let script = SELECTION_READ_SCRIPT_TEMPLATE.replace("{element_id}", element_id);
    eval(&script).join::<SelectionSnapshot>().await.ok()
}

pub async fn read_editable_html(element_id: &str) -> Option<String> {
    let script = read_editable_html_script(element_id);
    eval(&script).join::<String>().await.ok()
}

pub async fn set_editable_html(element_id: &str, html: &str) {
    let script = set_editable_html_script(element_id, html);
    let _ = eval(&script).await;
}

pub async fn replace_selection_or_all(element_id: &str, html: &str) {
    let script = replace_selection_or_all_script(element_id, html);
    let _ = eval(&script).await;
}

pub async fn write_clipboard(html: &str, text: &str) {
    let script = write_clipboard_script(html, text);
    let _ = eval(&script).await;
}

pub async fn attach_rich_paste_handler(element_id: &str) {
    let script = attach_rich_paste_handler_script(element_id);
    let _ = eval(&script).await;
}

pub async fn read_selected_link_href(element_id: &str) -> Option<String> {
    let script = read_selected_link_href_script(element_id);
    eval(&script).join::<String>().await.ok()
}

pub async fn read_link_href_at_point(element_id: &str, x: f64, y: f64) -> Option<String> {
    let script = read_link_href_at_point_script(element_id, x, y);
    let href = eval(&script).join::<String>().await.ok()?;
    let trimmed = href.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub async fn save_selection_range(element_id: &str) {
    let script = save_selection_range_script(element_id);
    let _ = eval(&script).await;
}

pub async fn apply_link(element_id: &str, url: &str) {
    let script = apply_link_script(element_id, url);
    let _ = eval(&script).await;
}

pub async fn remove_link(element_id: &str) {
    let script = remove_link_script(element_id);
    let _ = eval(&script).await;
}

pub fn set_block_dir_script(element_id: &str, dir: &str) -> String {
    let dir_literal = js_string_literal(dir);
    let align_literal = if dir == "rtl" {
        "\"right\""
    } else {
        "\"left\""
    };
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        const sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) {{ return; }}
        const blockTags = new Set(["P", "DIV", "LI", "BLOCKQUOTE", "PRE"]);
        let node = sel.anchorNode;
        if (node && node.nodeType === Node.TEXT_NODE) {{
            node = node.parentElement;
        }}
        while (node && node !== el && !blockTags.has(node.tagName)) {{
            node = node.parentElement;
        }}
        if (node === el) {{
            document.execCommand("formatBlock", false, "div");
            node = sel.anchorNode;
            if (node && node.nodeType === Node.TEXT_NODE) {{
                node = node.parentElement;
            }}
            while (node && node !== el && !blockTags.has(node.tagName)) {{
                node = node.parentElement;
            }}
        }}
        if (node && node !== el) {{
            node.setAttribute("dir", {dir_literal});
            node.style.textAlign = {align_literal};
            node.style.unicodeBidi = "plaintext";
        }}
        "#
    )
}

pub fn wrap_selection_script(element_id: &str, tag: &str, inner_tag: Option<&str>) -> String {
    let (before, after) = match inner_tag {
        Some(inner_tag) => (
            format!("<{tag}><{inner_tag}>"),
            format!("</{inner_tag}></{tag}>"),
        ),
        None => (format!("<{tag}>"), format!("</{tag}>")),
    };
    let before_literal = js_string_literal(&before);
    let after_literal = js_string_literal(&after);
    let marker_literal = js_string_literal(r#"<span data-caret="true"></span>"#);
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        const sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) {{
            document.execCommand("insertHTML", false, {before_literal} + {after_literal});
            return;
        }}
        const range = sel.getRangeAt(0);
        if (!el.contains(range.commonAncestorContainer)) {{
            return;
        }}
        const container = document.createElement("div");
        container.appendChild(range.cloneContents());
        const selectedHtml = container.innerHTML;
        const html = selectedHtml
            ? {before_literal} + selectedHtml + {after_literal}
            : {before_literal} + {marker_literal} + {after_literal};
        document.execCommand("insertHTML", false, html);
        const marker = el.querySelector('[data-caret="true"]');
        if (marker) {{
            const newRange = document.createRange();
            newRange.setStartAfter(marker);
            newRange.collapse(true);
            const sel2 = window.getSelection();
            sel2.removeAllRanges();
            sel2.addRange(newRange);
            marker.remove();
        }}
        "#
    )
}

pub fn exec_command_script(element_id: &str, command: &str, value: Option<&str>) -> String {
    let command_literal = js_string_literal(command);
    let value_literal = value.map_or_else(|| "null".to_string(), js_string_literal);
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        document.execCommand({command_literal}, false, {value_literal});
        "#
    )
}

fn read_editable_html_script(element_id: &str) -> String {
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return ""; }}
        const tooltip = "Cmd/Ctrl+click to open link";
        const anchors = el.querySelectorAll("a");
        for (const anchor of anchors) {{
            const dataHref = (anchor.getAttribute("data-href") || "").trim();
            const href = (anchor.getAttribute("href") || "").trim();
            const raw = dataHref || href;
            if (!raw) {{
                continue;
            }}
            const lower = raw.toLowerCase();
            if (lower.startsWith("javascript:") || lower.startsWith("data:")) {{
                anchor.removeAttribute("href");
                anchor.removeAttribute("data-href");
                continue;
            }}
            anchor.setAttribute("data-href", raw);
            anchor.removeAttribute("href");
            const title = (anchor.getAttribute("title") || "").trim();
            if (!title) {{
                anchor.setAttribute("title", tooltip);
            }}
        }}
        const clone = el.cloneNode(true);
        const cloneAnchors = clone.querySelectorAll("a");
        for (const anchor of cloneAnchors) {{
            const dataHref = (anchor.getAttribute("data-href") || "").trim();
            if (dataHref) {{
                anchor.setAttribute("href", dataHref);
                anchor.removeAttribute("data-href");
            }}
            const title = (anchor.getAttribute("title") || "").trim();
            if (title === tooltip) {{
                anchor.removeAttribute("title");
            }}
        }}
        return clone.innerHTML;
        "#
    )
}

fn set_editable_html_script(element_id: &str, html: &str) -> String {
    let html_literal = js_string_literal(html);
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (el) {{
            el.innerHTML = {html_literal};
            const tooltip = "Cmd/Ctrl+click to open link";
            const anchors = el.querySelectorAll("a");
            for (const anchor of anchors) {{
                const dataHref = (anchor.getAttribute("data-href") || "").trim();
                const href = (anchor.getAttribute("href") || "").trim();
                const raw = dataHref || href;
                if (!raw) {{
                    continue;
                }}
                const lower = raw.toLowerCase();
                if (lower.startsWith("javascript:") || lower.startsWith("data:")) {{
                    anchor.removeAttribute("href");
                    anchor.removeAttribute("data-href");
                    continue;
                }}
                anchor.setAttribute("data-href", raw);
                anchor.removeAttribute("href");
                const title = (anchor.getAttribute("title") || "").trim();
                if (!title) {{
                    anchor.setAttribute("title", tooltip);
                }}
            }}
        }}
        "#
    )
}

fn replace_selection_or_all_script(element_id: &str, html: &str) -> String {
    let html_literal = js_string_literal(html);
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        const sel = window.getSelection();
        const range = sel && sel.rangeCount > 0 ? sel.getRangeAt(0) : null;
        const isInside = range ? el.contains(range.commonAncestorContainer) : false;
        if (isInside && sel && !sel.isCollapsed) {{
            document.execCommand("insertHTML", false, {html_literal});
            return;
        }}
        document.execCommand("selectAll", false, null);
        document.execCommand("insertHTML", false, {html_literal});
        "#
    )
}

fn write_clipboard_script(html: &str, text: &str) -> String {
    let html_literal = js_string_literal(html);
    let text_literal = js_string_literal(text);
    format!(
        r#"
        const html = {html_literal};
        const text = {text_literal};
        try {{
            if (navigator.clipboard && navigator.clipboard.write) {{
                const item = new ClipboardItem({{
                    "text/html": new Blob([html], {{ type: "text/html" }}),
                    "text/plain": new Blob([text], {{ type: "text/plain" }})
                }});
                await navigator.clipboard.write([item]);
                return;
            }}
        }} catch (_) {{}}
        try {{
            if (navigator.clipboard && navigator.clipboard.writeText) {{
                await navigator.clipboard.writeText(text);
            }}
        }} catch (_) {{}}
        "#
    )
}

fn read_selected_link_href_script(element_id: &str) -> String {
    format!(
        r#"
        if (!window.__learnStoredSelection) {{
            window.__learnStoredSelection = {{}};
        }}
        window.__learnGetStoredRange = function (el) {{
            const stored = window.__learnStoredSelection[el.id];
            if (!stored) return null;
            if (!el.contains(stored.commonAncestorContainer)) return null;
            return stored;
        }};
        const el = document.getElementById("{element_id}");
        if (!el) {{ return ""; }}
        const sel = window.getSelection();
        let range = null;
        if (sel && sel.rangeCount > 0) {{
            const candidate = sel.getRangeAt(0);
            if (el.contains(candidate.commonAncestorContainer)) {{
                range = candidate;
            }}
        }}
        if (!range) {{
            range = window.__learnGetStoredRange(el);
        }}
        if (!range) {{ return ""; }}
        let node = range.startContainer;
        if (node && node.nodeType === Node.TEXT_NODE) {{
            node = node.parentElement;
        }}
        while (node && node !== el) {{
            if (node.tagName === "A") {{
                const dataHref = (node.getAttribute("data-href") || "").trim();
                if (dataHref) {{
                    return dataHref;
                }}
                const href = (node.getAttribute("href") || "").trim();
                return href;
            }}
            node = node.parentElement;
        }}
        return "";
        "#
    )
}

fn apply_link_script(element_id: &str, url: &str) -> String {
    let url_literal = js_string_literal(url);
    format!(
        r#"
        if (!window.__learnStoredSelection) {{
            window.__learnStoredSelection = {{}};
        }}
        window.__learnGetStoredRange = function (el) {{
            const stored = window.__learnStoredSelection[el.id];
            if (!stored) return null;
            if (!el.contains(stored.commonAncestorContainer)) return null;
            return stored;
        }};
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        const sel = window.getSelection();
        const stored = window.__learnGetStoredRange(el);
        if (stored && sel) {{
            sel.removeAllRanges();
            sel.addRange(stored);
        }}
        if (!sel || sel.rangeCount === 0) {{
            return;
        }}
        const range = sel.getRangeAt(0);
        if (!el.contains(range.commonAncestorContainer)) {{
            return;
        }}
        let node = sel.anchorNode;
        if (node && node.nodeType === Node.TEXT_NODE) {{
            node = node.parentElement;
        }}
        while (node && node !== el) {{
            if (node.tagName === "A") {{
                node.setAttribute("data-href", {url_literal});
                node.removeAttribute("href");
                return;
            }}
            node = node.parentElement;
        }}
        if (sel.isCollapsed) {{
            const url = {url_literal};
            const link = document.createElement("a");
            link.setAttribute("data-href", url);
            link.textContent = url;
            document.execCommand("insertHTML", false, link.outerHTML);
        }} else {{
            document.execCommand("createLink", false, {url_literal});
            const anchors = el.querySelectorAll("a");
            for (const anchor of anchors) {{
                const dataHref = (anchor.getAttribute("data-href") || "").trim();
                const href = (anchor.getAttribute("href") || "").trim();
                const raw = dataHref || href;
                if (!raw) {{
                    continue;
                }}
                const lower = raw.toLowerCase();
                if (lower.startsWith("javascript:") || lower.startsWith("data:")) {{
                    anchor.removeAttribute("href");
                    anchor.removeAttribute("data-href");
                    continue;
                }}
                anchor.setAttribute("data-href", raw);
                anchor.removeAttribute("href");
            }}
        }}
        "#
    )
}

fn remove_link_script(element_id: &str) -> String {
    format!(
        r#"
        if (!window.__learnStoredSelection) {{
            window.__learnStoredSelection = {{}};
        }}
        window.__learnGetStoredRange = function (el) {{
            const stored = window.__learnStoredSelection[el.id];
            if (!stored) return null;
            if (!el.contains(stored.commonAncestorContainer)) return null;
            return stored;
        }};
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        const sel = window.getSelection();
        const stored = window.__learnGetStoredRange(el);
        if (stored && sel) {{
            sel.removeAllRanges();
            sel.addRange(stored);
        }}
        if (!sel || sel.rangeCount === 0) {{ return; }}
        const range = sel.getRangeAt(0);
        if (!el.contains(range.commonAncestorContainer)) {{ return; }}
        let node = sel.anchorNode;
        if (node && node.nodeType === Node.TEXT_NODE) {{
            node = node.parentElement;
        }}
        while (node && node !== el) {{
            if (node.tagName === "A") {{
                const parent = node.parentNode;
                if (parent) {{
                    while (node.firstChild) {{
                        parent.insertBefore(node.firstChild, node);
                    }}
                    parent.removeChild(node);
                }}
                return;
            }}
            node = node.parentElement;
        }}
        document.execCommand("unlink", false, null);
        "#
    )
}

fn attach_rich_paste_handler_script(element_id: &str) -> String {
    format!(
        r###"
        if (!window.__learnRichPasteInit) {{
            window.__learnRichPasteInit = true;
            window.__learnLooksLikeHtml = function (input) {{
                const trimmed = input.trimStart();
                if (!trimmed) return false;
                const lower = trimmed.toLowerCase();
                const start = lower.indexOf("<");
                if (start === -1) return false;
                if (lower.slice(start + 1).indexOf(">") === -1) return false;
                const tags = ["<!doctype", "<html", "<body", "<p", "<div", "<span", "<br", "<a ", "<img"];
                return tags.some(tag => lower.includes(tag));
            }};
            window.__learnLooksLikeMarkdown = function (input) {{
                const trimmed = input.trimStart();
                if (!trimmed) return false;
                const lower = trimmed.toLowerCase();
                if (lower.includes("```") || lower.includes("**") || lower.includes("__")) return true;
                if (lower.includes("](")) return true;
                const lines = trimmed.split(/\\r?\\n/);
                for (const line of lines) {{
                    const check = line.trimStart();
                    if (
                        check.startsWith("# ") ||
                        check.startsWith("## ") ||
                        check.startsWith("- ") ||
                        check.startsWith("* ") ||
                        check.startsWith("> ")
                    ) {{
                        return true;
                    }}
                }}
                return false;
            }};
            window.__learnEscapeHtml = function (value) {{
                return value
                    .replace(/&/g, "&amp;")
                    .replace(/</g, "&lt;")
                    .replace(/>/g, "&gt;")
                    .replace(/\"/g, "&quot;")
                    .replace(/'/g, "&#39;");
            }};
            window.__learnInlineMarkdown = function (line) {{
                let text = window.__learnEscapeHtml(line);
                text = text.replace(/`([^`]+)`/g, "<code>$1</code>");
                text = text.replace(/\\*\\*([^*]+)\\*\\*/g, "<strong>$1</strong>");
                text = text.replace(/__([^_]+)__/g, "<strong>$1</strong>");
                text = text.replace(/\\*([^*]+)\\*/g, "<em>$1</em>");
                text = text.replace(/_([^_]+)_/g, "<em>$1</em>");
                return text;
            }};
            window.__learnMarkdownToHtml = function (input) {{
                const lines = input.replace(/\\r\\n/g, "\\n").split("\\n");
                let html = "";
                let inUl = false;
                let inOl = false;
                const closeLists = () => {{
                    if (inUl) {{
                        html += "</ul>";
                        inUl = false;
                    }}
                    if (inOl) {{
                        html += "</ol>";
                        inOl = false;
                    }}
                }};
                for (const rawLine of lines) {{
                    const line = rawLine.trimEnd();
                    if (/^\\s*[-*+]\\s+/.test(line)) {{
                        if (!inUl) {{
                            closeLists();
                            html += "<ul>";
                            inUl = true;
                        }}
                        html += "<li>" + window.__learnInlineMarkdown(line.replace(/^\\s*[-*+]\\s+/, "")) + "</li>";
                        continue;
                    }}
                    if (/^\\s*\\d+\\.\\s+/.test(line)) {{
                        if (!inOl) {{
                            closeLists();
                            html += "<ol>";
                            inOl = true;
                        }}
                        html += "<li>" + window.__learnInlineMarkdown(line.replace(/^\\s*\\d+\\.\\s+/, "")) + "</li>";
                        continue;
                    }}
                    closeLists();
                    if (line.trim() === "") {{
                        html += "<br>";
                        continue;
                    }}
                    if (/^\\s*#{{1,6}}\\s+/.test(line)) {{
                        const headingText = line.replace(/^\\s*#{{1,6}}\\s+/, "");
                        html += "<p><strong>" + window.__learnInlineMarkdown(headingText) + "</strong></p>";
                        continue;
                    }}
                    if (/^\\s*>\\s+/.test(line)) {{
                        const quoteText = line.replace(/^\\s*>\\s+/, "");
                        html += "<blockquote>" + window.__learnInlineMarkdown(quoteText) + "</blockquote>";
                        continue;
                    }}
                    html += "<p>" + window.__learnInlineMarkdown(line) + "</p>";
                }}
                closeLists();
                return html;
            }};
            window.__learnSanitizeHtml = function (html) {{
                const template = document.createElement("template");
                template.innerHTML = html;
                const allowedTags = new Set([
                    "p", "div", "span", "br", "em", "strong", "b", "i", "code", "pre",
                    "blockquote", "ul", "ol", "li", "a"
                ]);
                const allowedAttrs = {{
                    a: new Set(["href"])
                }};
                const cleanNode = function (node) {{
                    if (node.nodeType === Node.ELEMENT_NODE) {{
                        const tag = node.tagName.toLowerCase();
                        if (!allowedTags.has(tag)) {{
                            const parent = node.parentNode;
                            if (parent) {{
                                const children = Array.from(node.childNodes);
                                for (const child of children) {{
                                    parent.insertBefore(child, node);
                                    cleanNode(child);
                                }}
                                parent.removeChild(node);
                                return;
                            }}
                        }}
                        const attrs = Array.from(node.attributes || []);
                        for (const attr of attrs) {{
                            const allowForTag = allowedAttrs[tag];
                            if (!allowForTag || !allowForTag.has(attr.name)) {{
                                node.removeAttribute(attr.name);
                            }}
                        }}
                        if (tag === "a") {{
                            const href = (node.getAttribute("href") || "").trim();
                            const lowerHref = href.toLowerCase();
                            if (lowerHref.startsWith("javascript:") || lowerHref.startsWith("data:")) {{
                                node.removeAttribute("href");
                            }}
                        }}
                    }}
                    const children = Array.from(node.childNodes || []);
                    for (const child of children) {{
                        cleanNode(child);
                    }}
                }};
                cleanNode(template.content);
                return template.innerHTML || "";
            }};
            window.__learnAttachRichPaste = function (el) {{
                if (!el || el.dataset.learnRichPaste === "true") return;
                el.dataset.learnRichPaste = "true";
                el.addEventListener("paste", function (event) {{
                    if (!event.clipboardData) {{
                        return;
                    }}
                    const html = event.clipboardData.getData("text/html") || "";
                    const text = event.clipboardData.getData("text/plain") || "";
                    if (!html && !text) {{
                        return;
                    }}
                    let insertHtml = "";
                    if (html && html.trim()) {{
                        insertHtml = window.__learnSanitizeHtml(html);
                    }} else if (window.__learnLooksLikeHtml(text)) {{
                        insertHtml = window.__learnSanitizeHtml(text);
                    }} else if (window.__learnLooksLikeMarkdown(text)) {{
                        insertHtml = window.__learnSanitizeHtml(window.__learnMarkdownToHtml(text));
                    }} else {{
                        event.preventDefault();
                        document.execCommand("insertText", false, text);
                        return;
                    }}
                    event.preventDefault();
                    document.execCommand("insertHTML", false, insertHtml);
                }}, {{ passive: false }});
            }};
        }}
        window.__learnAttachRichPaste(document.getElementById("{element_id}"));
        "###,
        element_id = element_id
    )
}

fn save_selection_range_script(element_id: &str) -> String {
    format!(
        r#"
        if (!window.__learnStoredSelection) {{
            window.__learnStoredSelection = {{}};
        }}
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        const sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) {{ return; }}
        const range = sel.getRangeAt(0);
        if (!el.contains(range.commonAncestorContainer)) {{ return; }}
        window.__learnStoredSelection[el.id] = range.cloneRange();
        "#
    )
}

fn read_link_href_at_point_script(element_id: &str, x: f64, y: f64) -> String {
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return ""; }}
        const target = document.elementFromPoint({x}, {y});
        if (!target) {{ return ""; }}
        let link = target;
        if (link && link.nodeType === Node.TEXT_NODE) {{
            link = link.parentElement;
        }}
        if (link && link.closest) {{
            link = link.closest("a");
        }}
        if (!link || !el.contains(link)) {{ return ""; }}
        const dataHref = (link.getAttribute("data-href") || "").trim();
        if (dataHref) {{
            return dataHref;
        }}
        const href = (link.getAttribute("href") || "").trim();
        return href;
        "#,
        element_id = element_id,
        x = x,
        y = y
    )
}

fn js_string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
