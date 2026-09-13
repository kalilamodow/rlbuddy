use eframe::egui;
use egui::{FontData, FontDefinitions};
use std::fs;
use std::io;

fn insert_font_data(ctx: &egui::Context, font_datas: Vec<(String, FontData)>) {
    let mut fonts = FontDefinitions::default();

    for (name, data) in font_datas.into_iter() {
        fonts.font_data.insert(name.clone(), data.into());
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, name);
    }

    ctx.set_fonts(fonts);
}

fn by_path((name, path): (&str, &str)) -> io::Result<(String, FontData)> {
    let file_content = fs::read(path)?;
    let font_data = FontData::from_owned(file_content);
    Ok((name.to_owned(), font_data))
}

#[cfg(windows)]
pub fn load_fonts(ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
    macro_rules! windows_font_name_and_path {
        ($font_name: literal) => {
            ($font_name, concat!("C:\\Windows\\Fonts\\", $font_name))
        };
    }

    // lower->higher priority
    let fonts: Vec<_> = vec![
        by_path(windows_font_name_and_path!("YuGothM.ttc")),
        by_path(windows_font_name_and_path!("seguisym.ttf")),
        by_path(windows_font_name_and_path!("segoeui.ttf")),
    ]
    .into_iter()
    .flatten()
    .collect();

    insert_font_data(ctx, fonts);

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

    insert_font_data(
        ctx,
        vec![by_path((&face.post_script_name, path.to_str().unwrap())).unwrap()],
    );

    Ok(())
}
