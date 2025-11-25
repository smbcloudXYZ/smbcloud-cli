use {
    serde::{Deserialize, Serialize},
    smbcloud_model::project::Project,
};

/// smbCloud config from the .smb/config.toml file.
#[derive(Deserialize, Serialize)]
pub struct Config {
    pub name: String,
    pub description: Option<String>,
    pub project: Project,
}

impl Config {
    pub fn ssh_key_path(&self, user_id: i32) -> String {
        // Use the dirs crate to get the home directory
        let home = dirs::home_dir().expect("Could not determine home directory");
        let key_path = home.join(".ssh").join(format!("id_{}@smbcloud", user_id));
        let key_path_str = key_path.to_string_lossy().to_string();
        println!("Use key path: {}", key_path_str);
        key_path_str
    }
}
