mod args;
mod history;
mod keybind;
mod logs;
mod models;
pub mod provider;
mod update;

pub use args::{
    Cli, CliCommand, HistoryCliArgs, KeybindCliArgs, KeybindCommand, LogsCliArgs, ModelsCliArgs,
    ModelsCommand, ProviderCliArgs, ProviderCommand, UpdateCliArgs,
};
pub use history::handle_history_command;
pub use keybind::handle_keybind_command;
pub use logs::handle_logs_command;
pub use models::handle_models_command;
pub use provider::handle_provider_command;
pub use update::handle_update_command;
