#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod file;

use slint::{slint, Model, ModelRc, SharedString, StandardListViewItem, VecModel};
use std::error::Error;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use win_msgbox::{CancelTryAgainContinue, Okay};

slint::include_modules!();

#[derive(Clone)]
struct Name {
    text: String,
}


fn main() -> Result<(), Box<dyn Error>> {
    let app = Arc::new(AppWindow::new()?);


    let filtered_model = Rc::new(VecModel::from(fetch_active_processes()));
    app.set_process_list(filtered_model.clone().into());

    let app_clone = Arc::clone(&app);

    app_clone.on_process_filter_value({
        let app_clone = Arc::clone(&app_clone);
        move || {
            let filtered_items: Vec<StandardListViewItem> = filtered_model
                .iter()
                .filter_map(|process_name| {
                    let process_name_str = process_name.text;
                    let item = StandardListViewItem::from(slint::format!("{}", process_name_str));
                    if process_name_str.to_lowercase().starts_with(
                        app_clone
                            .get_process_filter_name()
                            .as_str()
                            .to_lowercase()
                            .as_str(),
                    ) {
                        Some(item)
                    } else {
                        None
                    }
                })
                .collect();

            let filtered_model = VecModel::from(filtered_items);
            let filtered_model_rc: ModelRc<StandardListViewItem> =
                ModelRc::from(Rc::new(filtered_model));
            app_clone.set_process_list(filtered_model_rc);
        }
    });

    app_clone.on_inject_file({
        let app_clone = Arc::clone(&app_clone);
        move || {
            let path_to_dll = app_clone.get_path_to_dll();
            let path_str = path_to_dll.as_str();

            if path_str.is_empty() {
                println!("Path to dll is empty!");
                app_clone.set_error(SharedString::from("The path to your .dll file cannot be empty."));
                return;
            }

            let path = Path::new(path_str);

            if !path.exists() {
                app_clone.set_error(SharedString::from("Your .dll file could not be found."));
                return;
            }


            let selected_index = app_clone.get_selected_process_index();
            let process_list = app_clone.get_process_list();

            if let Some(process_list_data) = process_list.row_data(selected_index.parse().unwrap()) {
                let process_info = process_list_data.text;
                let parts: Vec<&str> = process_info.split_whitespace().collect();

                if let Some(process_id_str) = parts.last() {
                    if let Some(cleaned_process_id_str) = process_id_str.trim_matches(|c: char| c == '(' || c == ')').parse::<u32>().ok() {
                        println!("Attaching to process with ID: {}", cleaned_process_id_str);
                        file::attach_to_process(cleaned_process_id_str, &*app_clone.get_path_to_dll())
                            .expect("Failed to attach the DLL");
                        app_clone.set_success(SharedString::from(format!(
                            "Your .dll file has been attached to {}",
                            process_info
                        )));
                        app_clone.set_error(SharedString::new());
                    } else {
                        println!("Failed to parse process ID: {}", process_id_str);
                        app_clone.set_error(SharedString::from(format!(
                            "Failed to parse process ID: {}",
                            process_id_str
                        )));
                    }

                } else {
                    println!("Failed to extract process ID from the selected item");
                    app_clone.set_error(SharedString::from("Failed to extract process ID from the selected item"));
                }
            } else {
                println!("Invalid index or failed to access the process list");
                app_clone.set_error(SharedString::from("Invalid index or failed to access the process list"));
            }

        }
    });

    app.run()?;

    Ok(())
}

fn fetch_active_processes() -> Vec<StandardListViewItem> {
    let mut system = System::new_all();
    system.refresh_all();

    let mut items = Vec::new();

    for process in system.processes() {
        let process_name = process.1.name().to_string();
        let process_id = *process.0;
        items.push(StandardListViewItem::from(slint::format!("{} ({})", process_name, process_id)));
    }

    items
}
