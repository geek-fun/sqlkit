pub mod browse;
pub mod connection;
pub mod converter;
pub mod file_operations;
pub mod helpers;
pub mod jdbc;
pub mod query;
pub mod server;
pub mod store;
pub mod transfer;

pub use browse::*;
pub use connection::*;
pub use file_operations::*;
pub use jdbc::*;
pub use query::*;
pub use server::*;
pub use store::*;
pub use transfer::*;

/// Returns the current app version from Cargo.toml.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
