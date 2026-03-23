pub fn build_next_app() -> String {
    "> next build".to_owned()
}

#[allow(dead_code)]
pub fn build_vite_spa() -> String {
    "vite build".to_owned()
}

pub fn start_server(name: &str) -> String {
    format!("Start {}", name)
}

#[allow(dead_code)]
pub fn deploy_vite_spa_complete(name: &str) -> String {
    format!("Deployed {}", name)
}
