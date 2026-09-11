use eframe::egui;

#[cfg(windows)]
pub fn load_fonts(ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
    use egui::{FontData, FontDefinitions};
    use std::fs;
    const FONT_PATH: &str = "C:\\Windows\\Fonts\\segoeui.ttf";

    let file_content = fs::read(FONT_PATH)?;
    let font_data = FontData::from_owned(file_content);

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("Segoe UI".to_string(), font_data.into());

    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "Segoe UI".to_owned());

    ctx.set_fonts(fonts);
    Ok(())
}

#[cfg(not(windows))]
pub fn load_fonts(_ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
