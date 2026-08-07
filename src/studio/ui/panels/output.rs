use bevy::prelude::*;
use bevy_egui::egui;
use crate::scripting::ecs::{LocalScript, ModuleScript, ServerScript};
use crate::scripting::output::{OutputEntry, OutputLevel, RunInfo};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OutputTab {
    Output,
    StackTrace,
    Debugger,
}

#[derive(Resource)]
pub struct OutputPanelState {
    pub tab: OutputTab,
    pub selected_error: Option<u64>,
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
    pub autoscroll: bool,
    pub search: String,
    pub filter_run: Option<u64>,
}

impl Default for OutputPanelState {
    fn default() -> Self {
        Self {
            tab: OutputTab::Output,
            selected_error: None,
            show_info: true,
            show_warn: true,
            show_error: true,
            autoscroll: true,
            search: String::new(),
            filter_run: None,
        }
    }
}

type ExplorerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Name,
        Option<&'static ChildOf>,
        Option<&'static Children>,
        Option<&'static crate::common::game::bricks::components::Brick>,
        Option<&'static ServerScript>,
        Option<&'static LocalScript>,
        Option<&'static ModuleScript>,
    ),
    Without<Camera3d>,
>;

fn level_color(level: OutputLevel) -> egui::Color32 {
    match level {
        OutputLevel::Info => egui::Color32::from_rgb(90, 160, 230),
        OutputLevel::Warn => egui::Color32::from_rgb(230, 180, 60),
        OutputLevel::Error => egui::Color32::from_rgb(220, 80, 70),
    }
}

fn level_label(level: OutputLevel) -> &'static str {
    match level {
        OutputLevel::Info => "INFO",
        OutputLevel::Warn => "WARN",
        OutputLevel::Error => "ERROR",
    }
}

fn format_time(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{:02}:{:02}.{:03}", total / 60, total % 60, ((secs.fract()) * 1000.0) as u64)
}

fn run_label(run: &RunInfo) -> String {
    format!("Run #{} — {}", run.id + 1, run.label)
}

fn tab_button(ui: &mut egui::Ui, selected: &mut OutputTab, tab: OutputTab, label: &str) {
    let response = ui.selectable_label(*selected == tab, label);
    if response.clicked() {
        *selected = tab;
    }
}

fn draw_header(ui: &mut egui::Ui, state: &mut OutputPanelState) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Output").strong().size(14.0));
        ui.separator();
        tab_button(ui, &mut state.tab, OutputTab::Output, "Output");
        tab_button(ui, &mut state.tab, OutputTab::StackTrace, "Stack Trace");
        tab_button(ui, &mut state.tab, OutputTab::Debugger, "Debugger");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear").clicked() {
                crate::scripting::output::clear_entries();
            }
            ui.checkbox(&mut state.autoscroll, "Auto-scroll");
            if ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("Search...").desired_width(160.0)).changed() {
                state.selected_error = None;
            }
            ui.separator();
            ui.checkbox(&mut state.show_error, "Errors");
            ui.checkbox(&mut state.show_warn, "Warnings");
            ui.checkbox(&mut state.show_info, "Info");
        });
    });
}

fn entry_matches(entry: &OutputEntry, state: &OutputPanelState) -> bool {
    if let Some(run) = state.filter_run {
        if entry.run_id != run {
            return false;
        }
    }
    let level_ok = match entry.level {
        OutputLevel::Info => state.show_info,
        OutputLevel::Warn => state.show_warn,
        OutputLevel::Error => state.show_error,
    };
    if !level_ok {
        return false;
    }
    if state.search.is_empty() {
        return true;
    }
    let needle = state.search.to_lowercase();
    entry.message.to_lowercase().contains(&needle)
        || entry.source.to_lowercase().contains(&needle)
        || entry.traceback.as_ref().is_some_and(|t| t.to_lowercase().contains(&needle))
}

