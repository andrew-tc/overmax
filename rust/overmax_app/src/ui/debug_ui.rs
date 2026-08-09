//! Debug log ring buffer and deferred viewport content.

use eframe::egui::{
    self, Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, ViewportClass,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::ui::overlay_theme::{apply_secondary_window_style, Theme};
use overmax_engine::detector::RateTelemetry;

#[derive(Clone, Debug, Default)]
pub struct DebugAppStateSnapshot {
    pub scene_label: String,
    pub confidence: f32,
    pub game_found: bool,
    pub is_active: bool,
    pub overlay_on: bool,
    pub always_visible: bool,
    pub opacity: f32,
    pub capture_engine: String,
    pub content_protected: bool,
    pub cached_hwnd: Option<isize>,
    pub game_hwnd: Option<isize>,
    pub song_info: String,
    pub play_state_info: String,
    pub jacket_match_info: String,
    pub capture_res_info: String,
}

pub fn push_log(lines: &Arc<Mutex<VecDeque<Arc<str>>>>, max_lines: usize, line: impl AsRef<str>) {
    let Ok(mut g) = lines.lock() else {
        return;
    };
    if g.len() >= max_lines {
        g.pop_front();
    }
    g.push_back(Arc::from(line.as_ref()));
}

pub fn render_debug(
    ctx: &egui::Context,
    class: ViewportClass,
    title: &str,
    lines: &Arc<Mutex<VecDeque<Arc<str>>>>,
    paused: &Arc<AtomicBool>,
    filters: &Arc<Mutex<std::collections::HashMap<String, bool>>>,
    rate_ocr: &Arc<Mutex<Option<RateTelemetry>>>,
    rate_ocr_texture: &Arc<Mutex<Option<egui::TextureHandle>>>,
    app_state: &DebugAppStateSnapshot,
) {
    apply_secondary_window_style(ctx);

    if class == ViewportClass::Embedded {
        egui::Window::new(title).show(ctx, |ui| {
            render_app_state_dashboard(ui, app_state);
            ui.add_space(8.0);
            render_ocr_telemetry(ui, rate_ocr, rate_ocr_texture);
            render_controls(ui, lines, paused, filters);
            ui.add_space(8.0);
            log_scroll(ui, lines, filters);
        });
    } else {
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(Theme::PANEL_BG)
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Debug")
                            .color(Theme::TEXT_ACCENT)
                            .size(Theme::FONT_HEADING)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Logs")
                            .color(Theme::TEXT_PRIMARY)
                            .size(Theme::FONT_HEADING)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let total_lines = if let Ok(g) = lines.lock() { g.len() } else { 0 };
                        ui.label(
                            RichText::new(format!("{} lines", total_lines))
                                .color(Theme::TEXT_MUTED)
                                .size(Theme::FONT_TINY),
                        );
                    });
                });
                ui.add_space(12.0);

                render_app_state_dashboard(ui, app_state);
                ui.add_space(12.0);

                render_ocr_telemetry(ui, rate_ocr, rate_ocr_texture);
                render_controls(ui, lines, paused, filters);
                ui.add_space(16.0);

                log_scroll(ui, lines, filters);
            });
    }
}

