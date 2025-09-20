pub async fn is_logged_in(env: Environment) -> bool {
    // Check if token file exists
    if !smb_token_file_path(env).is_some() {
        return false;
    }
    // Check if token is valid
    let access_token = get_smb_token(env)?;
    match me(env, access_token).await {
        Ok(_) => true,
        Err(_) => false,
    }
}
