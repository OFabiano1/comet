//! Janela do Comet: abre no duplo clique. Mesmo estado e mesma fila do terminal.
use crate::app::{self, App, Item, Kind, Phase};
use crate::{downloader, sites, ui};
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, Stroke, Vec2};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const VOID: Color32 = Color32::from_rgb(0x09, 0x09, 0x0D);
const SURFACE: Color32 = Color32::from_rgb(0x10, 0x10, 0x15);
const GRAPHITE: Color32 = Color32::from_rgb(0x15, 0x15, 0x1C);
const BORDER: Color32 = Color32::from_rgb(0x29, 0x29, 0x33);
const PRIMARY: Color32 = Color32::from_rgb(0x8F, 0x00, 0xFF);
const ELECTRIC: Color32 = Color32::from_rgb(0xA3, 0x4A, 0xFF);
const BRAND: Color32 = Color32::from_rgb(0xC0, 0x85, 0xFF);
const TEXT: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF4);
const MUTED: Color32 = Color32::from_rgb(0x90, 0x90, 0x9D);
const SUCCESS: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
const DANGER: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");
const ICON: &[u8] = include_bytes!("../assets/comet.png");

struct Gui {
    app: Arc<Mutex<App>>,
    rt: tokio::runtime::Handle,
    /// Formato que o próximo link colado recebe.
    kind: Kind,
    /// Erro do último link (não reconhecido, repetido).
    error: Option<String>,
}

pub fn run(rt: tokio::runtime::Runtime) -> anyhow::Result<()> {
    let app = Arc::new(Mutex::new(App {
        history: downloader::load_history(),
        // Janela não tem "pasta do terminal": vai sempre pra Downloads\Comet.
        out_dir: downloader::default_downloads_dir(),
        ..App::default()
    }));
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Comet")
        .with_inner_size([560.0, 640.0])
        .with_min_inner_size([420.0, 460.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let handle = rt.handle().clone();
    eframe::run_native(
        "Comet",
        options,
        Box::new(move |cc| {
            setup_style(&cc.egui_ctx);
            Ok(Box::new(Gui {
                app,
                rt: handle,
                kind: Kind::Video,
                error: None,
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("não deu pra abrir a janela: {e}"))
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "jbm".into(),
        Arc::new(egui::FontData::from_static(FONT_REGULAR)),
    );
    fonts.font_data.insert(
        "jbm-medium".into(),
        Arc::new(egui::FontData::from_static(FONT_MEDIUM)),
    );
    // JetBrains Mono na frente; as fontes padrão do egui ficam de reserva pra ☄ e ✓.
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().insert(0, "jbm".into());
    }
    // ☄ não existe na JetBrains Mono nem nas fontes do egui; a Segoe UI Symbol vem no Windows.
    if let Ok(sym) = std::fs::read(r"C:\Windows\Fonts\seguisym.ttf") {
        fonts.font_data.insert("sym".into(), Arc::new(egui::FontData::from_owned(sym)));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push("sym".into());
        }
    }
    let mut medium = vec!["jbm-medium".to_string()];
    medium.extend(fonts.families[&egui::FontFamily::Proportional].iter().skip(1).cloned());
    fonts
        .families
        .insert(egui::FontFamily::Name("medium".into()), medium);
    ctx.set_fonts(fonts);

    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = VOID;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE;
    v.faint_bg_color = SURFACE;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = BRAND;
    v.selection.bg_fill = PRIMARY;
    v.selection.stroke = Stroke::new(2.0, ELECTRIC);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(6);
        w.bg_fill = GRAPHITE;
        w.weak_bg_fill = GRAPHITE;
    }
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ELECTRIC);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(14.0, 8.0);
        s.spacing.interact_size.y = 36.0;
    });
}

fn overline(text: &str) -> RichText {
    RichText::new(text.to_uppercase())
        .size(11.0)
        .color(MUTED)
        .extra_letter_spacing(1.3)
}

fn medium(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("medium".into()))
}