fn draw_entry_row(ui: &mut egui::Ui, entry: &OutputEntry, state: &mut OutputPanelState, run_label: &str, new_run: bool) {
    if new_run {
        ui.add_space(4.0);
        ui.separator();
        ui.label(egui::RichText::new(format!("--- {run_label} ---")).weak().small());
        ui.add_space(2.0);
    }

    let is_error = entry.level == OutputLevel::Error;
    let is_selected = state.selected_error == Some(entry.id);

    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
    let (row_rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_height), egui::Sense::click());
    let hovered = response.hovered();
    let fill = if is_selected {
        egui::Color32::from_rgb(230, 238, 248)
    } else if hovered {
        egui::Color32::from_rgb(240, 242, 245)
    } else {
        egui::Color32::TRANSPARENT
    };

    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    egui::Frame::NONE
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(&mut row_ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 12.0), egui::Sense::hover());
            ui.painter().rect_filled(
                egui::Rect::from_center_size(dot_rect.center(), egui::vec2(8.0, 8.0)),
                4.0,
                level_color(entry.level),
            );
            ui.label(egui::RichText::new(format_time(entry.time)).monospace().weak().size(11.0));
            ui.label(egui::RichText::new(format!("[{}]", level_label(entry.level)))
                .monospace().size(11.0).color(level_color(entry.level)));
            let source = if let Some(line) = entry.line {
                format!("{}:{}", entry.source, line)
            } else {
                entry.source.clone()
            };
            ui.label(egui::RichText::new(source).monospace().size(11.0).color(egui::Color32::from_rgb(120, 120, 120)));
            ui.label(egui::RichText::new(&entry.message).size(12.5).color(if is_error { egui::Color32::from_rgb(160, 40, 35) } else { egui::Color32::from_rgb(40, 40, 40) }));
        });

    if is_error {
        if response.clicked() {
            state.selected_error = Some(entry.id);
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    if is_selected {
        if let Some(traceback) = &entry.traceback {
            ui.add_space(2.0);
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(24, 4))
                .show(ui, |ui| {
                    ui.add(egui::Label::new(
                        egui::RichText::new(traceback).monospace().size(11.5).color(egui::Color32::from_rgb(150, 60, 55)),
                    ).wrap());
                });
        }
    }
}

fn draw_output_tab(ui: &mut egui::Ui, state: &mut OutputPanelState) {
    let guard = crate::scripting::output::buffer().lock().unwrap();

    ui.horizontal(|ui| {
        ui.label("Run:");
        egui::ComboBox::from_id_salt("output_run_filter")
            .selected_text(
                state
                    .filter_run
                    .and_then(|id| guard.runs.iter().find(|r| r.id == id))
                    .map(run_label)
                    .unwrap_or_else(|| "All runs".to_string()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.filter_run, None, "All runs");
                for run in &guard.runs {
                    let label = run_label(run);
                    ui.selectable_value(&mut state.filter_run, Some(run.id), label);
                }
            });
        let error_count = guard.entries.iter().filter(|e| e.level == OutputLevel::Error).count();
        let warn_count = guard.entries.iter().filter(|e| e.level == OutputLevel::Warn).count();
        ui.label(egui::RichText::new(format!("{} entries | {} errors | {} warnings", guard.entries.len(), error_count, warn_count)).weak().small());
    });

    ui.separator();

    let mut scroll = egui::ScrollArea::vertical()
        .id_salt("output_scroll")
        .auto_shrink([false, false]);
    if state.autoscroll {
        scroll = scroll.stick_to_bottom(true);
    }

    scroll.show(ui, |ui| {
        let mut last_run: Option<u64> = None;
        for entry in guard.entries.iter() {
            if !entry_matches(entry, state) {
                continue;
            }
            let new_run = last_run.map_or(true, |last| last != entry.run_id);
            let label = guard
                .runs
                .iter()
                .find(|r| r.id == entry.run_id)
                .map(|r| run_label(r))
                .unwrap_or_else(|| "?".to_string());
            last_run = Some(entry.run_id);
            draw_entry_row(ui, entry, state, &label, new_run);
        }
    });
}

fn draw_stack_trace_tab(ui: &mut egui::Ui, state: &mut OutputPanelState) {
    let guard = crate::scripting::output::buffer().lock().unwrap();
    let selected = state.selected_error.and_then(|id| guard.entries.iter().find(|e| e.id == id).cloned());

    match selected {
        Some(entry) => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{}:{}", entry.source, entry.line.map_or(0, |l| l)))
                    .strong()
                    .size(13.0));
                if ui.button("Copy").clicked() {
                    let text = entry.traceback.clone().unwrap_or_else(|| entry.message.clone());
                    ui.ctx().copy_text(text);
                }
                if ui.button("Clear selection").clicked() {
                    state.selected_error = None;
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&entry.message).strong().size(13.0).color(level_color(entry.level)));
            ui.add_space(6.0);
            let text = entry.traceback.clone().unwrap_or_else(|| "No stack trace available.".to_string());
            egui::ScrollArea::both()
                .id_salt("stack_trace_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(egui::Label::new(
                        egui::RichText::new(text).monospace().size(12.0).color(egui::Color32::from_rgb(150, 60, 55)),
                    ).wrap());
                });
        }
        None => {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Click an error in the Output tab to inspect its stack trace.").weak());
            });
        }
    }
}

