use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::jsheet_io;
use crate::state::i18n::{self, Language};
use crate::state::table_state::TableState;
use crate::ui::table::Table;
use crate::ui::toolbar::Toolbar;

const STYLESHEET: &str = include_str!("../../assets/styles.css");

#[derive(Clone, PartialEq)]
struct SheetTabState {
    data: TableState,
    file_path: Option<PathBuf>,
    error_message: Option<String>,
    selected_row: Option<usize>,
    selected_column: Option<String>,
}

impl Default for SheetTabState {
    fn default() -> Self {
        Self {
            data: TableState::new(),
            file_path: None,
            error_message: None,
            selected_row: None,
            selected_column: None,
        }
    }
}

#[component]
pub fn App() -> Element {
    let data = use_signal(TableState::new);
    let language = use_signal(Language::default);
    let file_path = use_signal::<Option<PathBuf>>(|| None);
    let error_message = use_signal::<Option<String>>(|| None);
    let mut selected_row = use_signal::<Option<usize>>(|| None);
    let mut selected_column = use_signal::<Option<String>>(|| None);
    let tabs = use_signal(|| vec![SheetTabState::default()]);
    let active_tab = use_signal(|| 0usize);

    use_effect({
        let mut data = data;
        let mut file_path = file_path;
        let mut error_message = error_message;
        move || {
            if let Ok(path) = std::env::var("JSONSHEET_OPEN") {
                let path = PathBuf::from(path);
                match jsheet_io::load_json_and_sidecar(&path) {
                    Ok((rows, jsheet_meta)) => {
                        data.with_mut(|state| {
                            state.replace_data_and_jsheet(rows, jsheet_meta);
                        });
                        file_path.set(Some(path));
                        error_message.set(None);
                        selected_row.set(None);
                        selected_column.set(None);
                    }
                    Err(e) => {
                        error_message.set(Some(e.to_string()));
                    }
                }
            }
        }
    });

    use_effect({
        let mut tabs = tabs;
        move || {
            let active_index = *active_tab.read();
            let current_snapshot = snapshot_active_tab(
                data,
                file_path,
                error_message,
                selected_row,
                selected_column,
            );
            let mut next_tabs = tabs.peek().clone();
            if active_index >= next_tabs.len() {
                next_tabs.resize_with(active_index + 1, SheetTabState::default);
            }
            if next_tabs[active_index] != current_snapshot {
                next_tabs[active_index] = current_snapshot;
                tabs.set(next_tabs);
            }
        }
    });

    let current_language = *language.read();
    let new_tab_label = i18n::tr(current_language, "tabs.new");
    let close_tab_label = i18n::tr(current_language, "tabs.close");
    let untitled_label = i18n::tr(current_language, "tabs.untitled");
    let tabs_snapshot = tabs.read().clone();
    let active_index = *active_tab.read();
    let tab_count = tabs_snapshot.len();

    rsx! {
        document::Style { "{STYLESHEET}" }
        div { class: "app",
            div { class: "tab-bar", id: "tab-bar",
                for (index, tab) in tabs_snapshot.iter().enumerate() {
                    div { class: if index == active_index { "tab-item active" } else { "tab-item" },
                        button {
                            class: "tab-btn",
                            id: format!("tab-{index}"),
                            onclick: {
                                let mut tabs = tabs;
                                let mut active_tab = active_tab;
                                move |_| {
                                    let current_index = *active_tab.read();
                                    if index == current_index {
                                        return;
                                    }

                                    let current_snapshot = snapshot_active_tab(
                                        data,
                                        file_path,
                                        error_message,
                                        selected_row,
                                        selected_column,
                                    );
                                    tabs.with_mut(|all_tabs| {
                                        if current_index < all_tabs.len() {
                                            all_tabs[current_index] = current_snapshot;
                                        }
                                    });

                                    if let Some(next_tab) = tabs.read().get(index).cloned() {
                                        load_tab_into_signals(
                                            &next_tab,
                                            data,
                                            file_path,
                                            error_message,
                                            selected_row,
                                            selected_column,
                                        );
                                        active_tab.set(index);
                                    }
                                }
                            },
                            "{tab_title(tab, index, untitled_label)}"
                        }
                        if tab_count > 1 {
                            button {
                                class: "tab-close-btn",
                                id: format!("btn-close-tab-{index}"),
                                title: "{close_tab_label}",
                                onclick: {
                                    let mut tabs = tabs;
                                    let mut active_tab = active_tab;
                                    move |evt: Event<MouseData>| {
                                        evt.stop_propagation();

                                        let current_index = *active_tab.read();
                                        let current_snapshot = snapshot_active_tab(
                                            data,
                                            file_path,
                                            error_message,
                                            selected_row,
                                            selected_column,
                                        );

                                        let mut next_active_index = 0usize;
                                        let mut next_tab_state = SheetTabState::default();

                                        tabs.with_mut(|all_tabs| {
                                            if current_index < all_tabs.len() {
                                                all_tabs[current_index] = current_snapshot.clone();
                                            }

                                            if all_tabs.len() <= 1 {
                                                all_tabs.clear();
                                                all_tabs.push(SheetTabState::default());
                                                next_active_index = 0;
                                                next_tab_state = all_tabs[0].clone();
                                                return;
                                            }

                                            let close_index = index.min(all_tabs.len() - 1);
                                            all_tabs.remove(close_index);

                                            next_active_index = if current_index > close_index {
                                                current_index - 1
                                            } else {
                                                current_index.min(all_tabs.len() - 1)
                                            };
                                            next_tab_state = all_tabs[next_active_index].clone();
                                        });

                                        load_tab_into_signals(
                                            &next_tab_state,
                                            data,
                                            file_path,
                                            error_message,
                                            selected_row,
                                            selected_column,
                                        );
                                        active_tab.set(next_active_index);
                                    }
                                },
                                "x"
                            }
                        }
                    }
                }
                button {
                    class: "tab-add-btn",
                    id: "btn-new-tab",
                    title: "{new_tab_label}",
                    onclick: {
                        let mut tabs = tabs;
                        let mut active_tab = active_tab;
                        move |_| {
                            let current_index = *active_tab.read();
                            let current_snapshot = snapshot_active_tab(
                                data,
                                file_path,
                                error_message,
                                selected_row,
                                selected_column,
                            );

                            let mut next_index = 0usize;
                            tabs.with_mut(|all_tabs| {
                                if current_index < all_tabs.len() {
                                    all_tabs[current_index] = current_snapshot;
                                }
                                all_tabs.push(SheetTabState::default());
                                next_index = all_tabs.len() - 1;
                            });

                            load_tab_into_signals(
                                &SheetTabState::default(),
                                data,
                                file_path,
                                error_message,
                                selected_row,
                                selected_column,
                            );
                            active_tab.set(next_index);
                        }
                    },
                    "+"
                }
            }
            Toolbar { data, language, file_path, error_message, selected_row, selected_column }
            Table { data, language, file_path, error_message, selected_row, selected_column }
        }
    }
}

fn snapshot_active_tab(
    data: Signal<TableState>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
) -> SheetTabState {
    SheetTabState {
        data: data.read().clone(),
        file_path: file_path.read().clone(),
        error_message: error_message.read().clone(),
        selected_row: *selected_row.read(),
        selected_column: selected_column.read().clone(),
    }
}

fn load_tab_into_signals(
    tab: &SheetTabState,
    mut data: Signal<TableState>,
    mut file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
    mut selected_row: Signal<Option<usize>>,
    mut selected_column: Signal<Option<String>>,
) {
    data.set(tab.data.clone());
    file_path.set(tab.file_path.clone());
    error_message.set(tab.error_message.clone());
    selected_row.set(tab.selected_row);
    selected_column.set(tab.selected_column.clone());
}

fn tab_title(tab: &SheetTabState, index: usize, untitled_label: &str) -> String {
    if let Some(path) = tab.file_path.as_ref() {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string())
    } else {
        format!("{untitled_label} {}", index + 1)
    }
}