impl Gui {
    /// Põe o link na fila com o formato escolhido e já começa a baixar.
    fn submit(&mut self, a: &mut App) {
        match a.add_url() {
            Ok(()) => {
                self.error = None;
                if let Some(item) = a.items.last_mut() {
                    item.kind = self.kind;
                }
                self.start(a);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn start(&self, a: &mut App) {
        if a.busy {
            return; // a fila que já roda pega os itens novos
        }
        a.busy = true;
        a.status = app::STATUS_BUSY.to_string();
        self.rt.spawn(crate::run_queue(self.app.clone()));
    }

    fn link_row(&mut self, ui: &mut egui::Ui, a: &mut App) {
        let link_label = ui.label(overline("link"));
        let pasted = ui
            .input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));
        let mut go = false;
        ui.horizontal(|ui| {
            let btn_w = 96.0;
            let field = ui.add_sized(
                [ui.available_width() - btn_w - 8.0, 40.0],
                egui::TextEdit::singleline(&mut a.input)
                    .hint_text(RichText::new("cola o link aqui (ctrl+v)").color(MUTED))
                    .font(FontId::proportional(14.0))
                    .margin(Vec2::new(12.0, 10.0)),
            )
            .labelled_by(link_label.id);
            if ui.memory(|m| m.focused().is_none()) {
                field.request_focus(); // ctrl+v funciona sem clicar no campo
            }
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            // Colou um link que o Comet conhece: baixa direto, sem precisar de Enter.
            let pasted_link = pasted && field.changed() && sites::detect(a.input.trim()).is_some();
            let button = egui::Button::new(
                RichText::new("baixar").font(medium(14.0)).color(Color32::WHITE),
            )
            .fill(PRIMARY)
            .stroke(Stroke::NONE);
            let clicked = ui.add_sized([btn_w, 40.0], button).clicked();
            go = enter || pasted_link || clicked;
        });
        if go && !a.input.trim().is_empty() {
            self.submit(a);
        }
        ui.horizontal(|ui| {
            for (kind, label, hint) in [
                (Kind::Video, "vídeo", "mp4"),
                (Kind::Gif, "gif", "gif de até 30s"),
                (Kind::Audio, "mp3", "só o áudio"),
            ] {
                let on = self.kind == kind;
                let text = RichText::new(label)
                    .size(13.0)
                    .color(if on { BRAND } else { MUTED });
                let b = egui::Button::new(text)
                    .fill(if on { GRAPHITE } else { VOID })
                    .stroke(Stroke::new(1.0, if on { PRIMARY } else { BORDER }))
                    .min_size(Vec2::new(64.0, 30.0));
                if ui.add(b).on_hover_text(hint).clicked() {
                    self.kind = kind;
                }
            }
            ui.label(RichText::new("youtube · twitter/x · tiktok").size(12.0).color(MUTED));
        });
        if let Some(e) = &self.error {
            ui.label(RichText::new(e).size(13.0).color(DANGER));
        }
    }

    fn queue(&mut self, ui: &mut egui::Ui, a: &mut App) {
        ui.horizontal(|ui| {
            ui.label(overline("fila"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dir = ui::short_dir(&a.out_dir).unwrap_or_default();
                if ui
                    .link(RichText::new(format!("abrir {dir}")).size(12.0).color(BRAND))
                    .clicked()
                {
                    crate::open_in_explorer(None, &a.out_dir);
                }
            });
        });
        if a.items.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Cola um link do YouTube, Twitter/X ou TikTok e ele começa a baixar na hora.")
                    .size(13.0)
                    .color(MUTED),
            );
            return;
        }
        let mut retry = None;
        for (idx, item) in a.items.iter().enumerate().rev() {
            item_card(ui, item, idx, &mut retry);
        }
        if let Some(idx) = retry {
            let item = &mut a.items[idx];
            item.phase = Phase::Queued;
            item.error = None;
            item.progress = Default::default();
            self.start(a);
        }
    }

    fn history(&mut self, ui: &mut egui::Ui, a: &mut App) {
        if a.history.is_empty() {
            return;
        }
        let mut again = None;
        egui::CollapsingHeader::new(overline(&format!("histórico · {}", a.history.len())))
            .id_salt("hist")
            .show(ui, |ui| {
                for h in a.history.iter().take(30) {
                    ui.horizontal(|ui| {
                        if ui.small_button("baixar de novo").clicked() {
                            again = Some(h.url.clone());
                        }
                        ui.label(RichText::new(&h.label).size(12.0).color(MUTED))
                            .on_hover_text(&h.url);
                    });
                }
            });
        if let Some(url) = again {
            a.input = url;
            self.submit(a);
        }
    }
}

fn item_card(ui: &mut egui::Ui, item: &Item, idx: usize, retry: &mut Option<usize>) {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(&item.title).font(medium(14.0))).truncate());
            ui.label(RichText::new(ui::meta_text(item)).size(12.0).color(MUTED));
            match item.phase {
                Phase::Queued => {
                    ui.label(RichText::new("na fila").size(12.0).color(MUTED));
                }
                Phase::Downloading | Phase::Converting => {
                    let p = &item.progress;
                    let pct = p.pct().unwrap_or(0.0) as f32 / 100.0;
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, BORDER);
                    let mut fill = rect;
                    fill.set_width(rect.width() * pct);
                    ui.painter().rect_filled(fill, 3.0, ELECTRIC);
                    let text = if item.phase == Phase::Converting {
                        "convertendo pra gif".to_string()
                    } else {
                        let mut t = ui::remaining_text(p);
                        if let Some(s) = p.speed {
                            t.push_str(&format!(" · {}/s", ui::human_bytes(s as u64)));
                        }
                        t
                    };
                    ui.label(RichText::new(text).size(12.0).color(BRAND));
                }
                Phase::Done => {
                    ui.horizontal(|ui| {
                        match &item.error {
                            // Concluído com aviso (ex.: gif falhou, mp4 ficou).
                            Some(note) => ui.label(RichText::new(format!("! {note}")).size(12.0).color(MUTED)),
                            None => ui.label(RichText::new("✓ pronto").size(12.0).color(SUCCESS)),
                        };
                        if let Some(file) = item.saved.as_deref().filter(|f| f.is_file()) {
                            if ui.small_button("mostrar na pasta").clicked() {
                                crate::open_in_explorer(Some(file), file.parent().unwrap_or(file));
                            }
                        }
                    });
                }
                Phase::Failed => {
                    ui.horizontal_wrapped(|ui| {
                        let err = item.error.as_deref().unwrap_or("o download falhou");
                        ui.label(RichText::new(format!("✗ {err}")).size(12.0).color(DANGER));
                        if ui.small_button("tentar de novo").clicked() {
                            *retry = Some(idx);
                        }
                    });
                }
            }
        });
    ui.add_space(4.0);
}

impl eframe::App for Gui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let app = self.app.clone();
        let mut a = app.blocking_lock();
        if a.busy {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(VOID).inner_margin(egui::Margin::same(24)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("☄").size(22.0).color(PRIMARY));
                    ui.label(RichText::new("comet").font(medium(20.0)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(&a.status).size(12.0).color(MUTED));
                    });
                });
                ui.add_space(16.0);
                self.link_row(ui, &mut a);
                ui.add_space(20.0);
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    self.queue(ui, &mut a);
                    ui.add_space(12.0);
                    self.history(ui, &mut a);
                });
            });
    }
}
