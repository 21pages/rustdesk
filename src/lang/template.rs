lazy_static::lazy_static! {
pub static ref T: std::collections::HashMap<&'static str, &'static str> =
    [
        ("Status", ""),
        ("Your Desktop", ""),
        ("desk_tip", ""),
        ("Password", ""),
        ("Ready", ""),
        ("Established", ""),
        ("connecting_status", ""),
        ("Enable Service", ""),
        ("Start Service", ""),
        ("Service is running", ""),
        ("Service is not running", ""),
        ("not_ready_status", ""),
        ("Control Remote Desktop", ""),
        ("Transfer File", ""),
        ("Connect", ""),
        ("Recent Sessions", ""),
        ("Address Book", ""),
        ("Confirmation", ""),
        ("TCP Tunneling", ""),
        ("Remove", ""),
        ("Refresh random password", ""),
        ("Set your own password", ""),
        ("Enable Keyboard/Mouse", ""),
        ("Enable Clipboard", ""),
        ("Enable File Transfer", ""),
        ("Enable TCP Tunneling", ""),
        ("IP Whitelisting", ""),
        ("ID/Relay Server", ""),
        ("Import Server Config", ""),
        ("Export Server Config", ""),
        ("Import server configuration successfully", ""),
        ("Export server configuration successfully", ""),
        ("Invalid server configuration", ""),
        ("Clipboard is empty", ""),
        ("Stop service", ""),
        ("Change ID", ""),
        ("Website", ""),
        ("About", ""),
        ("Mute", ""),
        ("Audio Input", ""),
        ("Enhancements", ""),
        ("Hardware Codec", ""),
        ("Adaptive Bitrate", ""),
        ("ID Server", ""),
        ("Relay Server", ""),
        ("API Server", ""),
        ("invalid_http", ""),
        ("Invalid IP", ""),
        ("id_change_tip", ""),
        ("Invalid format", ""),
        ("server_not_support", ""),
        ("Not available", ""),
        ("Too frequent", ""),
        ("Cancel", ""),
        ("Skip", ""),
        ("Close", ""),
        ("Retry", ""),
        ("OK", ""),
        ("Password Required", ""),
        ("Please enter your password", ""),
        ("Remember password", ""),
        ("Wrong Password", ""),
        ("Do you want to enter again?", ""),
        ("Connection Error", ""),
        ("Error", ""),
        ("Reset by the peer", ""),
        ("Connecting...", ""),
        ("Connection in progress. Please wait.", ""),
        ("Please try 1 minute later", ""),
        ("Login Error", ""),
        ("Successful", ""),
        ("Connected, waiting for image...", ""),
        ("Name", ""),
        ("Type", ""),
        ("Modified", ""),
        ("Size", ""),
        ("Show Hidden Files", ""),
        ("Receive", ""),
        ("Send", ""),
        ("Refresh File", ""),
        ("Local", ""),
        ("Remote", ""),
        ("Remote Computer", ""),
        ("Local Computer", ""),
        ("Confirm Delete", ""),
        ("Delete", ""),
        ("Properties", ""),
        ("Multi Select", ""),
        ("Select All", ""),
        ("Unselect All", ""),
        ("Empty Directory", ""),
        ("Not an empty directory", ""),
        ("Are you sure you want to delete this file?", ""),
        ("Are you sure you want to delete this empty directory?", ""),
        ("Are you sure you want to delete the file of this directory?", ""),
        ("Do this for all conflicts", ""),
        ("This is irreversible!", ""),
        ("Deleting", ""),
        ("files", ""),
        ("Waiting", ""),
        ("Finished", ""),
        ("Speed", ""),
        ("Custom Image Quality", ""),
        ("Privacy mode", ""),
        ("Block user input", ""),
        ("Unblock user input", ""),
        ("Adjust Window", ""),
        ("Original", ""),
        ("Shrink", ""),
        ("Stretch", ""),
        ("Scrollbar", ""),
        ("ScrollAuto", ""),
        ("Good image quality", ""),
        ("Balanced", ""),
        ("Optimize reaction time", ""),
        ("Custom", ""),
        ("Show remote cursor", ""),
        ("Show quality monitor", ""),
        ("Disable clipboard", ""),
        ("Lock after session end", ""),
        ("Insert", ""),
        ("Insert Lock", ""),
        ("Refresh", ""),
        ("ID does not exist", ""),
        ("Failed to connect to rendezvous server", ""),
        ("Please try later", ""),
        ("Remote desktop is offline", ""),
        ("Key mismatch", ""),
        ("Timeout", ""),
        ("Failed to connect to relay server", ""),
        ("Failed to connect via rendezvous server", ""),
        ("Failed to connect via relay server", ""),
        ("Failed to make direct connection to remote desktop", ""),
        ("Set Password", ""),
        ("OS Password", ""),
        ("install_tip", ""),
        ("Click to upgrade", ""),
        ("Click to download", ""),
        ("Click to update", ""),
        ("Configure", ""),
        ("config_acc", ""),
        ("config_screen", ""),
        ("Installing ...", ""),
        ("Install", ""),
        ("Installation", ""),
        ("Installation Path", ""),
        ("Create start menu shortcuts", ""),
        ("Create desktop icon", ""),
        ("agreement_tip", ""),
        ("Accept and Install", ""),
        ("End-user license agreement", ""),
        ("Generating ...", ""),
        ("Your installation is lower version.", ""),
        ("not_close_tcp_tip", ""),
        ("Listening ...", ""),
        ("Remote Host", ""),
        ("Remote Port", ""),
        ("Action", ""),
        ("Add", ""),
        ("Local Port", ""),
        ("Local Address", ""),
        ("Change Local Port", ""),
        ("setup_server_tip", ""),
        ("Too short, at least 6 characters.", ""),
        ("The confirmation is not identical.", ""),
        ("Permissions", ""),
        ("Accept", ""),
        ("Dismiss", ""),
        ("Disconnect", ""),
        ("Allow using keyboard and mouse", ""),
        ("Allow using clipboard", ""),
        ("Allow hearing sound", ""),
        ("Allow file copy and paste", ""),
        ("Connected", ""),
        ("Direct and encrypted connection", ""),
        ("Relayed and encrypted connection", ""),
        ("Direct and unencrypted connection", ""),
        ("Relayed and unencrypted connection", ""),
        ("Enter Remote ID", ""),
        ("Enter your password", ""),
        ("Logging in...", ""),
        ("Enable RDP session sharing", ""),
        ("Auto Login", ""),
        ("Enable Direct IP Access", ""),
        ("Rename", ""),
        ("Space", ""),
        ("Create Desktop Shortcut", ""),
        ("Change Path", ""),
        ("Create Folder", ""),
        ("Please enter the folder name", ""),
        ("Fix it", ""),
        ("Warning", ""),
        ("Login screen using Wayland is not supported", ""),
        ("Reboot required", ""),
        ("Unsupported display server ", ""),
        ("x11 expected", ""),
        ("Port", ""),
        ("Settings", ""),
        ("Username", ""),
        ("Invalid port", ""),
        ("Closed manually by the peer", ""),
        ("Enable remote configuration modification", ""),
        ("Run without install", ""),
        ("Always connected via relay", ""),
        ("Always connect via relay", ""),
        ("whitelist_tip", ""),
        ("Login", ""),
        ("Logout", ""),
        ("Tags", ""),
        ("Search ID", ""),
        ("Current Wayland display server is not supported", ""),
        ("whitelist_sep", ""),
        ("Add ID", ""),
        ("Add Tag", ""),
        ("Unselect all tags", ""),
        ("Network error", ""),
        ("Username missed", ""),
        ("Password missed", ""),
        ("Wrong credentials", ""),
        ("Edit Tag", ""),
        ("Unremember Password", ""),
        ("Favorites", ""),
        ("Add to Favorites", ""),
        ("Remove from Favorites", ""),
        ("Empty", ""),
        ("Invalid folder name", ""),
        ("Socks5 Proxy", ""),
        ("Hostname", ""),
        ("Discovered", ""),
        ("install_daemon_tip", ""),
        ("Remote ID", ""),
        ("Paste", ""),
        ("Paste here?", ""),
        ("Are you sure to close the connection?", ""),
        ("Download new version", ""),
        ("Touch mode", ""),
        ("Mouse mode", ""),
        ("One-Finger Tap", ""),
        ("Left Mouse", ""),
        ("One-Long Tap", ""),
        ("Two-Finger Tap", ""),
        ("Right Mouse", ""),
        ("One-Finger Move", ""),
        ("Double Tap & Move", ""),
        ("Mouse Drag", ""),
        ("Three-Finger vertically", ""),
        ("Mouse Wheel", ""),
        ("Two-Finger Move", ""),
        ("Canvas Move", ""),
        ("Pinch to Zoom", ""),
        ("Canvas Zoom", ""),
        ("Reset canvas", ""),
        ("No permission of file transfer", ""),
        ("Note", ""),
        ("Connection", ""),
        ("Share Screen", ""),
        ("CLOSE", ""),
        ("OPEN", ""),
        ("Chat", ""),
        ("Total", ""),
        ("items", ""),
        ("Selected", ""),
        ("Screen Capture", ""),
        ("Input Control", ""),
        ("Audio Capture", ""),
        ("File Connection", ""),
        ("Screen Connection", ""),
        ("Do you accept?", ""),
        ("Open System Setting", ""),
        ("How to get Android input permission?", ""),
        ("android_input_permission_tip1", ""),
        ("android_input_permission_tip2", ""),
        ("android_new_connection_tip", ""),
        ("android_service_will_start_tip", ""),
        ("android_stop_service_tip", ""),
        ("android_version_audio_tip", ""),
        ("android_start_service_tip", ""),
        ("Account", ""),
        ("Overwrite", ""),
        ("This file exists, skip or overwrite this file?", ""),
        ("Quit", ""),
        ("doc_mac_permission", ""),
        ("Help", ""),
        ("Failed", ""),
        ("Succeeded", ""),
        ("Someone turns on privacy mode, exit", ""),
        ("Unsupported", ""),
        ("Peer denied", ""),
        ("Please install plugins", ""),
        ("Peer exit", ""),
        ("Failed to turn off", ""),
        ("Turned off", ""),
        ("In privacy mode", ""),
        ("Out privacy mode", ""),
        ("Language", ""),
        ("Keep RustDesk background service", ""),
        ("Ignore Battery Optimizations", ""),
        ("android_open_battery_optimizations_tip", ""),
        ("Connection not allowed", ""),
        ("Legacy mode", ""),
        ("Map mode", ""),
        ("Translate mode", ""),
        ("Use temporary password", ""),
        ("Use permanent password", ""),
        ("Use both passwords", ""),
        ("Set permanent password", ""),
        ("Set temporary password length", ""),
        ("Enable Remote Restart", ""),
        ("Allow remote restart", ""),
        ("Restart Remote Device", ""),
        ("Are you sure you want to restart", ""),
        ("Restarting Remote Device", ""),
        ("remote_restarting_tip", ""),
        ("Copied", ""),
        ("Exit Fullscreen", ""),
        ("Fullscreen", ""),
        ("Mobile Actions", ""),
        ("Select Monitor", ""),
        ("Control Actions", ""),
        ("Display Settings", ""),
        ("Ratio", ""),
        ("Image Quality", ""),
        ("Scroll Style", ""),
        ("Show Menubar", ""),
        ("Hide Menubar", ""),
        ("Direct Connection", ""),
        ("Relay Connection", ""),
        ("Secure Connection", ""),
        ("Insecure Connection", ""),
        ("Scale original", ""),
        ("Scale adaptive", ""),
        ("General", ""),
        ("Security", ""),
        ("Account", ""),
        ("Theme", ""),
        ("Dark Theme", ""),
        ("Dark", ""),
        ("Light", ""),
        ("Follow System", ""),
        ("Enable hardware codec", ""),
        ("Unlock Security Settings", ""),
        ("Enable Audio", ""),
        ("Temporary Password Length", ""),
        ("Unlock Network Settings", ""),
        ("Server", ""),
        ("Direct IP Access", ""),
        ("Proxy", ""),
        ("Port", ""),
        ("Apply", ""),
        ("Disconnect all devices?", ""),
        ("Clear", ""),
        ("Audio Input Device", ""),
        ("Deny remote access", ""),
        ("Use IP Whitelisting", ""),
        ("Network", ""),
        ("Enable RDP", ""),
        ("Pin menubar", ""),
        ("Unpin menubar", ""),
        ("Recording", ""),
        ("Directory", ""),
        ("Automatically record incoming sessions", ""),
        ("Change", ""),
        ("Start session recording", ""),
        ("Stop session recording", ""),
        ("Enable Recording Session", ""),
        ("Allow recording session", ""),
        ("Enable LAN Discovery", ""),
        ("Deny LAN Discovery", ""),
        ("Write a message", ""),
        ("Prompt", ""),
        ("elevation_prompt", ""),
        ("uac_warning", ""),
        ("elevated_foreground_window_warning", ""),
        ("Disconnected", ""),
        ("Other", ""),
        ("Confirm before closing multiple tabs", ""),
        ("Keyboard Settings", ""),
        ("Custom", ""),
        ("Full Access", ""),
        ("Screen Share", ""),
        ("Wayland requires Ubuntu 21.04 or higher version.", ""),
        ("Wayland requires higher version of linux distro. Please try X11 desktop or change your OS.", ""),
        ("JumpLink", ""),
        ("Please Select the screen to be shared(Operate on the peer side).", ""),
        ("Switch Sides", ""),
        ("Please confirm if you want to share your desktop ?", ""),
    ].iter().cloned().collect();
}
