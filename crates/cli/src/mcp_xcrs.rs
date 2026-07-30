use super::SmbMcpServer;

xcrs::xcrs_mcp_tools!(
    SmbMcpServer,
    "smb_simulator_list",
    "smb_simulator_find",
    "smb_ios_app_test",
    "smb_simulator_screenshot",
    "smb_simulator_launch_app",
    "smb_simulator_terminate_app",
    "smb_simulator_open_url"
);
