use std::{
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui;

use crate::map_loader::map_card_widget::MapCardWidget;

#[derive(Debug)]
struct MapSearchResult {
    pub id: u16,
    pub title: String,
    pub author: String,
    pub description: String,
    pub image_url: String,
}

impl MapSearchResult {
    fn render(&self, ui: &mut egui::Ui, download: impl Fn(u16), allow_download: bool) {
        ui.add(MapCardWidget::new(
            &self.title,
            Some(&self.author),
            Some(&self.description),
            Some(&self.image_url),
            |ui| {
                if ui
                    .add_enabled(allow_download, egui::Button::new("Download"))
                    .clicked()
                    && allow_download
                {
                    download(self.id);
                }
            },
            false,
            true,
        ));
    }
}

pub struct MapDownloaderWidget {
    search_text: String,
    results: Arc<Mutex<Option<Vec<MapSearchResult>>>>, // none if loading
    currently_downloading: Option<MapSearchResult>,
}

impl MapDownloaderWidget {
    pub fn new() -> Self {
        Self {
            search_text: "".into(),
            results: Arc::new(Mutex::new(Some(Vec::new()))),
            currently_downloading: None,
        }
    }

    fn search(&self) {
        let results_handle = self.results.clone();
        let search_text_urlencoded = urlencoding::encode(&self.search_text).into_owned();

        thread::spawn(move || {
            let response = ureq::get(format!(
                "https://bakkesplugins.com/maps?search={}",
                search_text_urlencoded
            ))
            .call()
            .unwrap();

            let response = response.into_body().read_to_string().unwrap();
            let parsed = parse_bakkesplugins_response(&response);

            let mut results = results_handle.lock().unwrap();
            *results = Some(parsed);
        });
    }

    fn download(&self, id: u16) {
        println!("Downloading {id}");
    }
}

impl egui::Widget for &mut MapDownloaderWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.strong("Map downloader");

            ui.horizontal(|ui| {
                let mut results = self.results.lock().unwrap();
                ui.add_enabled(
                    results.is_some(),
                    egui::TextEdit::singleline(&mut self.search_text),
                );

                if ui
                    .add_enabled(results.is_some(), egui::Button::new("Search"))
                    .clicked()
                {
                    *results = None;

                    self.search();
                    return;
                }
            });

            {
                let mut results_guard = self.results.lock().unwrap();

                if let Some(results) = &mut *results_guard {
                    if !results.is_empty() && ui.button("Hide results").clicked() {
                        results.clear();
                        return;
                    }

                    if results.is_empty() {
                        ui.label("No results");
                    }

                    for result in results {
                        ui.add_space(4.0);
                        result.render(
                            ui,
                            |id| self.download(id),
                            self.currently_downloading.is_none(),
                        );
                    }
                } else {
                    ui.add_space(8.0);
                    ui.spinner();
                    ui.small("Results can take up to 10 seconds to load");
                }
            }
        })
        .response
    }
}

fn array_if_all_some<T, const N: usize>(array: [Option<T>; N]) -> Option<[T; N]> {
    let values: Vec<T> = array.into_iter().collect::<Option<Vec<_>>>()?;
    values.try_into().ok()
}

fn search_parse<'a, const N_RULES: usize>(
    buffer: &'a str,
    rules: [(&'a str, &'a str); N_RULES],
) -> Vec<[&'a str; N_RULES]> {
    let mut remaining = buffer;
    let mut output = vec![];

    loop {
        let mut current: [Option<&'a str>; N_RULES] = [None; N_RULES];

        for (index, rule) in rules.iter().enumerate() {
            let Some(start) = remaining.find(rule.0) else {
                eprintln!("failed to find rule {rule:?} ({}th find)", output.len());
                break;
            };

            remaining = &remaining[(start + rule.0.len())..];
            let Some(end) = remaining.find(rule.1) else {
                eprintln!("failed to find rule end {rule:?} ({}th find)", output.len());
                break;
            };

            let value = &remaining[..end];
            current[index] = Some(value);

            remaining = &remaining[end + 1..];
        }

        if let Some(current) = array_if_all_some(current) {
            output.push(current);
        } else {
            break;
        }
    }

    output
}

fn parse_bakkesplugins_response(response: &str) -> Vec<MapSearchResult> {
    let result = search_parse(
        response,
        [
            // map id
            ("<a href=\"/maps/", "\""),
            // image
            (
                "olute;height:100%;width:100%;left:0;top:0;right:0;bottom:0;color:transparent\" src=\"",
                "\"",
            ),
            // title
            (
                "<h2 class=\"mb-3 line-clamp-2 text-lg font-semibold text-gray-900 transition group-hover:text-blue-600\">",
                "</h2>",
            ),
            // author
            (
                "<span class=\"truncate text-sm font-medium\" title=\"",
                "\"",
            ),
            // description
            (
                "<p class=\"mb-3 line-clamp-3 text-sm leading-relaxed text-gray-600\">",
                "</p>",
            ),
        ],
    );

    result
        .iter()
        .filter_map(|result| {
            Some(MapSearchResult {
                id: result[0].parse().ok()?,
                title: result[2].to_owned(),
                author: result[3].to_owned(),
                description: result[4].split('\n').nth(0).unwrap_or_default().to_owned(),
                image_url: result[1].to_owned(),
            })
        })
        .collect()
}
