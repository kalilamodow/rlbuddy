use eframe::egui;
use emath::Rect;

pub struct MapCardWidget<'a, ButtonsWidget> {
    title: &'a str,
    author: Option<&'a str>,
    description: Option<&'a str>,
    image_url: Option<&'a str>,
    buttons: ButtonsWidget,
    highlight: bool,
}

impl<'a, ButtonsWidget> MapCardWidget<'a, ButtonsWidget>
where
    ButtonsWidget: Fn(&mut egui::Ui),
{
    pub fn new(
        title: &'a str,
        author: Option<&'a str>,
        description: Option<&'a str>,
        image_url: Option<&'a str>,
        buttons: ButtonsWidget,
        highlight: bool,
    ) -> Self {
        Self {
            title,
            author,
            description,
            image_url,
            buttons,
            highlight,
        }
    }

    fn render_map_card_content(&self, ui: &mut egui::Ui, bg_rect: Rect) {
        // shrink for margin
        let content_rect = bg_rect.shrink(8.0);

        ui.place(content_rect, |ui: &mut egui::Ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(self.title).strong().size(15.0));
                ui.add_space(2.0);

                if let Some(description) = self.description {
                    ui.label(description);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        if let Some(author) = self.author {
                            ui.label(format!("By {}", author));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                            (self.buttons)(ui);
                        });
                    });
                });
            })
            .response
        });
    }
}

impl<'a, ButtonsWidget> egui::Widget for MapCardWidget<'a, ButtonsWidget>
where
    ButtonsWidget: Fn(&mut egui::Ui),
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(egui::vec2(200.0, 75.0), |ui| {
            // first, draw the background
            let image_rect = match self.image_url {
                Some(image_url) => {
                    ui.add(
                        egui::Image::new(egui::ImageSource::Uri(image_url.into()))
                            .fit_to_exact_size(egui::vec2(210.0, 120.0))
                            .maintain_aspect_ratio(false)
                            .corner_radius(egui::CornerRadius::same(8)),
                    )
                    .rect
                }
                None => ui.allocate_space(egui::vec2(210.0, 120.0)).1,
            };

            // then, add a dark overlay for contrast
            ui.painter().add(
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(200))
                    .corner_radius(egui::CornerRadius::same(8))
                    .stroke(if self.highlight {
                        egui::Stroke::new(0.5f32, egui::Color32::WHITE)
                    } else {
                        Default::default()
                    })
                    .paint(image_rect),
            );

            if image_rect.width() < 50.0 || image_rect.height() < 50.0 {
                // probably loading, dont render content
                return;
            }

            // finally, put the actual content
            self.render_map_card_content(ui, image_rect);
        })
        .response
    }
}
