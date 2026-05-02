use {
    smbcloud_network::environment::Environment,
    smbcloud_networking::{smb_base_url_builder, smb_client::SmbClient},
};

pub(crate) fn build_mail_apps_url(
    env: Environment,
    client: (&SmbClient, &str),
    project_id: Option<&str>,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    if let Some(project_id) = project_id {
        url_builder.add_param("project_id", project_id);
    }
    url_builder.build()
}

pub(crate) fn build_mail_app_url(
    env: Environment,
    client: (&SmbClient, &str),
    mail_app_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    url_builder.add_route(mail_app_id);
    url_builder.build()
}

pub(crate) fn build_mail_inboxes_url(
    env: Environment,
    client: (&SmbClient, &str),
    mail_app_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    url_builder.add_route(mail_app_id);
    url_builder.add_route("inboxes");
    url_builder.build()
}

pub(crate) fn build_mail_inbox_url(
    env: Environment,
    client: (&SmbClient, &str),
    mail_app_id: &str,
    inbox_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    url_builder.add_route(mail_app_id);
    url_builder.add_route("inboxes");
    url_builder.add_route(inbox_id);
    url_builder.build()
}

pub(crate) fn build_mail_inbox_test_url(
    env: Environment,
    client: (&SmbClient, &str),
    mail_app_id: &str,
    inbox_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    url_builder.add_route(mail_app_id);
    url_builder.add_route("inboxes");
    url_builder.add_route(inbox_id);
    url_builder.add_route("send_test_email");
    url_builder.build()
}

pub(crate) fn build_mail_messages_url(
    env: Environment,
    client: (&SmbClient, &str),
    mail_app_id: &str,
    inbox_id: &str,
    limit: Option<u32>,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    url_builder.add_route(mail_app_id);
    url_builder.add_route("inboxes");
    url_builder.add_route(inbox_id);
    url_builder.add_route("messages");
    if let Some(limit) = limit {
        let limit_string = limit.to_string();
        url_builder.add_param("limit", &limit_string);
    }
    url_builder.build()
}

pub(crate) fn build_mail_message_url(
    env: Environment,
    client: (&SmbClient, &str),
    mail_app_id: &str,
    inbox_id: &str,
    message_id: &str,
) -> String {
    let mut url_builder = smb_base_url_builder(env, client);
    url_builder.add_route("v1/mail_apps");
    url_builder.add_route(mail_app_id);
    url_builder.add_route("inboxes");
    url_builder.add_route(inbox_id);
    url_builder.add_route("messages");
    url_builder.add_route(message_id);
    url_builder.build()
}
