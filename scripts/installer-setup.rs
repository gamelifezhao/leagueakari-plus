#![windows_subsystem = "windows"]

use std::{
    env,
    error::Error,
    ffi::{c_void, OsStr},
    fs::{self, File},
    io::{self, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const MB_OK: u32 = 0x00000000;
const MB_ICONERROR: u32 = 0x00000010;
const MB_ICONINFORMATION: u32 = 0x00000040;
const BIF_RETURNONLYFSDIRS: u32 = 0x00000001;
const BIF_EDITBOX: u32 = 0x00000010;
const BIF_NEWDIALOGSTYLE: u32 = 0x00000040;

static APP_EXE: &[u8] = include_bytes!(env!("LEAGUEAKARI_APP_EXE"));
static PROBE_EXE: &[u8] = include_bytes!(env!("LEAGUEAKARI_PROBE_EXE"));
static README: &[u8] = include_bytes!(env!("LEAGUEAKARI_README"));

#[repr(C)]
struct BrowseInfoW {
    hwnd_owner: *mut c_void,
    pidl_root: *const c_void,
    psz_display_name: *mut u16,
    lpsz_title: *const u16,
    ul_flags: u32,
    lpfn: isize,
    l_param: isize,
    i_image: i32,
}

#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn SHBrowseForFolderW(info: *mut BrowseInfoW) -> *mut c_void;
    fn SHGetPathFromIDListW(pidl: *const c_void, path: *mut u16) -> i32;
}

#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemFree(ptr: *mut c_void);
}

type AppResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    if let Err(err) = run() {
        let _ = append_log(&format!("ERROR: {err}"));
        show_message(
            "LeagueAkari Plus",
            &format!(
                "Installation failed:\n{}\n\nLog: {}",
                err,
                log_path().display()
            ),
            MB_OK | MB_ICONERROR,
        );
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    append_log("install started")?;

    let Some(install_dir) = choose_install_dir()? else {
        append_log("install canceled")?;
        return Ok(());
    };

    append_log(&format!("install_dir={}", install_dir.display()))?;
    fs::create_dir_all(&install_dir)?;
    write_payload(&install_dir.join("LeagueAkari Plus.exe"), APP_EXE)?;
    write_payload(&install_dir.join("leagueakari-probe.exe"), PROBE_EXE)?;
    write_payload(&install_dir.join("README.txt"), README)?;
    create_shortcuts(&install_dir)?;

    show_message(
        "LeagueAkari Plus",
        &format!(
            "LeagueAkari Plus has been installed to:\n{}",
            install_dir.display()
        ),
        MB_OK | MB_ICONINFORMATION,
    );
    append_log("install completed")?;
    Ok(())
}

fn choose_install_dir() -> AppResult<Option<PathBuf>> {
    if let Some(path) = install_dir_arg() {
        return Ok(Some(path));
    }

    if let Some(value) = env::var_os("LEAGUEAKARI_INSTALL_DIR") {
        if !value.is_empty() {
            return Ok(Some(PathBuf::from(value)));
        }
    }

    choose_install_dir_dialog()
}

fn install_dir_arg() -> Option<PathBuf> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].to_string_lossy();
        if let Some(value) = arg.strip_prefix("--install-dir=") {
            let mut parts = vec![value.to_string()];
            index += 1;
            while index < args.len() {
                let value = args[index].to_string_lossy();
                if value.starts_with("--") {
                    break;
                }
                parts.push(value.to_string());
                index += 1;
            }

            return Some(PathBuf::from(parts.join(" ")));
        }

        if arg == "--install-dir" {
            let mut parts: Vec<String> = Vec::new();
            index += 1;
            while index < args.len() {
                let value = args[index].to_string_lossy();
                if value.starts_with("--") {
                    break;
                }
                parts.push(value.to_string());
                index += 1;
            }

            if !parts.is_empty() {
                return Some(PathBuf::from(parts.join(" ")));
            }
        }

        index += 1;
    }
    None
}

fn has_flag(name: &str) -> bool {
    env::args_os().skip(1).any(|arg| arg == name)
}

