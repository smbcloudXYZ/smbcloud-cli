use super::SmbMcpServer;

xcrs::xcrs_mcp_tools!(
    SmbMcpServer,
    "smb_simulator_list",
    "smb_simulator_find",
    "smb_ios_app_test",
    "smb_simulator_screenshot",
    "smb_simulator_launch_app",
    "smb_simulator_terminate_app",
    "smb_simulator_open_url",
    "smb_simulator_ui_dump",
    "smb_simulator_list_elements",
    "smb_simulator_tap",
    "smb_simulator_type_text",
    "smb_simulator_swipe",
    "smb_simulator_press_home",
    "smb_simulator_press_button",
    "smb_simulator_orientation_get",
    "smb_simulator_orientation_set"
);
