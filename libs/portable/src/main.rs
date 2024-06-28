#![windows_subsystem = "windows"]

use std::{
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    process::{Command, Stdio},
};

use bin_reader::BinaryReader;

use std::ptr::null_mut;
use winapi::um::winuser::{AllowSetForegroundWindow, FindWindowA, SetForegroundWindow, SW_SHOW};
use winapi::um::{
    shellapi::{ShellExecuteExA, SEE_MASK_NOASYNC, SHELLEXECUTEINFOA},
    winuser::SW_SHOWNORMAL,
};
use winapi::{shared::minwindef::LPARAM, um::winbase::STARTF_USESHOWWINDOW};

use std::{ffi::CString, mem::zeroed, ptr};
use winapi::um::processthreadsapi::{CreateProcessA, PROCESS_INFORMATION, STARTUPINFOA};
use winapi::um::winnt::LPSTR;
use winapi::um::winuser::{
    EnumWindows, GetWindowThreadProcessId, SetWindowPos, SWP_NOMOVE, SWP_NOSIZE,
};

use winapi::um::processthreadsapi::{CreateProcessW, STARTUPINFOW};
use winapi::um::winbase::CREATE_NO_WINDOW;
use winapi::um::winnt::LPWSTR;

pub mod bin_reader;
#[cfg(windows)]
mod ui;

#[cfg(windows)]
const APP_METADATA: &[u8] = include_bytes!("../app_metadata.toml");
#[cfg(not(windows))]
const APP_METADATA: &[u8] = &[];
const APP_METADATA_CONFIG: &str = "meta.toml";
const META_LINE_PREFIX_TIMESTAMP: &str = "timestamp = ";
const APP_PREFIX: &str = "rustdesk";
const APPNAME_RUNTIME_ENV_KEY: &str = "RUSTDESK_APPNAME";

fn is_timestamp_matches(dir: &PathBuf, ts: &mut u64) -> bool {
    let Ok(app_metadata) = std::str::from_utf8(APP_METADATA) else {
        return true;
    };
    for line in app_metadata.lines() {
        if line.starts_with(META_LINE_PREFIX_TIMESTAMP) {
            if let Ok(stored_ts) = line.replace(META_LINE_PREFIX_TIMESTAMP, "").parse::<u64>() {
                *ts = stored_ts;
                break;
            }
        }
    }
    if *ts == 0 {
        return true;
    }

    if let Ok(content) = std::fs::read_to_string(dir.join(APP_METADATA_CONFIG)) {
        for line in content.lines() {
            if line.starts_with(META_LINE_PREFIX_TIMESTAMP) {
                if let Ok(stored_ts) = line.replace(META_LINE_PREFIX_TIMESTAMP, "").parse::<u64>() {
                    return *ts == stored_ts;
                }
            }
        }
    }
    false
}

fn write_meta(dir: &PathBuf, ts: u64) {
    let meta_file = dir.join(APP_METADATA_CONFIG);
    if ts != 0 {
        let content = format!("{}{}", META_LINE_PREFIX_TIMESTAMP, ts);
        // Ignore is ok here
        let _ = std::fs::write(meta_file, content);
    }
}

fn setup(reader: BinaryReader, dir: Option<PathBuf>, clear: bool) -> Option<PathBuf> {
    let dir = if let Some(dir) = dir {
        dir
    } else {
        // home dir
        if let Some(dir) = dirs::data_local_dir() {
            dir.join(APP_PREFIX)
        } else {
            eprintln!("not found data local dir");
            return None;
        }
    };

    let mut ts = 0;
    if clear || !is_timestamp_matches(&dir, &mut ts) {
        std::fs::remove_dir_all(&dir).ok();
    }
    for file in reader.files.iter() {
        file.write_to_file(&dir);
    }
    write_meta(&dir, ts);
    #[cfg(windows)]
    windows::copy_runtime_broker(&dir);
    #[cfg(linux)]
    reader.configure_permission(&dir);
    Some(dir.join(&reader.exe))
}