fn draw_debugger_tab(ui: &mut egui::Ui, explorer_query: &ExplorerQuery) {
    let guard = crate::scripting::output::buffer().lock().unwrap();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Runs").strong().size(13.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("New run").clicked() {
                crate::scripting::output::start_run("Manual");
            }
        });
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("debugger_runs_scroll")
        .max_height(150.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for run in guard.runs.iter().rev() {
                let entries_in_run = guard.entries.iter().filter(|e| e.run_id == run.id).count();
                let errors_in_run = guard.entries.iter().filter(|e| e.run_id == run.id && e.level == OutputLevel::Error).count();
                let duration = match run.ended_at {
                    Some(end) => format!("{}s", (end - run.started_at).max(0.0) as u64),
                    None => "active".to_string(),
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(run_label(run)).strong().size(12.0));
                    ui.label(egui::RichText::new(duration).weak().size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{} entries | {} errors | {} scripts",
                            entries_in_run, errors_in_run, run.script_count)).weak().size(11.0));
                    });
                });
            }
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Scripts").strong().size(13.0));
    ui.separator();

    let mut script_rows: Vec<(String, &'static str, bool, bool, usize)> = Vec::new();
    for (_, name, _, _, _, server, local, module) in explorer_query.iter() {
        if let Some(s) = server {
            script_rows.push((name.to_string(), "Server", s.enabled, s.started, s.code.len()));
        } else if let Some(l) = local {
            script_rows.push((name.to_string(), "Local", l.enabled, l.started, l.code.len()));
        } else if let Some(m) = module {
            script_rows.push((name.to_string(), "Module", true, false, m.code.len()));
        }
    }
    script_rows.sort_by(|a, b| a.0.cmp(&b.0));

    egui::ScrollArea::vertical()
        .id_salt("debugger_scripts_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if script_rows.is_empty() {
                ui.label(egui::RichText::new("No scripts in the scene.").weak());
            } else {
                egui::Grid::new("debugger_script_grid")
                    .striped(true)
                    .spacing(egui::vec2(16.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Name").strong().small());
                        ui.label(egui::RichText::new("Type").strong().small());
                        ui.label(egui::RichText::new("Enabled").strong().small());
                        ui.label(egui::RichText::new("Running").strong().small());
                        ui.label(egui::RichText::new("Size").strong().small());
                        ui.end_row();
                        for (name, kind, enabled, running, size) in &script_rows {
                            ui.label(egui::RichText::new(name).size(12.0));
                            ui.label(egui::RichText::new(*kind).size(12.0).weak());
                            ui.label(egui::RichText::new(if *enabled { "yes" } else { "no" }).size(12.0).color(if *enabled { egui::Color32::from_rgb(70, 150, 80) } else { egui::Color32::from_rgb(170, 170, 170) }));
                            ui.label(egui::RichText::new(if *running { "yes" } else { "no" }).size(12.0).color(if *running { egui::Color32::from_rgb(70, 150, 80) } else { egui::Color32::from_rgb(170, 170, 170) }));
                            ui.label(egui::RichText::new(format!("{} B", size)).size(12.0).weak());
                            ui.end_row();
                        }
                    });
            }
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Engine totals").strong().size(13.0));
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(format!("Scripts started: {}", guard.script_starts));
        ui.separator();
        ui.label(format!("Buffered entries: {}", guard.entries.len()));
    });
}

pub fn draw_output(
    ui: &mut egui::Ui,
    state: &mut OutputPanelState,
    explorer_query: &ExplorerQuery,
) {
    ui.style_mut().visuals = egui::Visuals::light();
    ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 6.0);

    draw_header(ui, state);
    ui.separator();
    ui.add_space(2.0);

    match state.tab {
        OutputTab::Output => draw_output_tab(ui, state),
        OutputTab::StackTrace => draw_stack_trace_tab(ui, state),
        OutputTab::Debugger => draw_debugger_tab(ui, explorer_query),
    }
}