fn choose_install_dir_dialog() -> AppResult<Option<PathBuf>> {
    let title = wide("Choose LeagueAkari Plus install folder");
    let mut display_name = [0u16; 260];
    let mut info = BrowseInfoW {
        hwnd_owner: std::ptr::null_mut(),
        pidl_root: std::ptr::null(),
        psz_display_name: display_name.as_mut_ptr(),
        lpsz_title: title.as_ptr(),
        ul_flags: BIF_RETURNONLYFSDIRS | BIF_EDITBOX | BIF_NEWDIALOGSTYLE,
        lpfn: 0,
        l_param: 0,
        i_image: 0,
    };

    let pidl = unsafe { SHBrowseForFolderW(&mut info) };
    if pidl.is_null() {
        return Ok(None);
    }

    let mut path = [0u16; 260];
    let ok = unsafe { SHGetPathFromIDListW(pidl, path.as_mut_ptr()) };
    unsafe { CoTaskMemFree(pidl) };

    if ok == 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "selected folder was invalid").into());
    }

    let len = path.iter().position(|ch| *ch == 0).unwrap_or(path.len());
    let selected = String::from_utf16_lossy(&path[..len]);
    if selected.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(selected)))
}

fn write_payload(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    append_log(&format!("wrote {} bytes: {}", bytes.len(), path.display()))?;
    Ok(())
}

fn create_shortcuts(install_dir: &Path) -> AppResult<()> {
    let app_exe = install_dir.join("LeagueAkari Plus.exe");
    let desktop_shortcut = special_folder("DesktopDirectory").join("LeagueAkari Plus.lnk");
    let start_shortcut = special_folder("Programs").join("LeagueAkari Plus.lnk");
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$appName = 'LeagueAkari Plus'
$appExe = '{}'
$installDir = '{}'
$shell = New-Object -ComObject WScript.Shell
$shortcutPaths = @(
  [System.IO.Path]::Combine([Environment]::GetFolderPath('DesktopDirectory'), "$appName.lnk"),
  [System.IO.Path]::Combine([Environment]::GetFolderPath('Programs'), "$appName.lnk")
)
foreach ($shortcutPath in $shortcutPaths) {{
  if ([System.IO.File]::Exists($shortcutPath)) {{
    [System.IO.File]::Delete($shortcutPath)
  }}
  $shortcut = $shell.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = $appExe
  $shortcut.WorkingDirectory = $installDir
  $shortcut.IconLocation = "$appExe,0"
  $shortcut.Description = $appName
  $shortcut.Save()
}}
"#,
        ps_quote(app_exe.to_string_lossy().as_ref()),
        ps_quote(install_dir.to_string_lossy().as_ref())
    );

    let output = powershell(&script)?;
    if output.status.success() {
        wait_for_file(&desktop_shortcut)?;
        wait_for_file(&start_shortcut)?;
        append_log("shortcuts created")?;
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "failed to create shortcuts: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    )
    .into())
}

fn special_folder(name: &str) -> PathBuf {
    match name {
        "DesktopDirectory" => env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("Desktop"),
        "Programs" => env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs"),
        _ => env::temp_dir(),
    }
}

fn wait_for_file(path: &Path) -> AppResult<()> {
    for _ in 0..20 {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("shortcut was not created: {}", path.display()),
    )
    .into())
}

fn powershell(script: &str) -> io::Result<std::process::Output> {
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

fn append_log(line: &str) -> io::Result<()> {
    let mut file = File::options().create(true).append(true).open(log_path())?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn log_path() -> PathBuf {
    env::temp_dir().join("LeagueAkariPlus-install.log")
}

fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn show_message(title: &str, body: &str, kind: u32) {
    if has_flag("--quiet") || env::var_os("LEAGUEAKARI_INSTALL_QUIET").is_some() {
        return;
    }

    let title = wide(title);
    let body = wide(body);
    unsafe {
        let _ = MessageBoxW(std::ptr::null_mut(), body.as_ptr(), title.as_ptr(), kind);
    }
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}