unsafe extern "system" fn enum_windows_callback(
    hwnd: winapi::shared::windef::HWND,
    lParam: LPARAM,
) -> winapi::shared::minwindef::BOOL {
    let mut process_id: winapi::shared::minwindef::DWORD = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id as *mut _);
    if process_id == lParam as winapi::shared::minwindef::DWORD {
        SetForegroundWindow(hwnd);
        // SetWindowPos(
        //     hwnd,
        //     winapi::um::winuser::HWND_TOPMOST,
        //     0,
        //     0,
        //     0,
        //     0,
        //     SWP_NOMOVE | SWP_NOSIZE,
        // );
        return 0; // 返回0以停止枚举
    }
    1 // 返回1以继续枚举
}

fn execute(path: PathBuf, args: Vec<String>) {
    println!("executing {}", path.display());
    // setup env
    let exe = std::env::current_exe().unwrap_or_default();
    let exe_name = exe.file_name().unwrap_or_default();
    // run executable
    let mut cmd = Command::new(path);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
    }
    let child = cmd
        .env(APPNAME_RUNTIME_ENV_KEY, exe_name)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();
    if let Ok(child) = child {
        let process_id = child.id();
        unsafe {
            // std::thread::sleep(std::time::Duration::from_millis(1000));
            // EnumWindows(Some(enum_windows_callback), process_id as LPARAM);
            AllowSetForegroundWindow(process_id);
        }
    }

    // unsafe {
    //     std::thread::sleep(std::time::Duration::from_millis(1000));
    //     let window_title = "RustDesk"; // 替换为你的窗口标题
    //     let window_title_c = CString::new(window_title).unwrap();
    //     let hwnd = FindWindowA(null_mut(), window_title_c.as_ptr());
    //     if !hwnd.is_null() {
    //         SetForegroundWindow(hwnd);
    //     }
    // }
}

// fn execute(path: PathBuf, args: Vec<String>) {
//     println!("executing {}", path.display());
//     let operation = CString::new("open").unwrap();
//     let file = CString::new(path.to_str().unwrap()).unwrap();
//     let parameters = CString::new(args.join(" ")).unwrap();

//     let mut sei = SHELLEXECUTEINFOA {
//         cbSize: std::mem::size_of::<SHELLEXECUTEINFOA>() as u32,
//         fMask: SEE_MASK_NOASYNC,
//         hwnd: null_mut(),
//         lpVerb: operation.as_ptr(),
//         lpFile: file.as_ptr(),
//         lpParameters: parameters.as_ptr(),
//         lpDirectory: null_mut(),
//         nShow: SW_SHOWNORMAL,
//         hInstApp: null_mut(),
//         lpIDList: null_mut(),
//         lpClass: null_mut(),
//         hkeyClass: null_mut(),
//         dwHotKey: 0,
//         hMonitor: null_mut(),
//         hProcess: null_mut(),
//     };

//     unsafe {
//         if ShellExecuteExA(&mut sei) != 0 {
//             // 等待一段时间，让应用窗口创建
//             std::thread::sleep(std::time::Duration::from_millis(1000));
//             // 尝试将窗口设置为前台
//             let window_title = "RustDesk"; // 替换为你的窗口标题
//             let window_title_c = CString::new(window_title).unwrap();
//             let hwnd = FindWindowA(null_mut(), window_title_c.as_ptr());
//             if !hwnd.is_null() {
//                 SetForegroundWindow(hwnd);
//             }
//         } else {
//             println!("Failed to start process");
//         }
//     }
// }

// pub fn wide_string(s: &str) -> Vec<u16> {
//     std::ffi::OsStr::new(s)
//         .encode_wide()
//         .chain(Some(0).into_iter())
//         .collect()
// }

// fn execute(path: PathBuf, args: Vec<String>) {
//     let application_name = wide_string(&path.to_string_lossy());
//     let mut command_line = wide_string(&args.join(" "));

