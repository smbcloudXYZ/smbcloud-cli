use {
    crate::runner::Runner,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize)]
#[tsync::tsync]
pub struct Repository {
    pub is_smbcloud_project: bool,
    pub short_name: String,
    pub name: String,
    pub path: String,
    pub runner: Runner,
}
