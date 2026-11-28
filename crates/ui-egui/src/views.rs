//! Drawing: the top bar, the calendar and the lists beside it, the page, the status bar.

use crate::app::{Action, DiaryCraftApp, Theme, today};
use crate::folders::suggested_diary as suggested_folder;
use crate::i18n::Lang;
use chrono::Datelike;
use diarycraft_engine::calendar::{add_months, month_grid, week_days};
use diarycraft_model::WeekStart;

pub fn top_bar(ui: &mut egui::Ui, app: &mut DiaryCraftApp) {
    let s = app.strings();
    ui.horizontal(|ui| {
        ui.menu_button(s.file, |ui| {
            if ui.button(s.open_diary).clicked() {
                app.actions.push(Action::OpenDialog);
                ui.close();
            }
            ui.menu_button(s.open_recent, |ui| {
                for p in app.prefs.recent.clone() {
                    if ui.button(p.display().to_string()).clicked() {
                        app.actions.push(Action::Open(p));
                        ui.close();
                    }
                }
            });
            if ui.add_enabled(app.diary.is_some(), egui::Button::new(s.close_diary)).clicked() {
                app.actions.push(Action::CloseDiary);
                ui.close();
            }
            ui.separator();
            ui.menu_button(s.language, |ui| {
                for (lang, name) in [(Lang::It, "Italiano"), (Lang::En, "English")] {
                    if ui.button(name).clicked() {
                        app.actions.push(Action::SetLang(lang));
                        ui.close();
                    }
                }
            });
            ui.menu_button(s.theme, |ui| {
                for (theme, name) in
                    [(Theme::System, s.theme_system), (Theme::Light, s.theme_light), (Theme::Dark, s.theme_dark)]
                {
                    if ui.radio(app.prefs.theme == theme, name).clicked() {
                        app.actions.push(Action::SetTheme(theme));
                        ui.close();
                    }
                }
            });
            let week_start = app.diary.as_ref().map(|d| d.config().week_start());
            ui.add_enabled_ui(week_start.is_some(), |ui| {
                ui.menu_button(s.week_start, |ui| {
                    for (start, name) in [(WeekStart::Monday, s.monday), (WeekStart::Sunday, s.sunday)] {
                        if ui.radio(week_start == Some(start), name).on_hover_text(s.week_start_hint).clicked() {
                            app.actions.push(Action::SetWeekStart(start));
                            ui.close();
                        }
                    }
                });
            });
            ui.separator();
            if ui.button(s.quit).clicked() {
                app.actions.push(Action::Quit);
                ui.close();
            }
        });
        if app.diary.is_none() {
            return;
        }
        ui.separator();
        if ui.button(s.today).clicked() {
            app.actions.push(Action::Select(today()));
        }
        let r = ui.add(egui::TextEdit::singleline(&mut app.query).hint_text(s.search_hint).desired_width(260.0));
        if r.changed() {
            app.actions.push(Action::Search);
        }
        let r = ui
            .add(egui::TextEdit::singleline(&mut app.thought).hint_text(s.capture_hint).desired_width(320.0))
            .on_hover_text(s.capture_tip);
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            app.actions.push(Action::Capture);
            // Ready for the next thought.
            r.request_focus();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut autosave = !app.prefs.manual_save;
            if ui.checkbox(&mut autosave, s.autosave).on_hover_text(s.autosave_hint).changed() {
                app.prefs.manual_save = !autosave;
            }
            if app.prefs.manual_save {
                let dirty = app.dirty_since.is_some();
                if ui
                    .add_enabled(dirty, egui::Button::new(s.save))
                    .on_hover_text(ui.ctx().format_shortcut(&crate::app::SAVE_SHORTCUT))
                    .clicked()
                {
                    app.actions.push(Action::Save);
                }
            }
            ui.separator();
            if let Some(diary) = &app.diary {
                ui.label(egui::RichText::new(diary.root().display().to_string()).weak().small());
            }
        });
    });
}

pub fn status_bar(ui: &mut egui::Ui, app: &mut DiaryCraftApp) {
    let (left, right) = app.status();
    ui.horizontal(|ui| {
        if let Some(e) = &app.error {
            ui.label(egui::RichText::new(e).color(ui.visuals().error_fg_color));
        } else if let Some((n, _)) = &app.notice {
            ui.label(egui::RichText::new(n).color(ui.visuals().warn_fg_color));
        } else {
            ui.label(egui::RichText::new(left).weak());
        }
        if let Some(right) = right {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = ui.link(egui::RichText::new(right).color(ui.visuals().warn_fg_color));
                if r.clicked()
                    && let Some(diary) = &app.diary
                    && let Some((_, copy)) = diary.conflict_copies().first()
                    && let Some(dir) = copy.parent()
                {
                    app.actions.push(Action::OpenPath(dir.to_owned()));
                }
            });
        }
    });
}