fn render_app_state_dashboard(ui: &mut egui::Ui, state: &DebugAppStateSnapshot) {
    Frame::new()
        .fill(Theme::CARD)
        .stroke(Stroke::new(1.0_f32, Theme::STROKE))
        .corner_radius(CornerRadius::same(Theme::R_MD))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("📊 Real-time App State Monitor")
                            .color(Theme::TEXT_PRIMARY)
                            .size(Theme::FONT_BODY)
                            .strong(),
                    );
                });
                ui.add_space(8.0);

                ui.columns(3, |cols| {
                    // Col 1: Scene & Confidence
                    cols[0].vertical(|ui| {
                        ui.label(
                            RichText::new("Scene / Conf")
                                .size(Theme::FONT_TINY)
                                .color(Theme::TEXT_MUTED),
                        );
                        let scene_color = if state.scene_label.contains("Unknown") {
                            Color32::from_rgb(255, 170, 0)
                        } else {
                            Color32::from_rgb(100, 200, 255)
                        };
                        ui.label(
                            RichText::new(format!(
                                "{} ({:.2})",
                                state.scene_label, state.confidence
                            ))
                            .size(Theme::FONT_SMALL)
                            .color(scene_color)
                            .strong(),
                        );
                    });

                    // Col 2: Game & Focus (Topmost)
                    cols[1].vertical(|ui| {
                        ui.label(
                            RichText::new("Game & Focus")
                                .size(Theme::FONT_TINY)
                                .color(Theme::TEXT_MUTED),
                        );
                        #[cfg(target_os = "windows")]
                        let (focus_txt, focus_color) = if state.is_active {
                            ("Active (Topmost)", Color32::from_rgb(100, 255, 100))
                        } else {
                            ("Inactive (Notopmost)", Color32::from_rgb(255, 170, 0))
                        };
                        #[cfg(not(target_os = "windows"))]
                        let (focus_txt, focus_color) = if state.is_active {
                            ("Active", Color32::from_rgb(100, 255, 100))
                        } else {
                            ("Inactive", Color32::from_rgb(255, 170, 0))
                        };

                        let game_txt = if state.game_found {
                            "Found"
                        } else {
                            "Not Found"
                        };
                        ui.label(
                            RichText::new(format!("{} | {}", game_txt, focus_txt))
                                .size(Theme::FONT_SMALL)
                                .color(focus_color)
                                .strong(),
                        );
                    });

                    // Col 3: Overlay Visibility & Capture Engine
                    cols[2].vertical(|ui| {
                        ui.label(
                            RichText::new("Overlay & Engine")
                                .size(Theme::FONT_TINY)
                                .color(Theme::TEXT_MUTED),
                        );
                        let (vis_txt, vis_color) = if state.overlay_on {
                            (
                                format!("Visible ({:.0}%)", state.opacity * 100.0),
                                Color32::from_rgb(100, 255, 100),
                            )
                        } else {
                            ("Hidden (0%)".to_string(), Color32::from_rgb(255, 100, 100))
                        };
                        #[cfg(target_os = "windows")]
                        let engine_str = state.capture_engine.to_uppercase();
                        #[cfg(not(target_os = "windows"))]
                        let engine_str = "XCOMPOSITE".to_string();

                        ui.label(
                            RichText::new(format!("{} | {}", vis_txt, engine_str))
                                .size(Theme::FONT_SMALL)
                                .color(vis_color)
                                .strong(),
                        );
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.columns(3, |cols| {
                    // Row 2 - Col 1: Song & Jacket Status
                    cols[0].vertical(|ui| {
                        ui.label(
                            RichText::new("Detected Song & Jacket Match")
                                .size(Theme::FONT_TINY)
                                .color(Theme::TEXT_MUTED),
                        );
                        let song_txt = if state.song_info.is_empty() {
                            "None".to_string()
                        } else {
                            state.song_info.clone()
                        };
                        let match_txt = if state.jacket_match_info.is_empty() {
                            "-".to_string()
                        } else {
                            state.jacket_match_info.clone()
                        };
                        ui.label(
                            RichText::new(format!("{} [{}]", song_txt, match_txt))
                                .size(Theme::FONT_SMALL)
                                .color(Color32::from_rgb(255, 220, 100))
                                .strong(),
                        );
                    });

                    // Row 2 - Col 2: PlayState & Stability
                    cols[1].vertical(|ui| {
                        ui.label(
                            RichText::new("PlayState / Mode / Stability")
                                .size(Theme::FONT_TINY)
                                .color(Theme::TEXT_MUTED),
                        );
                        let ps_txt = if state.play_state_info.is_empty() {
                            "None".to_string()
                        } else {
                            state.play_state_info.clone()
                        };
                        ui.label(
                            RichText::new(ps_txt)
                                .size(Theme::FONT_SMALL)
                                .color(Color32::from_rgb(180, 220, 255))
                                .strong(),
                        );
                    });

                    // Row 2 - Col 3: Capture Resolution & Geometry
                    cols[2].vertical(|ui| {
                        ui.label(
                            RichText::new("Captured Resolution & Game Rect")
                                .size(Theme::FONT_TINY)
                                .color(Theme::TEXT_MUTED),
                        );
                        let res_txt = if state.capture_res_info.is_empty() {
                            "Unknown".to_string()
                        } else {
                            state.capture_res_info.clone()
                        };
                        ui.label(
                            RichText::new(res_txt)
                                .size(Theme::FONT_SMALL)
                                .color(Theme::TEXT_PRIMARY)
                                .strong(),
                        );
                    });
                });
            });
        });
}