//     let mut startup_info = STARTUPINFOW {
//         cb: std::mem::size_of::<STARTUPINFOW>() as u32,
//         lpReserved: null_mut(),
//         lpDesktop: null_mut(),
//         lpTitle: null_mut(),
//         dwX: 0,
//         dwY: 0,
//         dwXSize: 0,
//         dwYSize: 0,
//         dwXCountChars: 0,
//         dwYCountChars: 0,
//         dwFillAttribute: 0,
//         dwFlags: STARTF_USESHOWWINDOW,
//         wShowWindow: SW_SHOWNORMAL as u16,
//         cbReserved2: 0,
//         lpReserved2: null_mut(),
//         hStdInput: null_mut(),
//         hStdOutput: null_mut(),
//         hStdError: null_mut(),
//     };
//     let mut process_information = PROCESS_INFORMATION {
//         hProcess: null_mut(),
//         hThread: null_mut(),
//         dwProcessId: 0,
//         dwThreadId: 0,
//     };

//     unsafe {
//         CreateProcessW(
//             application_name.as_ptr() as LPWSTR,
//             command_line.as_mut_ptr(), // Command line
//             null_mut(),                // Process handle not inheritable
//             null_mut(),                // Thread handle not inheritable
//             false as i32,              // Set handle inheritance to FALSE
//             CREATE_NO_WINDOW,          // Creation flags
//             null_mut(),                // Use parent's environment block
//             null_mut(),                // Use parent's starting directory
//             &mut startup_info,         // Pointer to STARTUPINFO structure
//             &mut process_information,  // Pointer to PROCESS_INFORMATION structure
//         );
//     }

//     // Attempt to bring the process window to the foreground
//     // This requires the window handle, which is not directly available from CreateProcessW
//     // You might need additional logic to find the window handle based on the process information
//     // For example, EnumWindows and a callback to match the process ID with the window
// }

fn main() {
    let mut args = Vec::new();
    let mut arg_exe = Default::default();
    let mut i = 0;
    for arg in std::env::args() {
        if i == 0 {
            arg_exe = arg.clone();
        } else {
            args.push(arg);
        }
        i += 1;
    }
    let click_setup = args.is_empty() && arg_exe.to_lowercase().ends_with("install.exe");
    let quick_support = args.is_empty() && arg_exe.to_lowercase().ends_with("qs.exe");

    #[cfg(windows)]
    if args.is_empty() {
        ui::setup();
    }

    let reader = BinaryReader::default();
    if let Some(exe) = setup(
        reader,
        None,
        click_setup || args.contains(&"--silent-install".to_owned()),
    ) {
        if click_setup {
            args = vec!["--install".to_owned()];
        } else if quick_support {
            args = vec!["--quick_support".to_owned()];
        }
        execute(exe, args);
    }
}

#[cfg(windows)]
mod windows {
    use std::{fs, os::windows::process::CommandExt, path::PathBuf, process::Command};

    // Used for privacy mode(magnifier impl).
    pub const RUNTIME_BROKER_EXE: &'static str = "C:\\Windows\\System32\\RuntimeBroker.exe";
    pub const WIN_TOPMOST_INJECTED_PROCESS_EXE: &'static str = "RuntimeBroker_rustdesk.exe";

    pub(super) fn copy_runtime_broker(dir: &PathBuf) {
        let src = RUNTIME_BROKER_EXE;
        let tgt = WIN_TOPMOST_INJECTED_PROCESS_EXE;
        let target_file = dir.join(tgt);
        if target_file.exists() {
            if let (Ok(src_file), Ok(tgt_file)) = (fs::read(src), fs::read(&target_file)) {
                let src_md5 = format!("{:x}", md5::compute(&src_file));
                let tgt_md5 = format!("{:x}", md5::compute(&tgt_file));
                if src_md5 == tgt_md5 {
                    return;
                }
            }
        }
        let _allow_err = Command::new("taskkill")
            .args(&["/F", "/IM", "RuntimeBroker_rustdesk.exe"])
            .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
            .output();
        let _allow_err = std::fs::copy(src, &format!("{}\\{}", dir.to_string_lossy(), tgt));
    }
}
