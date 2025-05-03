use console::style;

pub fn succeed_symbol() -> String {
  style("✔").green().to_string()
}

pub fn fail_symbol() -> String {
  style("✘").for_stderr().red().to_string()
}

pub fn succeed_message(message: &str) -> String {
  style(message).bold().white().to_string()
}