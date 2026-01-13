use std::process::Command;

use super::UiLinkOpener;

pub struct DesktopLinkOpener;

impl UiLinkOpener for DesktopLinkOpener {
    fn open_url(&self, url: &str) {
        let url = url.trim();
        if url.is_empty() {
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open").args(["-g", url]).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open").arg(url).spawn();
        }
    }
}
