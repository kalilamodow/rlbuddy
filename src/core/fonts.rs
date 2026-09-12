use eframe::egui;
use egui::{FontData, FontDefinitions};
use std::fs;

fn insert_font_data(ctx: &egui::Context, font_data: FontData, emoji_font_data: Option<FontData>) {
    let mut fonts = FontDefinitions::default();

    if let Some(emoji_font_data) = emoji_font_data {
        fonts
            .font_data
            .insert("emoji".to_string(), emoji_font_data.into());

        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "emoji".to_string());
    }

    fonts.font_data.insert("text".to_string(), font_data.into());
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "text".to_string());

    ctx.set_fonts(fonts);
}

#[cfg(windows)]
pub fn load_fonts(ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
    const TEXT_FONT_PATH: &str = "C:\\Windows\\Fonts\\segoeui.ttf";
    const EMOJI_FONT_PATH: &str = "C:\\Windows\\Fonts\\seguisym.ttf";

    let file_content = fs::read(TEXT_FONT_PATH)?;
    let font_data = FontData::from_owned(file_content);
    let emoji_font_data = fs::read(EMOJI_FONT_PATH).map(FontData::from_owned);
    if let Err(error) = &emoji_font_data {
        eprintln!("error while loading emoji font: {error:?}");
    }

    insert_font_data(ctx, font_data, emoji_font_data.ok());

    Ok(())
}

#[cfg(not(windows))]
pub fn load_fonts(ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
    let mut database = fontdb::Database::new();
    database.load_system_fonts();

    let font_id = database
        .query(&fontdb::Query {
            families: &[
                fontdb::Family::Name("DejaVu Sans"),
                fontdb::Family::Name("Noto Sans"),
                fontdb::Family::Name("Liberation Sans"),
            ],
            ..fontdb::Query::default()
        })
        .ok_or("no sans serif font")?;
    let face = database.face(font_id).ok_or("whered it go")?;

    let path = match &face.source {
        fontdb::Source::File(path) => path,
        _ => return Err("theres no file for the font help".into()),
    };

    let file_content = fs::read(path)?;
    let font_data = FontData::from_owned(file_content);
    insert_font_data(ctx, font_data, None);

    Ok(())
}
