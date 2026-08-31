use crate::map_loader::{
    MapLoaderService,
    map_card_widget::MapCardWidget,
    service::{CustomMapInfo, MapLoaderCommand},
};
use eframe::egui;
use std::{
    io::Read as _,
    sync::{Arc, Mutex, mpsc},
    thread,
};

#[derive(Debug, Clone, Copy)]
enum DownloadProgressState {
    FetchingInfo,
    DownloadingZip,
    DownloadingImage,
}

impl DownloadProgressState {
    fn as_str(self) -> &'static str {
        match self {
            Self::FetchingInfo => "Fetching manifest...",
            Self::DownloadingZip => "Downloading map...",
            Self::DownloadingImage => "Downloading thumbnail...",
        }
    }
}

#[derive(Debug, Clone)]
struct MapSearchResult {
    pub id: u16,
    pub title: String,
    pub author: String,
    pub description: String,
    pub image_url: String,
}

impl MapSearchResult {
    fn render(
        &self,
        ui: &mut egui::Ui,
        download_progress: Option<(DownloadProgressState, f32)>,
    ) -> bool {
        let mut download_btn_pressed = false;

        ui.add(MapCardWidget::new(
            &self.title,
            Some(&self.author),
            Some(&self.description),
            Some(&self.image_url),
            |ui| {
                if let Some(download_progress) = download_progress {
                    ui.add(
                        egui::ProgressBar::new(download_progress.1)
                            .text(download_progress.0.as_str()),
                    );
                } else if ui.button("Download").clicked() {
                    download_btn_pressed = true;
                }
            },
            false,
            true,
        ));

        download_btn_pressed
    }
}

pub struct MapDownloaderWidget {
    search_text: String,
    results: Arc<Mutex<Option<Vec<MapSearchResult>>>>, // none if loading
    currently_downloading: Arc<Mutex<Option<(MapSearchResult, DownloadProgressState, f32)>>>, // downloading, progress %
    service_command_sender: mpsc::Sender<MapLoaderCommand>,
}

impl MapDownloaderWidget {
    pub fn new(service: &MapLoaderService) -> Self {
        Self {
            search_text: "".into(),
            results: Arc::new(Mutex::new(Some(Vec::new()))),
            currently_downloading: Arc::default(),
            service_command_sender: service.sender(),
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
        let results_guard = self.results.lock().unwrap();
        let Some(results) = results_guard.as_ref() else {
            eprintln!("downloading from not loaded list");
            return;
        };

        let Some(map_to_download) = results.iter().find(|m| m.id == id).cloned() else {
            eprintln!("no loaded map found with id {id}");
            return;
        };

        let currently_downloading_handle = self.currently_downloading.clone();
        {
            let mut currently_downloading = currently_downloading_handle.lock().unwrap();
            *currently_downloading =
                Some((map_to_download, DownloadProgressState::FetchingInfo, 0.0));
        }
        let command_sender = self.service_command_sender.clone();

        thread::spawn(move || {
            let update_progress = |new_state, new_progress| {
                let mut currently_downloading = currently_downloading_handle.lock().unwrap();
                if let Some((_, state, progress)) = currently_downloading.as_mut() {
                    *state = new_state;
                    *progress = new_progress;
                }
            };

            update_progress(DownloadProgressState::FetchingInfo, 0.1);

            // first we have to get the url to the zip file from the map information page
            let map_info_response = ureq::get(format!("https://bakkesplugins.com/maps/{id}"))
                .call()
                .unwrap()
                .into_body()
                .read_to_string()
                .unwrap();

            update_progress(DownloadProgressState::FetchingInfo, 0.25);

            let parsed = search_parse(
                &map_info_response,
                [
                    // preview image url
                    (
                        "position:absolute;height:100%;width:100%;left:0;top:0;right:0;bottom:0;color:transparent\" src=\"",
                        "\"",
                    ),
                    // the zip link
                    (
                        "<a class=\"relative inline-flex items-center gap-2 rounded-l-md border border-blue-600 border-r-0 bg-blue-600 px-4 py-2 font-medium whitespace-nowrap text-white hover:bg-blue-700\" href=\"",
                        "\"",
                    ),
                ],
            );

            let Some([image_url, zip_url]) = parsed.first() else {
                eprintln!("Could not find urls");
                let mut currently_downloading = currently_downloading_handle.lock().unwrap();
                *currently_downloading = None;
                return;
            };

            // then we can actually download it
            let archive_bytes = download_with_progress::<65_535>(zip_url, |p| {
                update_progress(DownloadProgressState::DownloadingZip, p)
            });
            let image_bytes = download_with_progress::<4_096>(image_url, |p| {
                update_progress(DownloadProgressState::DownloadingImage, p)
            });

            let Some((map_info, _, _)) = ({
                let mut currently_downloading = currently_downloading_handle.lock().unwrap();
                currently_downloading.take()
            }) else {
                eprintln!("currently downloading state cleared while downloading");
                return;
            };

            command_sender
                .send(MapLoaderCommand::ImportBytes {
                    info: CustomMapInfo::new(
                        map_info.title,
                        Some(map_info.author),
                        Some(map_info.description),
                    ),
                    zip_archive_bytes: archive_bytes,
                    image_jpeg_bytes: image_bytes,
                })
                .unwrap()
        });
    }
}

impl egui::Widget for &mut MapDownloaderWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.strong("Map downloader");

            {
                let currently_downloading_guard = self.currently_downloading.lock().unwrap();
                if let Some(currently_downloading) = currently_downloading_guard.as_ref() {
                    currently_downloading
                        .0
                        .render(ui, Some((currently_downloading.1, currently_downloading.2)));
                    return;
                }
            }

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
                }
            });

            let mut map_to_download: Option<u16> = None;
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
                        if result.render(ui, None) {
                            map_to_download = Some(result.id);
                        }
                    }
                } else {
                    ui.add_space(8.0);
                    ui.spinner();
                    ui.small("Results can take up to 10 seconds to load");
                }
            }
            if let Some(map_to_download) = map_to_download {
                self.download(map_to_download);
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
                description: result[4].split('\n').next().unwrap_or_default().to_owned(),
                image_url: result[1].to_owned(),
            })
        })
        .collect()
}

fn download_with_progress<const CHUNK_SIZE: usize>(
    url: &str,
    mut on_progress: impl FnMut(f32),
) -> Vec<u8> {
    let response = ureq::get(url).call().unwrap();
    let content_length: Option<usize> = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok()?.parse::<usize>().ok());

    let Some(content_length) = content_length else {
        // cant stream, just download
        return response.into_body().read_to_vec().unwrap();
    };

    let n_chunks = (content_length / CHUNK_SIZE) as f32;

    let mut buffer = vec![0u8; content_length];
    let mut reader = response.into_body().into_reader();

    for (i, buf_chunk) in buffer.chunks_mut(CHUNK_SIZE).enumerate() {
        reader.read_exact(buf_chunk).unwrap();
        on_progress(i as f32 / n_chunks);
    }

    buffer
}