fn calendar(ui: &mut egui::Ui, app: &mut DiaryCraftApp) {
    let s = app.strings();
    let month = app.month;
    ui.horizontal(|ui| {
        if ui.button("◀").clicked() {
            app.actions.push(Action::ShowMonth(add_months(month, -1)));
        }
        ui.label(egui::RichText::new(format!("{} {}", s.month(month.month()), month.year())).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("▶").clicked() {
                app.actions.push(Action::ShowMonth(add_months(month, 1)));
            }
        });
    });
    let selected = app.page.as_ref().map(|p| p.loaded.date);
    let now = today();
    let start = app.diary.as_ref().map(|d| d.config().week_start()).unwrap_or_default().weekday();
    egui::Grid::new("month").num_columns(7).spacing([2.0, 2.0]).show(ui, |ui| {
        for wd in week_days(start) {
            let name = s.weekdays.get(wd.num_days_from_monday() as usize).copied().unwrap_or("");
            ui.centered_and_justified(|ui| ui.label(egui::RichText::new(name).weak().small()));
        }
        ui.end_row();
        for week in month_grid(month, start) {
            for cell in week {
                match cell {
                    None => {
                        ui.label("");
                    }
                    Some(day) => {
                        let has = app.diary.as_ref().is_some_and(|d| d.has_entry(day));
                        let mut text = egui::RichText::new(day.day().to_string());
                        if has {
                            text = text.strong().color(ui.visuals().hyperlink_color);
                        }
                        if day == now {
                            text = text.underline();
                        }
                        let unsaved = app.drafts.contains_key(&day)
                            || (selected == Some(day) && app.dirty_since.is_some() && app.prefs.manual_save);
                        if unsaved {
                            text = text.italics().color(ui.visuals().warn_fg_color);
                        }
                        let button =
                            egui::Button::selectable(selected == Some(day), text).min_size(egui::vec2(28.0, 24.0));
                        if ui.add(button).clicked() {
                            app.actions.push(Action::Select(day));
                        }
                    }
                }
            }
            ui.end_row();
        }
    });
}

pub fn side(ui: &mut egui::Ui, app: &mut DiaryCraftApp) {
    let s = app.strings();
    calendar(ui, app);
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match &app.hits {
        Some(hits) => {
            ui.label(egui::RichText::new(s.results).strong());
            if hits.is_empty() {
                ui.label(egui::RichText::new(s.no_results).weak());
            }
            for hit in hits {
                let label = format!("{}  {}", hit.date.format(s.short_date), hit.snippet);
                if ui.selectable_label(false, label).clicked() {
                    app.actions.push(Action::Select(hit.date));
                }
            }
        }
        None => {
            ui.label(egui::RichText::new(s.recent_entries).strong());
            for (date, title) in &app.recent {
                let label = format!("{}  {title}", date.format(s.short_date));
                if ui.selectable_label(false, label).clicked() {
                    app.actions.push(Action::Select(*date));
                }
            }
        }
    });
}

pub fn empty(ui: &mut egui::Ui, app: &mut DiaryCraftApp) {
    let s = app.strings();
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.heading(s.empty_title);
        ui.add_space(8.0);
        ui.set_max_width(520.0);
        ui.label(s.empty_body);
        ui.add_space(12.0);
        if let Some(folder) = suggested_folder() {
            ui.label(egui::RichText::new(folder.display().to_string()).monospace());
            if ui.button(egui::RichText::new(s.use_suggested).strong()).clicked() {
                app.actions.push(Action::Create(folder));
            }
            ui.add_space(4.0);
        }
        if ui.button(s.choose_other).clicked() {
            app.actions.push(Action::OpenDialog);
        }
        ui.add_space(12.0);
        for p in app.prefs.recent.clone() {
            if ui.link(p.display().to_string()).clicked() {
                app.actions.push(Action::Open(p));
            }
        }
    });
}

fn conflict_banner(ui: &mut egui::Ui, app: &mut DiaryCraftApp) {
    let s = app.strings();
    let Some(conflict) = &mut app.conflict else { return };
    egui::Frame::group(ui.style()).fill(ui.visuals().faint_bg_color).show(ui, |ui| {
        ui.label(egui::RichText::new(s.conflict_title).strong().color(ui.visuals().warn_fg_color));
        ui.horizontal_wrapped(|ui| {
            ui.label(s.conflict_body);
            if ui.link(conflict.copy.display().to_string()).clicked()
                && let Some(dir) = conflict.copy.parent()
            {
                app.actions.push(Action::OpenPath(dir.to_owned()));
            }
        });
        ui.horizontal(|ui| {
            if ui.button(s.compare).clicked() {
                conflict.comparing = true;
            }
            if ui.button(s.use_disk).clicked() {
                app.actions.push(Action::UseDisk);
            }
            if ui.button(s.keep_mine).clicked() {
                app.actions.push(Action::KeepMine);
            }
        });
    });
}

