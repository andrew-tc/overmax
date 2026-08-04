use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const DESKTOP_FILE: &str = "overmax.desktop";
const ICON_BYTES: &[u8] = include_bytes!("../../../../assets/overmax.ico");

pub fn install(app_dir: &Path) -> Result<PathBuf, String> {
    let data_home = xdg_data_home(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )?;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    install_at(&data_home, &exe, app_dir, ICON_BYTES)
}

fn xdg_data_home(xdg_data_home: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathBuf, String> {
    if let Some(path) = xdg_data_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path);
    }

    home.map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(".local/share"))
        .ok_or_else(|| "XDG_DATA_HOME 또는 HOME을 찾을 수 없습니다.".to_string())
}

fn install_at(
    data_home: &Path,
    exe: &Path,
    app_dir: &Path,
    icon_bytes: &[u8],
) -> Result<PathBuf, String> {
    let assets = app_dir.join("assets");
    let icon_path = assets.join("overmax.ico");
    let exe = desktop_path(exe)?;
    let app_dir = desktop_path(app_dir)?;
    let icon = desktop_path(&icon_path)?;
    let applications = data_home.join("applications");
    std::fs::create_dir_all(&assets).map_err(|error| error.to_string())?;
    std::fs::write(icon, icon_bytes).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&applications).map_err(|error| error.to_string())?;

    let destination = applications.join(DESKTOP_FILE);
    std::fs::write(
        &destination,
        format!(
            "[Desktop Entry]\nType=Application\nName=Overmax\nComment=DJMAX RESPECT V overlay\nExec=\"{exe}\"\nPath={app_dir}\nIcon={icon}\nTerminal=false\nCategories=Game;\n"
        ),
    )
    .map_err(|error| error.to_string())?;
    Ok(destination)
}

fn desktop_path(path: &Path) -> Result<&str, String> {
    let path = path
        .to_str()
        .ok_or_else(|| "UTF-8이 아닌 설치 경로는 지원하지 않습니다.".to_string())?;
    if path.chars().any(|character| {
        matches!(
            character,
            '\n' | '\r' | '\t' | '"' | '`' | '$' | '\\' | '%' | '='
        )
    }) {
        return Err("설치 경로에 .desktop에서 지원하지 않는 문자가 있습니다.".to_string());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{install_at, xdg_data_home};

    #[test]
    fn installs_launcher_with_working_directory() {
        let temp = std::env::temp_dir().join(format!("overmax-desktop-{}", std::process::id()));
        let app_dir = temp.join("Over Max");
        let exe = app_dir.join("overmax");
        let destination =
            install_at(&temp, &exe, &app_dir, b"icon").expect("install desktop entry");
        let contents = std::fs::read_to_string(destination).expect("read desktop entry");
        let icon = app_dir.join("assets/overmax.ico");

        assert!(contents.contains(&format!("Exec=\"{}\"", exe.display())));
        assert!(contents.contains(&format!("Path={}", app_dir.display())));
        assert!(contents.contains(&format!("Icon={}", icon.display())));
        assert!(contents.contains("Terminal=false"));
        assert_eq!(std::fs::read(icon).unwrap(), b"icon");
        let home = std::env::temp_dir().join("overmax-home");
        assert_eq!(
            xdg_data_home(None, Some(home.as_os_str())).unwrap(),
            home.join(".local/share")
        );

        let _ = std::fs::remove_dir_all(temp);
    }
}
