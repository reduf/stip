use eframe::egui;
use crate::{password, stb_image, totp, vault};
use std::path::PathBuf;
use rfd::FileDialog;

struct Row {
    secret: vault::VaultSecret,
    editing: bool,
}

impl Row {
    fn new(secret: vault::VaultSecret) -> Self {
        return Row {
            secret,
            editing: false,
        };
    }
}

enum Db {
    None,
    Path(PathBuf),
    Open(vault::Vault),
}

impl Db {
    pub fn take(&mut self) -> Self {
        return std::mem::replace(self, Db::None);
    }
}

struct App {
    password_modal: Option<PasswordWindow>,
    dropped_files: Vec<egui::DroppedFile>,

    database: Db,
    rows: Vec<Row>,
    icon_textures: Vec<egui::TextureHandle>,
}

struct PasswordWindow {
    select: bool,
    failure: bool,
    password: String,
}

impl PasswordWindow {
    pub fn open() -> Self {
        return Self {
            select: true,
            failure: false,
            password: String::new(),
        };
    }

    pub fn failed(mut self) -> Self {
        self.select = true;
        self.failure = true;
        return self;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<String> {
        let mut is_open = true;
        let mut close_after = false;
        egui::Window::new("Password input")
            .open(&mut is_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .title_bar(false)
            .resizable(false)
            .fixed_size(egui::vec2(240.0, 15.0))
            .show(ctx, |ui| {
                let response = ui.add(password::password(&mut self.password, self.failure));

                if self.select {
                    ui.memory_mut(|mem| mem.request_focus(response.id));
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    close_after = true;
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let mut size = ui.available_size();
                    size.x /= 2.0;

                    if ui.add_sized(size, egui::Button::new("Enter")).clicked() {
                        close_after = true;
                    }

                    if ui.add_sized(ui.available_size(), egui::Button::new("Cancel")).clicked() {
                        self.password.clear();
                        close_after = true;
                    }
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.password.clear();
                    close_after = true;
                }

                self.select = false;
            });

        if close_after || !is_open {
            return Some(self.password.clone());
        } else {
            return None;
        }
    }
}

fn load_icon() -> Option<egui::IconData> {
    let buffer = include_bytes!("../assets/shield-96.png");
    if let Ok(img) = stb_image::load_from_memory(buffer.as_ref(), stb_image::Channel::Rgba) {
        let rgba_bytes = img.data().to_vec();
        return Some(egui::IconData {
            rgba: rgba_bytes,
            width: img.width as u32,
            height: img.height as u32,
        });
    } else {
        return None;
    }
}

pub fn build(path: Option<&str>, password: Option<String>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 480.0])
            .with_drag_and_drop(true)
            .with_icon(load_icon().unwrap()),
        ..Default::default()
    };

    let mut app = Box::new(App::new(path.map(PathBuf::from)));
    return eframe::run_native(
        "Stip",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            if let Some(password) = password {
                app.try_open_db(&cc.egui_ctx, password.as_str());
            }

            return app;
        }),
    );
}

impl App {
    fn new(path: Option<PathBuf>) -> App {
        let database = path.map(Db::Path).unwrap_or(Db::None);
        let mut app = Self {
            password_modal: None,
            dropped_files: Vec::new(),
            database,
            rows: Vec::new(),
            icon_textures: Vec::new(),
        };

        return app;
    }

    fn try_open_db(&mut self, ctx: &egui::Context, password: &str) -> bool {
        if let Db::Path(path) = self.database.take() {
            if let Ok(vault) = vault::Vault::open(path.clone(), password) {
                self.rows = vault.secrets().into_iter().map(Row::new).collect::<Vec<Row>>();
                for icon in vault.custom_icons.iter() {
                    Self::add_texture_from_image(&mut self.icon_textures, ctx, icon);
                }
                self.database = Db::Open(vault);
                return true;
            } else {
                self.database = Db::Path(path);
                return false;
            }
        } else {
            return false;
        }
    }

    fn pick_database() -> Option<PathBuf> {
        let file_dialog = FileDialog::new();
        return file_dialog.pick_file();
    }

    fn add_texture_from_image(
        icon_textures: &mut Vec<egui::TextureHandle>,
        ctx: &egui::Context,
        img: &stb_image::Image
    ) {
        icon_textures.push(ctx.load_texture(
            format!("icon:{}", icon_textures.len()),
            egui::ColorImage::from_rgba_unmultiplied(
                [img.width, img.height],
                img.data(),
            ),
            Default::default(),
        ));
    }

    fn show_menu(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        use egui::menu;

        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    if let Some(path) = Self::pick_database() {
                        self.rows.clear();
                        self.icon_textures.clear();
                        self.database = Db::Path(path);
                    }
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let img = egui::Image::new(egui::include_image!("../assets/plus.svg"));
                let button = egui::Button::image_and_text(img, "Add");

                if ui.add(button).clicked() {
                }
            });
        });
    }

    fn draw_grid_content(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        let first_column_size = [175.0, ui.available_height()];
        for row in self.rows.iter_mut() {
            if row.editing {
                let text_edit = egui::TextEdit::singleline(&mut row.secret.name)
                    .vertical_align(egui::Align::Center)
                    .horizontal_align(egui::Align::Center);

                if ui.add_sized(first_column_size, text_edit).lost_focus() {
                    row.editing = false;
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    }
                }
            } else {
                if ui.add_sized(first_column_size, egui::Label::new(&row.secret.name)).double_clicked() {
                    row.editing = true;
                }
            }

            if let Some(icon_idx) = row.secret.icon.clone() {
                let texture = &self.icon_textures[icon_idx];
                ui.image((texture.id(), egui::vec2(texture.aspect_ratio() * 20.0, 20.0)));
            } else {
                ui.label("");
            }

            let token = totp::from_now_with_period(
                row.secret.secret.as_ref(),
                row.secret.period,
                row.secret.digits,
            );

            let token_text = format!("{:0digits$}", token.number, digits = row.secret.digits);
            ui.label(&token_text);

            let img = egui::Image::new(egui::include_image!("../assets/copy.svg"));
            let button = egui::ImageButton::new(img);
            if ui.add_sized([24.0, 24.0], button).clicked() {
                ui.output_mut(|o| o.copied_text = token_text.clone());
            }
            ui.end_row();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.password_modal.is_none() {
            if let Db::Path(_) = self.database {
                self.password_modal = Some(PasswordWindow::open());
            }
        }

        if let Some(mut window) = self.password_modal.take() {
            if let Some(password) = window.show(ctx) {
                if !self.try_open_db(ctx, password.as_ref()) {
                    self.password_modal = Some(window.failed());
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
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| self.draw_grid_content(ctx, ui));
                });
            });
        });

        ctx.input(|input| {
            if !input.raw.dropped_files.is_empty() {
                self.dropped_files = input.raw.dropped_files.clone();
            }
        });

        for file in self.dropped_files.iter() {
            if let Some(path) = file.path.as_deref() {
                match vault::VaultSecret::from_path(path) {
                    Ok(secret) => self.rows.push(Row { secret, editing: false }),
                    Err(err) => eprintln!("Failed to load {:?} as secret, err: {:?}", path, err),
                }
            }
        }

        self.dropped_files.clear();
    }
}