pub fn page(ui: &mut egui::Ui, app: &mut DiaryCraftApp, time: f64) {
    let s = app.strings();
    conflict_banner(ui, app);
    let Some(page) = &mut app.page else { return };
    let date = page.loaded.date;
    ui.horizontal(|ui| {
        ui.heading(s.long_date(date));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(s.attach).clicked() {
                app.actions.push(Action::Attach);
            }
            ui.separator();
            if ui.selectable_label(app.prefs.preview, s.preview).clicked() {
                app.prefs.preview = true;
            }
            if ui.selectable_label(!app.prefs.preview, s.write).clicked() {
                app.prefs.preview = false;
            }
        });
    });
    if !page.attachments.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(format!("{}:", s.attachments)).weak());
            for a in &page.attachments {
                let name = a.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if ui.link(name).clicked() {
                    app.actions.push(Action::OpenPath(a.clone()));
                }
            }
        });
    }
    ui.separator();
    if app.prefs.preview {
        let body = diarycraft_engine::text::split_front_matter(&page.text).1;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui_commonmark::CommonMarkViewer::new().show(ui, &mut app.markdown_cache, body);
        });
        // The links of the formatted view: web addresses go to the browser, relative ones to the
        // files of the diary (the attachments). egui would open neither by itself.
        let urls = ui.ctx().output_mut(|o| {
            let (urls, rest) = std::mem::take(&mut o.commands)
                .into_iter()
                .partition::<Vec<_>, _>(|c| matches!(c, egui::OutputCommand::OpenUrl(_)));
            o.commands = rest;
            urls
        });
        if let (Some(dir), Some(diary)) = (page.loaded.path.parent(), &app.diary) {
            for command in urls {
                if let egui::OutputCommand::OpenUrl(open) = command
                    && let Some(target) = crate::markdown::resolve_link(&open.url, dir, diary.root())
                {
                    app.actions.push(Action::OpenPath(target));
                }
            }
        }
        return;
    }
    let locked = app.conflict.is_some();
    let mut changed = false;
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let job = crate::markdown::layout_job(ui, buf.as_str(), wrap_width);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let edit = egui::TextEdit::multiline(&mut page.text)
            .layouter(&mut layouter)
            .desired_width(f32::INFINITY)
            .desired_rows(24)
            .frame(egui::Frame::NONE)
            .lock_focus(true)
            .interactive(!locked);
        changed = ui.add_sized(ui.available_size(), edit).changed();
    });
    if changed {
        app.mark_dirty(time);
    }
}

pub fn compare_window(ctx: &egui::Context, app: &mut DiaryCraftApp) {
    let s = app.strings();
    let (Some(conflict), Some(page)) = (&mut app.conflict, &app.page) else { return };
    if !conflict.comparing {
        return;
    }
    let mut open = true;
    egui::Window::new(s.compare).open(&mut open).default_size([900.0, 560.0]).show(ctx, |ui| {
        ui.columns(2, |cols| {
            if let [left, right] = cols {
                left.label(egui::RichText::new(s.my_version).strong());
                egui::ScrollArea::vertical().id_salt("mine").show(left, |ui| {
                    ui.add(egui::Label::new(page.text.as_str()).wrap());
                });
                right.label(egui::RichText::new(s.disk_version).strong());
                egui::ScrollArea::vertical().id_salt("disk").show(right, |ui| {
                    ui.add(egui::Label::new(conflict.disk_text.as_str()).wrap());
                });
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(s.use_disk).clicked() {
                app.actions.push(Action::UseDisk);
            }
            if ui.button(s.keep_mine).clicked() {
                app.actions.push(Action::KeepMine);
            }
        });
    });
    if !open {
        conflict.comparing = false;
    }
}

/// With autosave off, leaving with unsaved pages asks what to do with them.
pub fn unsaved_window(ctx: &egui::Context, app: &mut DiaryCraftApp) {
    if app.leaving.is_none() {
        return;
    }
    let s = app.strings();
    let days: Vec<String> = app.unsaved_days().iter().map(|d| s.long_date(*d)).collect();
    egui::Modal::new(egui::Id::unique("unsaved")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.heading(s.unsaved_title);
        ui.label(s.unsaved_body);
        for day in &days {
            ui.label(format!("• {day}"));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(s.save).clicked() {
                app.actions.push(Action::SaveAndLeave);
            }
            if ui.button(s.discard).clicked() {
                app.actions.push(Action::DiscardAndLeave);
            }
            if ui.button(s.cancel).clicked() {
                app.actions.push(Action::CancelLeave);
            }
        });
    });
}
