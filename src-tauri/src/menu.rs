use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{App, Emitter, Error, Manager};

pub fn create_menu(app: &App) -> Result<(), Error> {
    let app_menu = SubmenuBuilder::new(app, "SqlKit")
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&MenuItem::with_id(
            app,
            "new_connection",
            "New Connection",
            true,
            Some("CommandOrControl+N"),
        )?)
        .separator()
        .close_window()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&MenuItem::with_id(
            app,
            "reload",
            "Reload",
            true,
            Some("CommandOrControl+R"),
        )?)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "toggle_dev_tools",
            "Toggle Developer Tools",
            true,
            Some("F12"),
        )?)
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .build()?;

    app.set_menu(menu)?;

    app.on_menu_event(move |app_handle, event| {
        if let Some(window) = app_handle.get_webview_window("main") {
            match event.id().0.as_str() {
                "new_connection" => {
                    let _ = window.emit("menu:new-connection", ());
                }
                "reload" => {
                    let _ = window.eval("location.reload()");
                }
                "toggle_dev_tools" =>
                {
                    #[cfg(debug_assertions)]
                    if window.is_devtools_open() {
                        window.close_devtools();
                    } else {
                        window.open_devtools();
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}
