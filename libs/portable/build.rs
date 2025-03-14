#[cfg(windows)]
use regex::Regex;
#[cfg(windows)]
use std::{
    env, fs,
    io::{Read, Write},
    path::Path,
};

fn main() {
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../res/icon.ico")
            .set_language(winapi::um::winnt::MAKELANGID(
                winapi::um::winnt::LANG_ENGLISH,
                winapi::um::winnt::SUBLANG_ENGLISH_US,
            ))
            .set_manifest_file("../../res/manifest.xml");
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
    // #[cfg(windows)]
    // modify_app_prefix();
}

#[cfg(windows)]
fn modify_app_prefix() {
    println!("cargo:rerun-if-env-changed=CLIENT_TYPE");
    println!("cargo:rerun-if-changed=src/main.rs");

    let client_type = env::var("CLIENT_TYPE").unwrap_or_else(|_| "full".to_string());

    let app_prefix_suffix = match client_type.to_lowercase().as_str() {
        "host" => "host",
        "client" => "client",
        "sos" => "sos",
        _ => "full",
    };

    let new_app_prefix = format!("rustdesk-{}", app_prefix_suffix);

    let main_rs_path = Path::new("src/main.rs");
    let mut main_rs_content = String::new();
    let mut file = fs::File::open(main_rs_path).expect("Failed to open main.rs");
    file.read_to_string(&mut main_rs_content)
        .expect("Failed to read main.rs");

    let re = Regex::new(r#"const APP_PREFIX: &str = "rustdesk(?:_\w+)*";"#).expect("Invalid regex");

    let new_content = re.replace(
        &main_rs_content,
        &format!("const APP_PREFIX: &str = \"{}\";", new_app_prefix),
    );

    let mut file = fs::File::create(main_rs_path).expect("Failed to create main.rs");
    file.write_all(new_content.as_bytes())
        .expect("Failed to write to main.rs");

    println!("cargo:warning=Building with APP_PREFIX={}", new_app_prefix);
}
