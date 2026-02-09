use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::json_io;
use crate::state::data_model::TableData;

#[component]
pub fn Toolbar(
    data: Signal<TableData>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "toolbar",
            button {
                class: "toolbar-btn",
                onclick: move |_| {
                    spawn(async move {
                        open_file(data, file_path, error_message).await;
                    });
                },
                "Open"
            }
            button {
                class: "toolbar-btn",
                disabled: file_path.read().is_none(),
                onclick: move |_| {
                    save_file(data, file_path, error_message);
                },
                "Save"
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

async fn open_file(
    mut data: Signal<TableData>,
    mut file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) {
    let task = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await;

    if let Some(handle) = task {
        let path = handle.path().to_path_buf();
        match json_io::load_json(&path) {
            Ok(rows) => {
                data.set(rows);
                file_path.set(Some(path));
                error_message.set(None);
            }
            Err(e) => {
                error_message.set(Some(e.to_string()));
            }
        }
    }
}

fn save_file(
    data: Signal<TableData>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) {
    if let Some(path) = file_path.read().as_ref() {
        match json_io::save_json(path, &data.read()) {
            Ok(()) => {
                error_message.set(None);
            }
            Err(e) => {
                error_message.set(Some(e.to_string()));
            }
        }
    }
}
