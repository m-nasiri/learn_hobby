use std::sync::Arc;

mod macos;

pub trait UiLinkOpener: Send + Sync {
    fn open_url(&self, url: &str);
}

pub type LinkOpenerRef = Arc<dyn UiLinkOpener>;

pub use macos::DesktopLinkOpener;
