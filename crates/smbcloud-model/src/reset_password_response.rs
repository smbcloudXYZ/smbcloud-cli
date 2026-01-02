use {
    serde::{Deserialize, Serialize},
    tsync::tsync,
};

#[derive(Debug, Serialize, Deserialize)]
#[tsync]
pub struct ResetPasswordResponse {
    pub code: Option<i32>,
    pub message: String,
}
