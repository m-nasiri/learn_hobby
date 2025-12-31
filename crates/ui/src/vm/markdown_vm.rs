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

use std::collections::{HashMap, HashSet};
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

pub fn sanitize_html(html: &str) -> String {
    let tags: HashSet<&str> = [
        "p", "div", "span", "br", "em", "strong", "b", "i", "code", "pre", "blockquote", "ul",
        "ol", "li", "a",
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

pub fn looks_like_markdown(input: &str) -> bool {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("```") || lower.contains("**") || lower.contains("__") {
        return true;
    }

    if lower.contains("](") {
        return true;
    }

    for line in trimmed.lines() {
        let line = line.trim_start();
        if line.starts_with("# ")
            || line.starts_with("## ")
            || line.starts_with("- ")
            || line.starts_with("* ")
            || line.starts_with("> ")
        {
            return true;
        }
    }

    false
}

pub fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();

    for ch in input.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                let tag = tag_buf.trim().to_ascii_lowercase();
                if tag.starts_with("br")
                    || tag.starts_with("/p")
                    || tag.starts_with("p")
                    || tag.starts_with("/div")
                    || tag.starts_with("div")
                    || tag.starts_with("/li")
                    || tag.starts_with("li")
                    || tag.starts_with("/blockquote")
                    || tag.starts_with("blockquote")
                    || tag.starts_with("/pre")
                    || tag.starts_with("pre")
                {
                    out.push('\n');
                }
                tag_buf.clear();
            } else {
                tag_buf.push(ch);
            }
            continue;
        }

        if ch == '<' {
            in_tag = true;
            tag_buf.clear();
            continue;
        }

        out.push(ch);
    }

    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
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


#[cfg(test)]
mod tests {
    use super::{
        html_to_markdown, looks_like_html, looks_like_markdown, markdown_to_html,
        normalize_markdown, strip_html_tags,
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
    fn markdown_detection_matches_common_patterns() {
        assert!(looks_like_markdown("**bold**"));
        assert!(looks_like_markdown("- list item"));
        assert!(looks_like_markdown("> quote"));
        assert!(looks_like_markdown("[link](https://example.com)"));
        assert!(!looks_like_markdown("Plain sentence."));
    }

    #[test]
    fn strip_html_tags_removes_markup() {
        let text = strip_html_tags("<p>Hello<br>World</p>");
        assert_eq!(text.trim(), "Hello\nWorld");
    }
}
