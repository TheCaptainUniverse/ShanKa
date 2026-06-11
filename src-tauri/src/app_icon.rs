use tauri::image::Image;

const APP_ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");

pub fn load() -> tauri::Result<Image<'static>> {
    Image::from_bytes(APP_ICON_PNG).map(Image::to_owned)
}
