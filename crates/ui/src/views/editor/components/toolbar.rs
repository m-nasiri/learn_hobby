use dioxus::prelude::*;

use crate::vm::{MarkdownAction, MarkdownField};

#[component]
pub fn EditorFormatToolbar(
    field: MarkdownField,
    disabled: bool,
    on_format: Callback<(MarkdownField, MarkdownAction)>,
    on_block_dir: Callback<(MarkdownField, String)>,
) -> Element {
    rsx! {
        div { class: "editor-md-toolbar",
            div { class: "editor-md-toolbar-group",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Bold",
                    aria_label: "Bold",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::Bold));
                    },
                    span { class: "editor-md-toolbar-glyph", "B" }
                }
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Italic",
                    aria_label: "Italic",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::Italic));
                    },
                    span {
                        class: "editor-md-toolbar-glyph editor-md-toolbar-glyph--italic",
                        "I"
                    }
                }
            }
            div { class: "editor-md-toolbar-separator" }
            div { class: "editor-md-toolbar-group",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Quote",
                    aria_label: "Quote",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::Quote));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        path { d: "M7 9h3v4H8l1 3" }
                        path { d: "M13 9h3v4h-2l1 3" }
                    }
                }
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Bulleted list",
                    aria_label: "Bulleted list",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::BulletList));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        circle { cx: "6", cy: "8", r: "1.2" }
                        circle { cx: "6", cy: "12", r: "1.2" }
                        circle { cx: "6", cy: "16", r: "1.2" }
                        line { x1: "10", y1: "8", x2: "19", y2: "8" }
                        line { x1: "10", y1: "12", x2: "19", y2: "12" }
                        line { x1: "10", y1: "16", x2: "19", y2: "16" }
                    }
                }
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Numbered list",
                    aria_label: "Numbered list",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::NumberedList));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        rect { x: "4.6", y: "6.6", width: "2.8", height: "2.8", rx: "0.6" }
                        rect { x: "4.6", y: "10.6", width: "2.8", height: "2.8", rx: "0.6" }
                        rect { x: "4.6", y: "14.6", width: "2.8", height: "2.8", rx: "0.6" }
                        line { x1: "10", y1: "8", x2: "19", y2: "8" }
                        line { x1: "10", y1: "12", x2: "19", y2: "12" }
                        line { x1: "10", y1: "16", x2: "19", y2: "16" }
                    }
                }
            }
            div { class: "editor-md-toolbar-separator" }
            div { class: "editor-md-toolbar-group",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Add image",
                    aria_label: "Add image",
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        rect { x: "4", y: "5", width: "16", height: "14", rx: "2" }
                        path { d: "M7.5 14l2.5-3 3.5 4 2.5-3 3 4" }
                        circle { cx: "9", cy: "9", r: "1.2" }
                    }
                }
            }
            div { class: "editor-md-toolbar-separator" }
            div { class: "editor-md-toolbar-group",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Link",
                    aria_label: "Link",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::Link));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        path { d: "M9.5 14.5l5-5" }
                        path { d: "M8.7 15.3a4 4 0 0 1 0-5.7l2.1-2.1a4 4 0 1 1 5.7 5.7l-1 1" }
                        path { d: "M15.3 8.7a4 4 0 0 1 0 5.7l-2.1 2.1a4 4 0 1 1-5.7-5.7l1-1" }
                    }
                }
            }
            div { class: "editor-md-toolbar-separator" }
            div { class: "editor-md-toolbar-group",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Inline code",
                    aria_label: "Inline code",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::Code));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        path { d: "M9 8L5 12l4 4" }
                        path { d: "M15 8l4 4-4 4" }
                    }
                }
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Code block",
                    aria_label: "Code block",
                    onclick: move |_| {
                        on_format.call((field, MarkdownAction::CodeBlock));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        rect { x: "4.5", y: "4.5", width: "15", height: "15", rx: "2.5" }
                        path { d: "M10 10l-2 2 2 2" }
                        path { d: "M14 10l2 2-2 2" }
                    }
                }
            }
            div { class: "editor-md-toolbar-separator" }
            div { class: "editor-md-toolbar-group",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Left-to-right",
                    aria_label: "Left-to-right",
                    onclick: move |_| {
                        on_block_dir.call((field, "ltr".to_string()));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        line { x1: "4", y1: "6", x2: "14", y2: "6" }
                        line { x1: "4", y1: "12", x2: "14", y2: "12" }
                        line { x1: "4", y1: "18", x2: "14", y2: "18" }
                        path { d: "M15 9l3 3-3 3" }
                    }
                }
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Right-to-left",
                    aria_label: "Right-to-left",
                    onclick: move |_| {
                        on_block_dir.call((field, "rtl".to_string()));
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        line { x1: "10", y1: "6", x2: "20", y2: "6" }
                        line { x1: "10", y1: "12", x2: "20", y2: "12" }
                        line { x1: "10", y1: "18", x2: "20", y2: "18" }
                        path { d: "M9 9l-3 3 3 3" }
                    }
                }
            }
        }
    }
}
