use dioxus::prelude::*;
use std::path::PathBuf;

use crate::state::i18n::{self, Language};
use crate::state::table_state::TableState;
use crate::ui::actions;

#[component]
pub fn Toolbar(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
    show_meta_row: Signal<bool>,
    save_success: Signal<bool>,
) -> Element {
    let mut new_column = use_signal(String::new);

    let snapshot = data.read().clone();
    let current_language = *language.read();
    let columns = snapshot.display_columns();
    let can_undo = snapshot.can_undo();
    let can_redo = snapshot.can_redo();
    let filter_column_value = snapshot.filter_column().unwrap_or("").to_string();
    let filter_query_value = snapshot.filter_query().to_string();
    let search_query_value = snapshot.search_query().to_string();

    let open_label = i18n::tr(current_language, "toolbar.open");
    let save_label = i18n::tr(current_language, "toolbar.save");
    let undo_label = i18n::tr(current_language, "toolbar.undo");
    let redo_label = i18n::tr(current_language, "toolbar.redo");
    let add_row_label = i18n::tr(current_language, "toolbar.add_row");
    let delete_row_label = i18n::tr(current_language, "toolbar.delete_row");
    let filter_column_placeholder = i18n::tr(current_language, "toolbar.filter_column_placeholder");
    let filter_value_placeholder = i18n::tr(current_language, "toolbar.filter_value_placeholder");
    let clear_filter_label = i18n::tr(current_language, "toolbar.clear_filter");
    let search_placeholder = i18n::tr(current_language, "toolbar.search_placeholder");
    let new_column_placeholder = i18n::tr(current_language, "toolbar.new_column_placeholder");
    let add_column_label = i18n::tr(current_language, "toolbar.add_column");
    let delete_column_label = i18n::tr(current_language, "toolbar.delete_column");
    let selected_column_label = i18n::tr(current_language, "toolbar.selected_column");
    let save_success_label = i18n::tr(current_language, "toolbar.save_success");
    let show_meta_label = i18n::tr(current_language, "toolbar.show_meta");
    let hide_meta_label = i18n::tr(current_language, "toolbar.hide_meta");

    let meta_visible = *show_meta_row.read();

    rsx! {
        div { class: "toolbar",
            // File group
            div { class: "toolbar-group",
                select {
                    class: "toolbar-select toolbar-select-sm",
                    id: "select-language",
                    value: "{current_language.code()}",
                    onchange: move |evt| {
                        if let Some(next_language) = Language::from_code(&evt.value()) {
                            language.set(next_language);
                        }
                    },
                    for lang in Language::all().iter().copied() {
                        option { value: "{lang.code()}", "{i18n::tr(current_language, lang.label_key())}" }
                    }
                }
                button {
                    class: "toolbar-btn",
                    id: "btn-open",
                    onclick: move |_| {
                        spawn(async move {
                            actions::open_file(data, language, file_path, error_message, selected_row, selected_column)
                                .await;
                        });
                    },
                    "\u{1F4C2} {open_label}"
                }
                button {
                    class: "toolbar-btn",
                    id: "btn-save",
                    disabled: file_path.read().is_none(),
                    onclick: move |_| {
                        let success = actions::save_file(data, file_path, error_message);
                        if success {
                            save_success.set(true);
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                save_success.set(false);
                            });
                        }
                    },
                    "\u{1F4BE} {save_label}"
                }
                if *save_success.read() {
                    span { class: "save-success", "\u{2714} {save_success_label}" }
                }
            }
            div { class: "toolbar-separator" }

            // Edit group
            div { class: "toolbar-group",
                button {
                    class: "toolbar-btn",
                    id: "btn-undo",
                    disabled: !can_undo,
                    onclick: move |_| {
                        let undone = data.with_mut(|state| state.undo());
                        if undone {
                            selected_row.set(None);
                            selected_column.set(None);
                            error_message.set(None);
                        }
                    },
                    "\u{21A9} {undo_label}"
                }
                button {
                    class: "toolbar-btn",
                    id: "btn-redo",
                    disabled: !can_redo,
                    onclick: move |_| {
                        let redone = data.with_mut(|state| state.redo());
                        if redone {
                            selected_row.set(None);
                            selected_column.set(None);
                            error_message.set(None);
                        }
                    },
                    "\u{21AA} {redo_label}"
                }
            }
            div { class: "toolbar-separator" }

            // Row/Column group
            div { class: "toolbar-group",
                button {
                    class: "toolbar-btn",
                    id: "btn-add-row",
                    onclick: move |_| {
                        let mut new_index = None;
                        data.with_mut(|state| {
                            if state.add_row() {
                                new_index = state.data().len().checked_sub(1);
                            }
                        });
                        if let Some(idx) = new_index {
                            selected_row.set(Some(idx));
                        }
                        error_message.set(None);
                    },
                    "\u{2795} {add_row_label}"
                }
                button {
                    class: "toolbar-btn toolbar-btn-danger",
                    id: "btn-delete-row",
                    disabled: selected_row.read().is_none(),
                    onclick: move |_| {
                        let row_index = *selected_row.read();
                        if let Some(row_index) = row_index {
                            let removed = data.with_mut(|state| state.delete_row(row_index));
                            if removed {
                                selected_row.set(None);
                                error_message.set(None);
                            } else {
                                error_message.set(Some(
                                    i18n::tr(*language.read(), "error.delete_row_failed").to_string(),
                                ));
                            }
                        } else {
                            error_message.set(Some(
                                i18n::tr(*language.read(), "error.select_row_to_delete").to_string(),
                            ));
                        }
                    },
                    "\u{1F5D1} {delete_row_label}"
                }
                input {
                    class: "toolbar-input",
                    id: "input-new-column",
                    placeholder: "{new_column_placeholder}",
                    value: "{new_column.read()}",
                    oninput: move |evt| {
                        new_column.set(evt.value());
                    }
                }
                button {
                    class: "toolbar-btn",
                    id: "btn-add-column",
                    onclick: move |_| {
                        let name = new_column.read().trim().to_string();
                        if name.is_empty() {
                            error_message.set(Some(
                                i18n::tr(*language.read(), "error.column_name_required").to_string(),
                            ));
                            return;
                        }

                        let added = data.with_mut(|state| state.add_column(&name));
                        if added {
                            selected_column.set(Some(name));
                            new_column.set(String::new());
                            error_message.set(None);
                        } else {
                            error_message
                                .set(Some(i18n::tr(*language.read(), "error.column_exists").to_string()));
                        }
                    },
                    "\u{2795} {add_column_label}"
                }
                button {
                    class: "toolbar-btn toolbar-btn-danger",
                    id: "btn-delete-column",
                    disabled: selected_column.read().is_none(),
                    onclick: move |_| {
                        let column = selected_column.read().clone();
                        if let Some(col) = column {
                            let removed = data.with_mut(|state| state.delete_column(&col));
                            if removed {
                                selected_column.set(None);
                                error_message.set(None);
                                actions::persist_sidecar_if_possible(data, file_path, error_message);
                            } else {
                                error_message.set(Some(
                                    i18n::tr(*language.read(), "error.delete_column_failed").to_string(),
                                ));
                            }
                        } else {
                            error_message.set(Some(
                                i18n::tr(*language.read(), "error.select_column_to_delete").to_string(),
                            ));
                        }
                    },
                    "\u{1F5D1} {delete_column_label}"
                }
            }
            div { class: "toolbar-separator" }

            // Filter group
            div { class: "toolbar-group",
                select {
                    class: "toolbar-select",
                    id: "select-filter-column",
                    value: "{filter_column_value}",
                    onchange: move |evt| {
                        let value = evt.value();
                        data.with_mut(|state| {
                            let query = state.filter_query().to_string();
                            let col = if value.is_empty() { None } else { Some(value) };
                            state.set_filter(col, query);
                        });
                    },
                    option { value: "", "{filter_column_placeholder}" }
                    for col in &columns {
                        option { value: "{col}", "{col}" }
                    }
                }
                input {
                    class: "toolbar-input",
                    id: "input-filter-query",
                    placeholder: "{filter_value_placeholder}",
                    value: "{filter_query_value}",
                    oninput: move |evt| {
                        let query = evt.value();
                        data.with_mut(|state| {
                            let col = state.filter_column().map(|c| c.to_string());
                            state.set_filter(col, query);
                        });
                    }
                }
                button {
                    class: "toolbar-btn",
                    id: "btn-clear-filter",
                    onclick: move |_| {
                        data.with_mut(|state| {
                            state.clear_filter();
                        });
                    },
                    "\u{2715} {clear_filter_label}"
                }
            }
            div { class: "toolbar-separator" }

            // Search group
            div { class: "toolbar-group",
                input {
                    class: "toolbar-input",
                    id: "input-search-query",
                    placeholder: "{search_placeholder}",
                    value: "{search_query_value}",
                    oninput: move |evt| {
                        let query = evt.value();
                        data.with_mut(|state| {
                            state.set_search(query);
                        });
                    }
                }
            }
            div { class: "toolbar-separator" }

            // View group
            div { class: "toolbar-group",
                {
                    let freeze_label = i18n::tr(current_language, "toolbar.freeze_columns");
                    let frozen = snapshot.frozen_columns();
                    rsx! {
                        span { class: "toolbar-label", "{freeze_label}" }
                        input {
                            class: "toolbar-input toolbar-input-sm",
                            id: "input-frozen-columns",
                            r#type: "number",
                            min: "0",
                            value: "{frozen}",
                            oninput: move |evt: Event<FormData>| {
                                let val = evt.value().parse::<usize>().unwrap_or(0);
                                data.with_mut(|state| {
                                    state.set_frozen_columns(if val == 0 { None } else { Some(val) });
                                });
                                actions::persist_sidecar_if_possible(data, file_path, error_message);
                            }
                        }
                    }
                }
                button {
                    class: "toolbar-btn",
                    id: "btn-toggle-meta",
                    onclick: move |_| {
                        show_meta_row.set(!meta_visible);
                    },
                    if meta_visible {
                        "\u{25BC} {hide_meta_label}"
                    } else {
                        "\u{25B6} {show_meta_label}"
                    }
                }
            }

            // Info area (right-aligned)
            div { class: "toolbar-info",
                if let Some(col) = selected_column.read().as_ref() {
                    span {
                        class: "toolbar-label",
                        id: "label-selected-column",
                        "{selected_column_label}: {col}"
                    }
                }
                if let Some(path) = file_path.read().as_ref() {
                    span { class: "file-path", "{path.display()}" }
                }
                if let Some(err) = error_message.read().as_ref() {
                    span { class: "error-message", "{err}" }
                }
            }
        }
    }
}
