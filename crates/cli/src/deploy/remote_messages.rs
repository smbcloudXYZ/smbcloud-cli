pub fn build_next_app() -> String {
  "> next build".to_owned()
}

pub fn start_server(name: &str) -> String {
  format!("Start {}", name)
}