use smbcloud_networking::environment::Environment;

pub struct Payload {
  pub project_id: i32,
  pub commit_hash: String,
  pub status: i8
}

pub async fn create(env: Environment, payload: Payload) {
  todo!()
}