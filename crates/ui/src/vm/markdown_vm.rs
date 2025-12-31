#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownField {
    Front,
    Back,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownAction {
    Bold,
    Italic,
    Link,
    Quote,
    BulletList,
    NumberedList,
    Code,
    CodeBlock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasteOffer {
    pub target: MarkdownField,
    pub html: String,
    pub text: String,
}

pub fn markdown_to_html(input: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);

    let parser = pulldown_cmark::Parser::new_ext(input, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    sanitize_html(&html)
}

pub fn html_to_markdown(input: &str) -> String {
    let markdown = html2md::parse_html(input);
    normalize_markdown(&markdown)
}

fn sanitize_html(html: &str) -> String {
    let tags: HashSet<&str> = [
        "p", "br", "em", "strong", "code", "pre", "blockquote", "ul", "ol", "li", "a",
    ]
    .into_iter()
    .collect();

    let mut attributes: HashMap<&str, HashSet<&str>> = HashMap::new();
    attributes.insert("a", ["href"].into_iter().collect());

    ammonia::Builder::new()
        .tags(tags)
        .tag_attributes(attributes)
        .clean(html)
        .to_string()
}

pub fn looks_like_html(input: &str) -> bool {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    let Some(start) = lower.find('<') else {
        return false;
    };
    if !lower[start + 1..].contains('>') {
        return false;
    }

    let tags = [
        "<!doctype",
        "<html",
        "<body",
        "<p",
        "<div",
        "<span",
        "<br",
        "<a ",
        "<img",
    ];
    tags.iter().any(|tag| lower.contains(tag))
}

pub fn normalize_markdown(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = Vec::new();
    let mut blank_streak = 0usize;

    for line in normalized.split('\n') {
        let trimmed = line.trim_end_matches([' ', '\t']).to_string();
        if trimmed.is_empty() {
            blank_streak += 1;
            if blank_streak > 1 {
                continue;
            }
        } else {
            blank_streak = 0;
        }
        lines.push(trimmed);
    }

    lines.join("\n")
}

pub fn apply_markdown_action(
    text: &str,
    action: MarkdownAction,
    selection: Option<SelectionRange>,
) -> (String, Option<SelectionRange>) {
    let selection = selection.unwrap_or_else(|| SelectionRange {
        start: utf16_len(text),
        end: utf16_len(text),
    });
    let start = utf16_to_byte_idx(text, selection.start);
    let end = utf16_to_byte_idx(text, selection.end);
    let (start, end) = if start <= end { (start, end) } else { (end, start) };
    let has_selection = start != end;

    let (next, selection_bytes) = match action {
        MarkdownAction::Bold => wrap(text, start, end, "**", "**", has_selection),
        MarkdownAction::Italic => wrap(text, start, end, "*", "*", has_selection),
        MarkdownAction::Code => wrap(text, start, end, "`", "`", has_selection),
        MarkdownAction::Link => apply_link(text, start, end, has_selection),
        MarkdownAction::Quote => prefix_lines(text, start, end, "> ", has_selection),
        MarkdownAction::BulletList => prefix_lines(text, start, end, "- ", has_selection),
        MarkdownAction::NumberedList => prefix_lines(text, start, end, "1. ", has_selection),
        MarkdownAction::CodeBlock => wrap_block(text, start, end, has_selection),
    };

    let selection = selection_bytes.map(|(sel_start, sel_end)| SelectionRange {
        start: byte_to_utf16_idx(&next, sel_start),
        end: byte_to_utf16_idx(&next, sel_end),
    });

    (next, selection)
}

fn wrap(
    text: &str,
    start: usize,
    end: usize,
    prefix: &str,
    suffix: &str,
    has_selection: bool,
) -> (String, Option<(usize, usize)>) {
    let mut output = String::with_capacity(text.len() + prefix.len() + suffix.len());
    output.push_str(&text[..start]);
    output.push_str(prefix);
    if has_selection {
        output.push_str(&text[start..end]);
        output.push_str(suffix);
        output.push_str(&text[end..]);
        let sel_start = start + prefix.len();
        let sel_end = sel_start + (end - start);
        return (output, Some((sel_start, sel_end)));
    }
    output.push_str(suffix);
    output.push_str(&text[start..]);
    let cursor = start + prefix.len();
    (output, Some((cursor, cursor)))
}

fn apply_link(text: &str, start: usize, end: usize, has_selection: bool) -> (String, Option<(usize, usize)>) {
    if has_selection {
        let selected = &text[start..end];
        let mut output = String::with_capacity(text.len() + selected.len() + 7);
        output.push_str(&text[..start]);
        output.push('[');
        output.push_str(selected);
        output.push_str("](url)");
        output.push_str(&text[end..]);
        let url_start = start + 2 + selected.len();
        let url_end = url_start + 3;
        return (output, Some((url_start, url_end)));
    }

    let mut output = String::with_capacity(text.len() + 4);
    output.push_str(&text[..start]);
    output.push_str("[]()");
    output.push_str(&text[start..]);
    let cursor = start + 1;
    (output, Some((cursor, cursor)))
}

fn prefix_lines(
    text: &str,
    start: usize,
    end: usize,
    prefix: &str,
    has_selection: bool,
) -> (String, Option<(usize, usize)>) {
    if !has_selection {
        let mut output = String::with_capacity(text.len() + prefix.len());
        output.push_str(&text[..start]);
        output.push_str(prefix);
        output.push_str(&text[start..]);
        let cursor = start + prefix.len();
        return (output, Some((cursor, cursor)));
    }

    let selected = &text[start..end];
    let mut prefixed = String::new();
    for (idx, line) in selected.split('\n').enumerate() {
        if idx > 0 {
            prefixed.push('\n');
        }
        prefixed.push_str(prefix);
        prefixed.push_str(line);
    }

    let mut output = String::with_capacity(text.len() + prefixed.len());
    output.push_str(&text[..start]);
    output.push_str(&prefixed);
    output.push_str(&text[end..]);
    let sel_end = start + prefixed.len();
    (output, Some((start, sel_end)))
}

fn wrap_block(text: &str, start: usize, end: usize, has_selection: bool) -> (String, Option<(usize, usize)>) {
    let prefix = "```\n";
    let suffix = "\n```";
    let mut output = String::with_capacity(text.len() + prefix.len() + suffix.len());
    output.push_str(&text[..start]);
    output.push_str(prefix);
    if has_selection {
        output.push_str(&text[start..end]);
        output.push_str(suffix);
        output.push_str(&text[end..]);
        let sel_start = start + prefix.len();
        let sel_end = sel_start + (end - start);
        return (output, Some((sel_start, sel_end)));
    }
    output.push_str(suffix);
    output.push_str(&text[start..]);
    let cursor = start + prefix.len();
    (output, Some((cursor, cursor)))
}

fn utf16_len(text: &str) -> usize {
    text.chars().map(|ch| ch.len_utf16()).sum()
}

fn utf16_to_byte_idx(text: &str, utf16_idx: usize) -> usize {
    let mut count = 0usize;
    for (byte_idx, ch) in text.char_indices() {
        let next = count + ch.len_utf16();
        if next > utf16_idx {
            return byte_idx;
        }
        count = next;
    }
    text.len()
}

fn byte_to_utf16_idx(text: &str, byte_idx: usize) -> usize {
    let mut count = 0usize;
    for (idx, ch) in text.char_indices() {
        if idx >= byte_idx {
            break;
        }
        count += ch.len_utf16();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::{
        MarkdownAction, SelectionRange, apply_markdown_action, html_to_markdown, looks_like_html,
        markdown_to_html, normalize_markdown,
    };

    #[test]
    fn html_detection_requires_structure_and_known_tags() {
        assert!(looks_like_html("<p>Hello</p>"));
        assert!(looks_like_html("  <div class=\"x\">Hi</div>"));
        assert!(looks_like_html("<span>Ok</span>"));
        assert!(looks_like_html("<br>"));
        assert!(looks_like_html("<a href=\"/\">Link</a>"));
        assert!(looks_like_html("<img src=\"x\"/>"));
        assert!(looks_like_html("<!doctype html><html></html>"));

        assert!(!looks_like_html("2 < 3 > 1"));
        assert!(!looks_like_html("<math>x</math>"));
        assert!(!looks_like_html("<notatag"));
        assert!(!looks_like_html("plain text"));
    }

    #[test]
    fn normalize_markdown_trims_and_collapses_blank_lines() {
        let input = "Line one  \r\n\r\n\r\nLine two\t\r\n\r\n";
        let output = normalize_markdown(input);
        assert_eq!(output, "Line one\n\nLine two\n");
    }

    #[test]
    fn markdown_to_html_sanitizes_links() {
        let html = markdown_to_html("[Link](javascript:alert(1))");
        assert!(html.contains("Link"));
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn html_to_markdown_normalizes_output() {
        let markdown = html_to_markdown("<p>Hello</p>\r\n<p>World</p>");
        assert_eq!(markdown, "Hello\n\nWorld\n");
    }

    #[test]
    fn apply_markdown_action_wraps_selection() {
        let (out, selection) = apply_markdown_action(
            "Hello",
            MarkdownAction::Bold,
            Some(SelectionRange { start: 0, end: 5 }),
        );
        assert_eq!(out, "**Hello**");
        assert_eq!(selection, Some(SelectionRange { start: 2, end: 7 }));
    }
}
use std::collections::{HashMap, HashSet};