fn update_ocr_texture(
    ctx: &egui::Context,
    info: &RateTelemetry,
    texture_guard: &mut Option<egui::TextureHandle>,
) {
    let should_update = match texture_guard.as_ref() {
        None => true,
        Some(handle) => {
            handle.size()[0] != info.image_width || handle.size()[1] != info.image_height
        }
    };
    let texture_name = format!(
        "ocr_rate_{}_{}_{}",
        info.rate_text, info.threshold, info.use_invert
    );
    let should_update = should_update
        || match texture_guard.as_ref() {
            None => true,
            Some(handle) => handle.name() != texture_name,
        };

    if should_update {
        let pixels = if info.image_pixels.len() == info.image_width * info.image_height * 4 {
            info.image_pixels
                .chunks_exact(4)
                .map(|chunk| {
                    let color = overmax_cv::Bgr::from_bgra_slice(chunk);
                    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, chunk[3])
                })
                .collect()
        } else {
            info.image_pixels
                .iter()
                .map(|&p| egui::Color32::from_gray(p))
                .collect()
        };
        let color_image = egui::ColorImage {
            size: [info.image_width, info.image_height],
            pixels,
            source_size: egui::vec2(info.image_width as f32, info.image_height as f32),
        };
        *texture_guard =
            Some(ctx.load_texture(texture_name, color_image, egui::TextureOptions::default()));
    }
}

fn render_ocr_telemetry(
    ui: &mut egui::Ui,
    rate_ocr: &Arc<Mutex<Option<RateTelemetry>>>,
    rate_ocr_texture: &Arc<Mutex<Option<egui::TextureHandle>>>,
) {
    let ocr_info = if let Ok(g) = rate_ocr.lock() {
        g.clone()
    } else {
        None
    };
    let Some(info) = ocr_info else {
        return;
    };
    if info.image_width == 0 || info.image_height == 0 || info.image_pixels.is_empty() {
        return;
    }

    let mut texture_guard = overmax_core::sync::lock_or_recover(rate_ocr_texture);
    update_ocr_texture(ui.ctx(), &info, &mut texture_guard);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Rate OCR Status:")
                    .strong()
                    .color(Theme::TEXT_ACCENT),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Text: \"{}\"", info.rate_text)).color(Theme::TEXT_PRIMARY),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("Threshold: {}", info.threshold)).color(Theme::TEXT_PRIMARY),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("BgMean: {:.1}", info.bg_mean)).color(Theme::TEXT_PRIMARY),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("Inverted: {}", info.use_invert)).color(Theme::TEXT_PRIMARY),
            );
        });

        ui.add_space(6.0);

        if let Some(texture) = &*texture_guard {
            let max_width = 300.0;
            let ratio = texture.size()[1] as f32 / texture.size()[0] as f32;
            let display_width = (texture.size()[0] as f32).min(max_width);
            let display_size = egui::vec2(display_width, display_width * ratio);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("OCR Image:")
                        .size(Theme::FONT_TINY)
                        .color(Theme::TEXT_MUTED),
                );
                ui.add_space(4.0);
                ui.image((texture.id(), display_size));
            });
        }
    });
    ui.add_space(8.0);
}

