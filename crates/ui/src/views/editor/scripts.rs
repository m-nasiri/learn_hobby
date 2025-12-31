use dioxus::document::eval;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ClipboardSnapshot {
    pub html: String,
    pub text: String,
}

const CLIPBOARD_READ_SCRIPT: &str = r#"
    let html = "";
    let text = "";
    try {
        if (navigator.clipboard && navigator.clipboard.read) {
            const items = await navigator.clipboard.read();
            for (const item of items) {
                if (!html && item.types.includes("text/html")) {
                    const blob = await item.getType("text/html");
                    html = await blob.text();
                }
                if (!text && item.types.includes("text/plain")) {
                    const blob = await item.getType("text/plain");
                    text = await blob.text();
                }
            }
        }
        if (!text && navigator.clipboard && navigator.clipboard.readText) {
            text = await navigator.clipboard.readText();
        }
    } catch (_) {}
    return { html, text };
"#;

pub async fn read_clipboard_snapshot() -> Option<ClipboardSnapshot> {
    eval(CLIPBOARD_READ_SCRIPT).join::<ClipboardSnapshot>().await.ok()
}

pub async fn read_editable_html(element_id: &str) -> Option<String> {
    let script = read_editable_html_script(element_id);
    eval(&script).join::<String>().await.ok()
}

pub async fn set_editable_html(element_id: &str, html: &str) {
    let script = set_editable_html_script(element_id, html);
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

pub fn insert_html_script(element_id: &str, html: &str) -> String {
    let html_literal = js_string_literal(html);
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        document.execCommand("insertHTML", false, {html_literal});
        "#
    )
}

pub fn insert_text_script(element_id: &str, text: &str) -> String {
    let text_literal = js_string_literal(text);
    let html_literal = js_string_literal(&escape_html(text));
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (!el) {{ return; }}
        el.focus();
        if (!document.execCommand("insertText", false, {text_literal})) {{
            document.execCommand("insertHTML", false, {html_literal});
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
        return el ? el.innerHTML : "";
        "#
    )
}

fn set_editable_html_script(element_id: &str, html: &str) -> String {
    let html_literal = js_string_literal(html);
    format!(
        r#"
        const el = document.getElementById("{element_id}");
        if (el) {{ el.innerHTML = {html_literal}; }}
        "#
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

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
