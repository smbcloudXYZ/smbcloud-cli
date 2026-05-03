pub mod confirm_dialog;
pub mod deployment_detail_view;
pub mod deployment_table;
pub mod me_view;
pub mod project_detail_view;
pub mod project_table;
pub mod theme;

use console::style;

pub fn succeed_symbol() -> String {
    style("✔").green().to_string()
}

pub fn fail_symbol() -> String {
    style("✘").for_stderr().red().to_string()
}

pub fn succeed_message(message: &str) -> String {
    style(message).white().to_string()
}

pub fn fail_message(message: &str) -> String {
    style(message).italic().red().to_string()
}

pub fn highlight(string: &str) -> String {
    style(string).italic().green().to_string()
}

pub fn description(string: &str) -> String {
    style(string).italic().yellow().to_string()
}
