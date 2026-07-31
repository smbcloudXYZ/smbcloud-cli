use super::SmbMcpServer;

xcrs::xcrs_mcp_tools!(
    SmbMcpServer,
    "smb_list_simulators",
    "smb_find_simulator",
    "smb_boot_and_install",
    "smb_screenshot",
    "smb_launch_app",
    "smb_terminate_app",
    "smb_open_url",
    "smb_describe_ui",
    "smb_list_elements",
    "smb_tap",
    "smb_type_text",
    "smb_swipe",
    "smb_button",
    "smb_orientation_get",
    "smb_orientation_set",
    "smb_capabilities",
    "smb_click",
    "smb_gesture",
    "smb_use_target"
);
