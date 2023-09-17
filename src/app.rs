use eframe::egui;
use crate::{password, totp, vault};

struct App {
    copy_icon: egui_extras::RetainedImage,
    lock_icon: egui_extras::RetainedImage,
    password_modal_open: bool,
    password_text: String,
    secrets: Vec<vault::VaultSecret>,
}

pub fn build(secrets: Vec<vault::VaultSecret>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 480.0)),
        ..Default::default()
    };

    let app = Box::new(App::new(secrets));
    return eframe::run_native(
        "Stip",
        options,
        Box::new(|_| app),
    );
}

impl App {
    fn new(secrets: Vec<vault::VaultSecret>) -> App {
        let copy_icon = egui_extras::RetainedImage::from_svg_str(
            "Copy",
            include_str!("../assets/copy.svg"),
        ).unwrap();

        let lock_icon = egui_extras::RetainedImage::from_svg_str(
            "Lock",
            include_str!("../assets/key.svg"),
        ).unwrap();

        return Self {
            copy_icon,
            lock_icon,
            password_modal_open: false,
            password_text: String::new(),
            secrets,
        };
    }

    fn show_menu(ui: &mut egui::Ui) {
        use egui::{menu, Button};

        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    // …
                }
            });
        });
    }

    fn draw_grid_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        for row in &self.secrets {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button = egui::ImageButton::new(
                    self.copy_icon.texture_id(ctx),
                    egui::vec2(24.0, 24.0)
                );

                let request_copy_into_clipboard = ui.add(button).clicked();

                if let Some(secret) = row.secret.as_deref() {
                    let token = totp::from_now(secret, 6);
                    let token_text = format!("{:06}", token.number);
                    if request_copy_into_clipboard {
                        ui.output_mut(|o| o.copied_text = token_text.clone());
                    }
                    ui.label(&token_text);
                } else {
                    let button = egui::ImageButton::new(self.lock_icon.texture_id(ctx), egui::vec2(24.0, 24.0));
                    if ui.add(button).clicked() {
                        self.password_modal_open = true;
                    }
                }

                let filename = row.filename.clone();
                ui.add_sized(ui.available_size(), egui::Label::new(&filename));
            });

            ui.end_row();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut password_modal_open = self.password_modal_open;
        if password_modal_open {
            egui::Window::new("Password input")
                .open(&mut password_modal_open)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .title_bar(false)
                .resizable(false)
                .fixed_size(egui::vec2(240.0, 15.0))
                .show(ctx, |ui| {
                    let response = ui.add(password::password(&mut self.password_text));

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.password_modal_open = false;
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let mut size = ui.available_size();
                        size.x /= 2.0;

                        if ui.add_sized(size, egui::Button::new("Enter")).clicked() {
                        }

                        if ui.add_sized(ui.available_size(), egui::Button::new("Cancel")).clicked() {
                            self.password_text.clear();
                            self.password_modal_open = false;
                        }
                    });

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.password_text.clear();
                        self.password_modal_open = false;
                    }
                });
            self.password_modal_open &= password_modal_open;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(!self.password_modal_open, |ui| {
                Self::show_menu(ui);
                ui.separator();
                // We call `request_repaint` otherwise the progress bar glitch, presumably, because
                // it doesn't know it has to repaint every frame?
                ui.ctx().request_repaint();
                ui.add(egui::widgets::ProgressBar::new(totp::progress()));
                ui.separator();

                ui.set_visible(true);

                egui::Grid::new("my_grid")
                    .num_columns(1)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| self.draw_grid_content(ctx, ui));
            });
        });
    }
}
