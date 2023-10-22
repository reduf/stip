use eframe::egui;
use crate::{password, totp, vault};
use std::path::PathBuf;

struct App {
    copy_icon: egui_extras::RetainedImage,
    lock_icon: egui_extras::RetainedImage,
    plus_icon: egui_extras::RetainedImage,

    password_modal: Option<PasswordWindow>,
    secrets: Vec<vault::VaultSecret>,
    database: vault::Vault,
}

struct PasswordWindow {
    first_use: bool,
    password: String,
}

impl PasswordWindow {
    pub fn open() -> Self {
        return Self {
            first_use: true,
            password: String::new(),
        };
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<String> {
        let mut is_open = true;
        let mut close_after = true;
        egui::Window::new("Password input")
            .open(&mut is_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .title_bar(false)
            .resizable(false)
            .fixed_size(egui::vec2(240.0, 15.0))
            .show(ctx, |ui| {
                let response = ui.add(password::password(&mut self.password));

                if self.first_use {
                    ui.memory_mut(|mem| mem.request_focus(response.id));
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    close_after = false;
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let mut size = ui.available_size();
                    size.x /= 2.0;

                    if ui.add_sized(size, egui::Button::new("Enter")).clicked() {
                    }

                    if ui.add_sized(ui.available_size(), egui::Button::new("Cancel")).clicked() {
                        self.password.clear();
                        close_after = false;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.password.clear();
                    close_after = false;
                }

                self.first_use = false;
            });

        if !(close_after && is_open) {
            return Some(self.password.clone());
        } else {
            return None;
        }
    }
}

pub fn build(input: &str) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 480.0)),
        ..Default::default()
    };

    let app = Box::new(App::new(input));
    return eframe::run_native(
        "Stip",
        options,
        Box::new(|_| app),
    );
}

impl App {
    fn new(input: &str) -> App {
        let copy_icon = egui_extras::RetainedImage::from_svg_str(
            "Copy",
            include_str!("../assets/copy.svg"),
        ).unwrap();

        let lock_icon = egui_extras::RetainedImage::from_svg_str(
            "Lock",
            include_str!("../assets/key.svg"),
        ).unwrap();

        let plus_icon = egui_extras::RetainedImage::from_svg_str(
            "Plus",
            include_str!("../assets/plus.svg"),
        ).unwrap();

        let database = vault::Vault::open(PathBuf::from(input)).unwrap();

        return Self {
            copy_icon,
            lock_icon,
            plus_icon,
            password_modal: None,
            secrets: Vec::new(),
            database,
        };
    }

    fn show_menu(&self, ctx: &egui::Context, ui: &mut egui::Ui) {
        use egui::menu;

        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    // …
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let button = egui::Button::image_and_text(
                    self.plus_icon.texture_id(ctx),
                    egui::vec2(24.0, 24.0),
                    "Add",
                );

                if ui.add(button).clicked() {
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
                        // self.password_modal_open = true;
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
        if self.secrets.is_empty() && self.password_modal.is_none() {
            if self.database.requires_password() {
                self.password_modal = Some(PasswordWindow::open());
            } else {
                self.secrets = self.database.list(None).unwrap();
            }
        }

        if let Some(mut window) = self.password_modal.take() {
            if let Some(password) = window.show(ctx) {
                if let Ok(authenticodes) = self.database.list(Some(password.as_ref())) {
                    self.secrets = authenticodes;
                } else {
                    self.password_modal = Some(window);
                }
            } else {
                self.password_modal = Some(window);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(self.password_modal.is_none(), |ui| {
                self.show_menu(ctx, ui);
                ui.separator();

                // We call `request_repaint` otherwise the progress bar glitch, presumably, because
                // it doesn't know it has to repaint every frame?
                ui.ctx().request_repaint();
                ui.add(egui::widgets::ProgressBar::new(totp::progress()));
                ui.separator();

                ui.set_visible(true);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("my_grid")
                        .num_columns(1)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| self.draw_grid_content(ctx, ui));
                });
            });
        });
    }
}