fn render_controls(
    ui: &mut egui::Ui,
    lines: &Arc<Mutex<VecDeque<Arc<str>>>>,
    paused: &Arc<AtomicBool>,
    filters: &Arc<Mutex<std::collections::HashMap<String, bool>>>,
) {
    ui.horizontal(|ui| {
        // Pause Button
        let is_paused = paused.load(Ordering::Relaxed);
        let pause_text = if is_paused {
            "▶ 재개"
        } else {
            "⏸ 일시정지"
        };
        let pause_btn =
            egui::Button::new(RichText::new(pause_text).size(Theme::FONT_SMALL).strong())
                .min_size(egui::vec2(80.0, Theme::CONTROL_HEIGHT))
                .fill(if is_paused {
                    Theme::BTN_PAUSED
                } else {
                    Theme::SECONDARY
                })
                .corner_radius(egui::CornerRadius::same(Theme::R_SM));
        if ui.add(pause_btn).clicked() {
            paused.store(!is_paused, Ordering::Relaxed);
        }

        // Clear Button
        let clear_btn = egui::Button::new(RichText::new("🗑 지우기").size(Theme::FONT_SMALL))
            .min_size(egui::vec2(80.0, Theme::CONTROL_HEIGHT))
            .fill(Theme::CARD)
            .stroke(Stroke::new(1.0_f32, Theme::STROKE))
            .corner_radius(egui::CornerRadius::same(Theme::R_SM));
        if ui.add(clear_btn).clicked() {
            if let Ok(mut g) = lines.lock() {
                g.clear();
            }
        }
    });

    ui.add_space(8.0);

    // Filters Row
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("필터:")
                .color(Theme::TEXT_SECONDARY)
                .size(Theme::FONT_SMALL)
                .strong(),
        );
        ui.add_space(4.0);

        if let Ok(mut filters_lock) = filters.lock() {
            let tags = [
                "[ScreenCapture]",
                "[Overlay]",
                "[VArchive]",
                "[WindowTracker]",
                "[Main]",
            ];
            for tag in &tags {
                let tag_name = tag.trim_matches(|c| c == '[' || c == ']');
                if let Some(val) = filters_lock.get_mut(*tag) {
                    let color = get_line_color(tag);
                    let mut checked = *val;
                    let cb = egui::Checkbox::new(
                        &mut checked,
                        RichText::new(tag_name).color(color).size(Theme::FONT_SMALL),
                    );
                    if ui.add(cb).changed() {
                        *val = checked;
                    }
                }
            }
        }
    });
}

fn log_scroll(
    ui: &mut egui::Ui,
    lines: &Arc<Mutex<VecDeque<Arc<str>>>>,
    filters: &Arc<Mutex<std::collections::HashMap<String, bool>>>,
) {
    // 락 보유 시간을 최소화하기 위해 Arc<str> 참조만 극속 채취 후 즉시 락 해제
    let snapshot: Vec<Arc<str>> = {
        let Ok(lines_guard) = lines.lock() else {
            return;
        };
        lines_guard.iter().cloned().collect()
    };

    let filters_lock = overmax_core::sync::lock_or_recover(filters);

    let tags = [
        "[ScreenCapture]",
        "[Overlay]",
        "[VArchive]",
        "[WindowTracker]",
        "[Main]",
        "[UI]",
    ];

    let filtered_lines: Vec<&Arc<str>> = snapshot
        .iter()
        .filter(|line| {
            for tag in &tags {
                if line.contains(tag) {
                    let lookup_tag = if *tag == "[UI]" { "[Main]" } else { *tag };
                    return *filters_lock.get(lookup_tag).unwrap_or(&true);
                }
            }
            true
        })
        .collect();

    Frame::new()
        .fill(Theme::CARD)
        .stroke(Stroke::new(1.0_f32, Theme::STROKE))
        .corner_radius(CornerRadius::same(Theme::R_MD))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
            ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show_rows(ui, row_height, filtered_lines.len(), |ui, range| {
                    for idx in range {
                        let line = filtered_lines[idx];
                        let color = get_line_color(line);
                        ui.label(
                            RichText::new(line.as_ref())
                                .color(color)
                                .monospace()
                                .size(Theme::FONT_TINY),
                        );
                    }
                });
        });
}

pub fn close_if_requested(ctx: &egui::Context, open: &Arc<AtomicBool>) {
    if ctx.input(|i| i.viewport().close_requested()) {
        open.store(false, Ordering::Relaxed);
        ctx.request_repaint_of(ctx.parent_viewport_id());
    }
}

fn get_line_color(line: &str) -> Color32 {
    if line.contains("[ScreenCapture]") {
        Theme::LOG_CAPTURE
    } else if line.contains("[Overlay]") {
        Theme::LOG_OVERLAY
    } else if line.contains("[VArchive]") {
        Theme::LOG_VARCHIVE
    } else if line.contains("[WindowTracker]") {
        Theme::LOG_WINDOW
    } else if line.contains("[Main]") || line.contains("[UI]") {
        Theme::LOG_MAIN
    } else {
        Theme::LOG_DEFAULT
    }
}
