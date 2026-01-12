use dioxus::prelude::*;

use crate::vm::{MarkdownAction, MarkdownField};
use crate::views::editor::state::{
    LinkEditorState, WritingToolsCommand, WritingToolsMenuState, WritingToolsResultStatus,
    WritingToolsTone,
};

#[component]
pub fn EditorFormatToolbar(
    field: MarkdownField,
    disabled: bool,
    writing_menu_state: WritingToolsMenuState,
    writing_prompt: String,
    writing_tone: WritingToolsTone,
    writing_result_status: WritingToolsResultStatus,
    writing_result_target: Option<MarkdownField>,
    writing_result_title: String,
    writing_result_html: String,
    link_editor_state: Option<LinkEditorState>,
    on_format: Callback<(MarkdownField, MarkdownAction)>,
    on_block_dir: Callback<(MarkdownField, String)>,
    on_open_link_editor: Callback<MarkdownField>,
    on_close_link_editor: Callback<()>,
    on_update_link_url: Callback<String>,
    on_apply_link: Callback<MarkdownField>,
    on_remove_link: Callback<MarkdownField>,
    on_toggle_writing_menu: Callback<MarkdownField>,
    on_writing_prompt_change: Callback<String>,
    on_select_writing_tone: Callback<WritingToolsTone>,
    on_select_writing_command: Callback<(MarkdownField, WritingToolsCommand)>,
    on_writing_result_replace: Callback<MarkdownField>,
    on_writing_result_copy: Callback<MarkdownField>,
) -> Element {
    let writing_menu_open = matches!(
        writing_menu_state,
        WritingToolsMenuState::Open(current) if current == field
    );
    let writing_result_open = writing_result_status != WritingToolsResultStatus::Idle
        && matches!(writing_result_target, Some(current) if current == field);
    let link_editor_open = matches!(
        link_editor_state.as_ref(),
        Some(state) if state.field == field
    );
    let link_editor_url = link_editor_state
        .as_ref()
        .map(|state| state.url.clone())
        .unwrap_or_default();
    rsx! {
        div { class: "editor-md-toolbar",
            div { class: "editor-md-toolbar-group editor-link-tools",
                button {
                    class: if writing_menu_open {
                        "editor-md-toolbar-btn editor-md-toolbar-btn--active"
                    } else {
                        "editor-md-toolbar-btn"
                    },
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
                    class: if link_editor_open {
                        "editor-md-toolbar-btn editor-md-toolbar-btn--active"
                    } else {
                        "editor-md-toolbar-btn"
                    },
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Link",
                    aria_label: "Link",
                    onclick: move |_| {
                        on_open_link_editor.call(field);
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        path { d: "M9.5 14.5l5-5" }
                        path { d: "M8.7 15.3a4 4 0 0 1 0-5.7l2.1-2.1a4 4 0 1 1 5.7 5.7l-1 1" }
                        path { d: "M15.3 8.7a4 4 0 0 1 0 5.7l-2.1 2.1a4 4 0 1 1-5.7-5.7l1-1" }
                    }
                }
                if link_editor_open {
                    div {
                        class: "editor-link-popover",
                        onclick: move |evt| evt.stop_propagation(),
                        div { class: "editor-link-row",
                            input {
                                class: "editor-link-input",
                                r#type: "text",
                                value: "{link_editor_url}",
                                placeholder: "Paste link",
                                oninput: move |evt| on_update_link_url.call(evt.value()),
                            }
                        }
                        div { class: "editor-link-actions",
                            button {
                                class: "editor-link-btn",
                                r#type: "button",
                                onclick: move |_| on_apply_link.call(field),
                                "Apply"
                            }
                            button {
                                class: "editor-link-btn editor-link-btn--ghost",
                                r#type: "button",
                                onclick: move |_| on_remove_link.call(field),
                                "Remove"
                            }
                            button {
                                class: "editor-link-btn editor-link-btn--ghost",
                                r#type: "button",
                                onclick: move |_| on_close_link_editor.call(()),
                                "Close"
                            }
                        }
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
            div { class: "editor-md-toolbar-separator" }
            div { class: "editor-md-toolbar-group editor-writing-tools",
                button {
                    class: "editor-md-toolbar-btn",
                    r#type: "button",
                    disabled: disabled,
                    "data-tooltip": "Writing tools",
                    aria_label: "Writing tools",
                    onclick: move |_| {
                        on_toggle_writing_menu.call(field);
                    },
                    svg {
                        class: "editor-md-toolbar-icon",
                        view_box: "0 0 24 24",
                        path { d: "M12 3l1.2 2.9 2.9 1.2-2.9 1.2L12 11l-1.2-2.7-2.9-1.2 2.9-1.2L12 3z" }
                        path { d: "M18 12l0.8 2 2 0.8-2 0.8-0.8 2-0.8-2-2-0.8 2-0.8 0.8-2z" }
                        path { d: "M6 13l0.8 2 2 0.8-2 0.8-0.8 2-0.8-2-2-0.8 2-0.8 0.8-2z" }
                    }
                }
                if writing_menu_open {
                    div {
                        class: "editor-writing-menu",
                        onclick: move |evt| evt.stop_propagation(),
                        div { class: "editor-writing-prompt",
                            svg {
                                class: "editor-writing-prompt-icon",
                                view_box: "0 0 24 24",
                                path { d: "M4 12a8 8 0 1 1 16 0" }
                                path { d: "M4 12a8 8 0 0 0 6 7.7" }
                                circle { cx: "12", cy: "12", r: "2.8" }
                            }
                            input {
                                class: "editor-writing-prompt-input",
                                r#type: "text",
                                value: "{writing_prompt}",
                                placeholder: "Describe your change",
                                oninput: move |evt| on_writing_prompt_change.call(evt.value()),
                            }
                        }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::ImproveWording,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M4 17l4 3 12-12-4-3-12 12z" }
                                path { d: "M14 5l4 3" }
                            }
                            span { "Improve wording" }
                        }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::Simplify,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M6 8h12" }
                                path { d: "M7 12h10" }
                                path { d: "M8 16h8" }
                            }
                            span { "Simplify" }
                        }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::Concise,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M6 9h10" }
                                path { d: "M6 13h8" }
                                path { d: "M6 17h6" }
                            }
                            span { "Concise" }
                        }
                        div { class: "editor-writing-divider" }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::Summary,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M6 7h12" }
                                path { d: "M6 12h10" }
                                path { d: "M6 17h8" }
                            }
                            span { "Summary" }
                        }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::KeyPoints,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                circle { cx: "7", cy: "9", r: "1.2" }
                                circle { cx: "7", cy: "12", r: "1.2" }
                                circle { cx: "7", cy: "15", r: "1.2" }
                                line { x1: "10", y1: "9", x2: "18", y2: "9" }
                                line { x1: "10", y1: "12", x2: "18", y2: "12" }
                                line { x1: "10", y1: "15", x2: "18", y2: "15" }
                            }
                            span { "Key points" }
                        }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::List,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                rect { x: "6", y: "7", width: "2.5", height: "2.5", rx: "0.6" }
                                rect { x: "6", y: "11", width: "2.5", height: "2.5", rx: "0.6" }
                                rect { x: "6", y: "15", width: "2.5", height: "2.5", rx: "0.6" }
                                line { x1: "10.5", y1: "8.2", x2: "18", y2: "8.2" }
                                line { x1: "10.5", y1: "12.2", x2: "18", y2: "12.2" }
                                line { x1: "10.5", y1: "16.2", x2: "18", y2: "16.2" }
                            }
                            span { "List" }
                        }
                        button {
                            class: "editor-writing-item",
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::TurnIntoQuestion,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M9 9a3 3 0 1 1 4.2 2.6c-.8.4-1.2.9-1.2 1.8" }
                                circle { cx: "12", cy: "17", r: "1" }
                            }
                            span { "Turn into question" }
                        }
                        div { class: "editor-writing-divider" }
                        button {
                            class: if writing_tone == WritingToolsTone::Clear {
                                "editor-writing-item editor-writing-item--active"
                            } else {
                                "editor-writing-item"
                            },
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_tone.call(WritingToolsTone::Clear);
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::ImproveWording,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M7.5 9c1.2-2.3 7.8-2.3 9 0" }
                                path { d: "M8 14c1.6 1.4 6.4 1.4 8 0" }
                                circle { cx: "9", cy: "10", r: "1" }
                                circle { cx: "15", cy: "10", r: "1" }
                            }
                            span { "Clear" }
                        }
                        button {
                            class: if writing_tone == WritingToolsTone::Simple {
                                "editor-writing-item editor-writing-item--active"
                            } else {
                                "editor-writing-item"
                            },
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_tone.call(WritingToolsTone::Simple);
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::ImproveWording,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                rect { x: "5", y: "6", width: "14", height: "12", rx: "2" }
                                path { d: "M8 6V4h8v2" }
                                path { d: "M9 10h6" }
                                path { d: "M9 13h6" }
                            }
                            span { "Simple" }
                        }
                        button {
                            class: if writing_tone == WritingToolsTone::Formal {
                                "editor-writing-item editor-writing-item--active"
                            } else {
                                "editor-writing-item"
                            },
                            r#type: "button",
                            onclick: move |_| {
                                on_select_writing_tone.call(WritingToolsTone::Formal);
                                on_select_writing_command.call((
                                    field,
                                    WritingToolsCommand::ImproveWording,
                                ));
                            },
                            svg {
                                class: "editor-writing-item-icon",
                                view_box: "0 0 24 24",
                                path { d: "M6 8h12" }
                                path { d: "M6 12h8" }
                                path { d: "M6 16h6" }
                            }
                            span { "Formal" }
                        }
                    }
                }
                if writing_result_open {
                    div { class: "editor-writing-result",
                        div { class: "editor-writing-result-header",
                            span { class: "editor-writing-result-title",
                                "{writing_result_title}"
                            }
                        }
                        div {
                            class: if writing_result_status == WritingToolsResultStatus::Loading {
                                "editor-writing-result-body editor-writing-result-body--loading"
                            } else {
                                "editor-writing-result-body"
                            },
                            dangerous_inner_html: "{writing_result_html}"
                        }
                        div { class: "editor-writing-result-actions",
                            button {
                                class: "editor-writing-result-btn",
                                r#type: "button",
                                disabled: writing_result_status != WritingToolsResultStatus::Ready,
                                onclick: move |_| {
                                    on_writing_result_replace.call(field);
                                },
                                "Replace"
                            }
                            button {
                                class: "editor-writing-result-btn",
                                r#type: "button",
                                disabled: writing_result_status != WritingToolsResultStatus::Ready,
                                onclick: move |_| {
                                    on_writing_result_copy.call(field);
                                },
                                "Copy"
                            }
                            button {
                                class: "editor-writing-result-flag",
                                r#type: "button",
                                aria_label: "Report",
                                "!"
                            }
                        }
                    }
                }
            }
        }
    }
}
