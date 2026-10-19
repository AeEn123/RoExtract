//! The asset table: column headers that can be sorted, dragged to reorder,
//! resized and hidden, and the cells of each row.
//!
//! Column widths are stored as fractions of the table width, so the table
//! follows the window size. Columns that don't fit are dropped, least
//! important first.

use std::{cmp::Ordering, time::SystemTime};

use fluent_bundle::{FluentBundle, FluentResource};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    config,
    gui::player,
    locale,
    logic::{audio, AssetInfo, Category},
};

const CONFIG_KEY: &str = "file_table";
/// Width of the grab area on a column's right edge.
const RESIZE_HANDLE_WIDTH: f32 = 8.0;
/// Space between a cell's edge and its text.
pub const CELL_PADDING: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    Name,
    Type,
    Size,
    Length,
    Created,
    Modified,
}

impl Column {
    const ALL: [Column; 6] = [
        Column::Name,
        Column::Type,
        Column::Size,
        Column::Length,
        Column::Created,
        Column::Modified,
    ];

    fn locale_key(self) -> &'static str {
        match self {
            Column::Name => "column-name",
            Column::Type => "column-type",
            Column::Size => "column-size",
            Column::Length => "column-length",
            Column::Created => "column-created",
            Column::Modified => "column-modified",
        }
    }

    fn default_width(self) -> f32 {
        match self {
            Column::Name => 0.36,
            Column::Type => 0.08,
            Column::Size => 0.1,
            Column::Length => 0.1,
            Column::Created => 0.18,
            Column::Modified => 0.18,
        }
    }

    fn min_width(self) -> f32 {
        match self {
            Column::Name => 120.0,
            _ => 60.0,
        }
    }

    /// Lower is kept longer when the table gets narrow.
    fn priority(self) -> u8 {
        match self {
            Column::Name => 0,
            Column::Size => 1,
            Column::Length => 2,
            Column::Type => 3,
            Column::Modified => 4,
            Column::Created => 5,
        }
    }

    fn align(self) -> egui::Align {
        match self {
            Column::Size | Column::Length => egui::Align::RIGHT,
            _ => egui::Align::LEFT,
        }
    }

    /// Whether this column means anything for assets of `category`.
    fn applies_to(self, category: Category) -> bool {
        match self {
            Column::Length => matches!(category, Category::Music | Category::Sounds),
            _ => true,
        }
    }

    /// The text shown in this column's cell for `asset`.
    pub fn cell_text(self, asset: &AssetInfo, alias: &str) -> String {
        match self {
            Column::Name => alias.to_owned(),
            Column::Type => asset.format.unwrap_or_default().to_uppercase(),
            Column::Size => format_size(asset.size),
            // Worked out in the background the first time the row is shown
            Column::Length => match audio::length(asset) {
                Some(length) => player::format_duration(length),
                None if audio::is_pending(asset) => "…".to_owned(),
                None => String::new(),
            },
            Column::Created => asset.created.map(format_time).unwrap_or_default(),
            Column::Modified => asset.last_modified.map(format_time).unwrap_or_default(),
        }
    }

    fn compare(self, a: &AssetInfo, b: &AssetInfo) -> Ordering {
        match self {
            // Name is compared on precomputed aliases in `sort_assets`.
            Column::Name => Ordering::Equal,
            Column::Type => a.format.cmp(&b.format),
            Column::Size => a.size.cmp(&b.size),
            Column::Length => compare_known(audio::cached_length(a), audio::cached_length(b)),
            Column::Created => compare_times(a.created, b.created),
            Column::Modified => compare_times(a.last_modified, b.last_modified),
        }
    }
}

/// Unknown times sort last.
fn compare_times(a: Option<SystemTime>, b: Option<SystemTime>) -> Ordering {
    compare_known(a, b)
}

/// Unknown values sort last.
fn compare_known<T: Ord>(a: Option<T>, b: Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut size = bytes as f64 / 1024.0;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}

pub fn format_time(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sort {
    pub column: Column,
    pub ascending: bool,
}

/// Sort `assets` in place. Ties keep their listing order.
pub fn sort_assets(assets: &mut [AssetInfo], sort: Sort) {
    if sort.column == Column::Name {
        assets.sort_by_cached_key(|asset| config::get_asset_alias(&asset.name).to_lowercase());
    } else {
        assets.sort_by(|a, b| sort.column.compare(a, b));
    }
    if !sort.ascending {
        assets.reverse();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ColumnLayout {
    column: Column,
    /// Relative width; the shown columns share the table width in proportion.
    width: f32,
    visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableLayout {
    columns: Vec<ColumnLayout>,
    pub sort: Option<Sort>,
    /// The column whose header is being dragged.
    #[serde(skip)]
    dragging: Option<Column>,
}

impl Default for TableLayout {
    fn default() -> Self {
        Self {
            columns: Column::ALL
                .iter()
                .map(|&column| ColumnLayout {
                    column,
                    width: column.default_width(),
                    visible: true,
                })
                .collect(),
            sort: None,
            dragging: None,
        }
    }
}

/// A column as laid out this frame.
#[derive(Clone, Copy, Debug)]
pub struct ShownColumn {
    pub column: Column,
    /// Offset from the table's left edge.
    pub x: f32,
    pub width: f32,
}

impl TableLayout {
    pub fn load() -> Self {
        let layout = config::get_config()
            .get(CONFIG_KEY)
            .and_then(|value| serde_json::from_value::<TableLayout>(value.clone()).ok())
            .unwrap_or_default();
        layout.repaired()
    }

    fn save(&self) {
        if let Ok(value) = serde_json::to_value(self) {
            config::set_config_value(CONFIG_KEY, value);
        }
    }

    /// Each column exactly once, with a usable width, and Name always visible.
    fn repaired(mut self) -> Self {
        let mut seen = Vec::new();
        self.columns.retain(|layout| {
            let new = !seen.contains(&layout.column);
            seen.push(layout.column);
            new
        });
        for column in Column::ALL {
            if !seen.contains(&column) {
                self.columns.push(ColumnLayout {
                    column,
                    width: column.default_width(),
                    visible: true,
                });
            }
        }
        for layout in &mut self.columns {
            if !layout.width.is_finite() || layout.width <= 0.0 {
                layout.width = layout.column.default_width();
            }
            if layout.column == Column::Name {
                layout.visible = true;
            }
        }
        self
    }

    /// The columns shown for `category` in a table `width` wide, dropping the
    /// least important ones that don't fit.
    pub fn shown_columns(&self, width: f32, category: Category) -> Vec<ShownColumn> {
        let mut shown: Vec<&ColumnLayout> = self
            .columns
            .iter()
            .filter(|layout| layout.visible && layout.column.applies_to(category))
            .collect();
        while shown.len() > 1
            && shown
                .iter()
                .map(|layout| layout.column.min_width())
                .sum::<f32>()
                > width
        {
            let least_important = shown
                .iter()
                .enumerate()
                .max_by_key(|(_, layout)| layout.column.priority())
                .map(|(i, _)| i)
                .unwrap();
            shown.remove(least_important);
        }

        let total: f32 = shown.iter().map(|layout| layout.width).sum();
        let mut x = 0.0;
        shown
            .iter()
            .map(|layout| {
                let column_width = width * layout.width / total;
                let column = ShownColumn {
                    column: layout.column,
                    x,
                    width: column_width,
                };
                x += column_width;
                column
            })
            .collect()
    }

    fn layout_mut(&mut self, column: Column) -> &mut ColumnLayout {
        self.columns
            .iter_mut()
            .find(|layout| layout.column == column)
            .expect("every column has a layout")
    }

    /// Move the border between two neighbouring shown columns by `delta` pixels.
    fn resize(&mut self, left: ShownColumn, right: ShownColumn, delta: f32, table_width: f32) {
        let pair = left.width + right.width;
        let new_left = (left.width + delta).clamp(
            left.column.min_width().min(pair / 2.0),
            pair - right.column.min_width().min(pair / 2.0),
        );
        // Convert pixels back to relative widths, keeping the pair's share.
        let pair_share = self.layout_mut(left.column).width + self.layout_mut(right.column).width;
        if pair <= 0.0 || table_width <= 0.0 {
            return;
        }
        self.layout_mut(left.column).width = pair_share * new_left / pair;
        self.layout_mut(right.column).width = pair_share * (pair - new_left) / pair;
    }

    /// Put `column` in front of `before` (or last when None).
    fn move_column(&mut self, column: Column, before: Option<Column>) {
        if before == Some(column) {
            return;
        }
        let Some(from) = self.columns.iter().position(|l| l.column == column) else {
            return;
        };
        let layout = self.columns.remove(from);
        let to = before
            .and_then(|before| self.columns.iter().position(|l| l.column == before))
            .unwrap_or(self.columns.len());
        self.columns.insert(to, layout);
    }

    /// Click cycles ascending, descending, then back to listing order.
    fn cycle_sort(&mut self, column: Column) {
        self.sort = match self.sort {
            Some(sort) if sort.column == column && sort.ascending => Some(Sort {
                column,
                ascending: false,
            }),
            Some(sort) if sort.column == column => None,
            _ => Some(Sort {
                column,
                ascending: true,
            }),
        };
    }

    /// Draw the header row. Returns the columns as laid out this frame.
    pub fn header_ui(
        &mut self,
        ui: &mut egui::Ui,
        locale: &FluentBundle<Arc<FluentResource>>,
        height: f32,
        category: Category,
    ) -> Vec<ShownColumn> {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let shown = self.shown_columns(width, category);
        let before = self.clone();
        let visuals = ui.visuals().clone();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let cell_rect = |column: &ShownColumn| {
            egui::Rect::from_min_size(
                rect.min + egui::vec2(column.x, 0.0),
                egui::vec2(column.width, height),
            )
        };

        ui.painter()
            .rect_filled(rect, 0.0, visuals.widgets.noninteractive.weak_bg_fill);

        for column in &shown {
            let cell = cell_rect(column);
            let response = ui.interact(
                cell.shrink2(egui::vec2(RESIZE_HANDLE_WIDTH / 2.0, 0.0)),
                ui.id().with(("table-header", column.column)),
                egui::Sense::click_and_drag(),
            );

            if response.hovered() && self.dragging.is_none() {
                ui.painter()
                    .rect_filled(cell, 0.0, visuals.widgets.hovered.weak_bg_fill);
            }
            if response.clicked() {
                self.cycle_sort(column.column);
            }
            if response.drag_started() {
                self.dragging = Some(column.column);
            }
            response.context_menu(|ui| self.columns_menu(ui, locale));

            let mut title = locale::get_message(locale, column.column.locale_key(), None);
            if let Some(sort) = self.sort.filter(|sort| sort.column == column.column) {
                title.push_str(if sort.ascending { " ⏶" } else { " ⏷" });
            }
            let colour = if self.dragging == Some(column.column) {
                visuals.weak_text_color()
            } else {
                visuals.strong_text_color()
            };
            paint_cell_text(
                ui,
                cell,
                &title,
                column.column.align(),
                font.clone(),
                colour,
            );
        }

        // Resize handles on the borders between columns
        for pair in shown.windows(2) {
            let (left, right) = (pair[0], pair[1]);
            let border = rect.left() + right.x;
            let handle = egui::Rect::from_x_y_ranges(
                border - RESIZE_HANDLE_WIDTH / 2.0..=border + RESIZE_HANDLE_WIDTH / 2.0,
                rect.y_range(),
            );
            let response = ui.interact(
                handle,
                ui.id().with(("table-resize", left.column)),
                egui::Sense::drag(),
            );
            let active = response.hovered() || response.dragged();
            if active {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }
            if response.dragged() {
                self.resize(left, right, response.drag_delta().x, width);
            }
            let stroke = if active {
                visuals.widgets.active.fg_stroke
            } else {
                visuals.widgets.noninteractive.bg_stroke
            };
            ui.painter().vline(border, rect.y_range(), stroke);
        }

        self.drag_ui(ui, rect, &shown, font, height);

        if *self != before && self.dragging.is_none() {
            self.save();
        }
        shown
    }

    /// While a header is dragged, show it under the pointer and where it would land.
    fn drag_ui(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        shown: &[ShownColumn],
        font: egui::FontId,
        height: f32,
    ) {
        let Some(dragging) = self.dragging else {
            return;
        };
        let Some(dragged) = shown.iter().find(|column| column.column == dragging) else {
            self.dragging = None;
            return;
        };
        let Some(pointer) = ui.ctx().pointer_latest_pos() else {
            return;
        };

        // Drop in front of the first column whose centre is right of the pointer
        let target = shown
            .iter()
            .find(|column| rect.left() + column.x + column.width / 2.0 > pointer.x)
            .map(|column| column.column);
        let marker_x = target
            .and_then(|target| shown.iter().find(|column| column.column == target))
            .map_or(rect.right(), |column| rect.left() + column.x);

        if ui.input(|i| i.pointer.any_released()) {
            self.dragging = None;
            self.move_column(dragging, target);
            return;
        }

        let visuals = ui.visuals();
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            ui.id().with("table-drag"),
        ));
        painter.vline(
            marker_x,
            rect.y_range(),
            egui::Stroke::new(2.0_f32, visuals.selection.bg_fill),
        );
        // Follow the pointer, kept inside the table
        let ghost_left = (pointer.x - dragged.width / 2.0)
            .min(rect.right() - dragged.width)
            .max(rect.left());
        let ghost = egui::Rect::from_min_size(
            egui::pos2(ghost_left, rect.top()),
            egui::vec2(dragged.width, height),
        );
        painter.rect_filled(ghost, 2.0, visuals.selection.bg_fill);
        let title = egui::WidgetText::from(locale::get_message(
            &locale::get_locale(None),
            dragging.locale_key(),
            None,
        ));
        let galley = title.into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            ghost.width() - CELL_PADDING * 2.0,
            font,
        );
        painter.galley(
            egui::pos2(
                ghost.left() + CELL_PADDING,
                ghost.center().y - galley.size().y / 2.0,
            ),
            galley,
            visuals.selection.stroke.color,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    /// Right-click menu on the headers: show/hide columns, reset.
    fn columns_menu(&mut self, ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
        for layout in &mut self.columns {
            if layout.column == Column::Name {
                continue; // Always shown
            }
            ui.checkbox(
                &mut layout.visible,
                locale::get_message(locale, layout.column.locale_key(), None),
            );
        }
        ui.separator();
        if ui
            .button(locale::get_message(locale, "button-reset-columns", None))
            .clicked()
        {
            *self = TableLayout::default();
            ui.close();
        }
    }
}

/// Paint `text` inside `cell`, cut short with "…" if it doesn't fit.
pub fn paint_cell_text(
    ui: &egui::Ui,
    cell: egui::Rect,
    text: &str,
    align: egui::Align,
    font: egui::FontId,
    colour: egui::Color32,
) {
    let max_width = (cell.width() - CELL_PADDING * 2.0).max(0.0);
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat::simple(font, colour),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    let x = match align {
        egui::Align::RIGHT => cell.right() - CELL_PADDING - galley.size().x,
        _ => cell.left() + CELL_PADDING,
    };
    let pos = egui::pos2(x, cell.center().y - galley.size().y / 2.0);
    ui.painter()
        .with_clip_rect(cell)
        .galley(pos, galley, colour);
}

/// Paint one row's cells.
pub fn paint_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    columns: &[ShownColumn],
    asset: &AssetInfo,
    alias: &str,
    colour: egui::Color32,
) {
    let font = egui::TextStyle::Body.resolve(ui.style());
    for column in columns {
        let cell = egui::Rect::from_min_size(
            rect.min + egui::vec2(column.x, 0.0),
            egui::vec2(column.width, rect.height()),
        );
        let text = column.column.cell_text(asset, alias);
        paint_cell_text(ui, cell, &text, column.column.align(), font.clone(), colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn asset(name: &str, size: u64, format: Option<&'static str>, age: Option<u64>) -> AssetInfo {
        AssetInfo {
            name: name.to_owned(),
            size,
            last_modified: age.map(|age| SystemTime::UNIX_EPOCH + Duration::from_secs(age)),
            created: None,
            format,
            from_file: true,
            from_sql: false,
            from_rbx_storage: false,
            category: Category::Images,
        }
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn test_sort_by_size_both_ways() {
        let mut assets = vec![
            asset("b", 30, None, None),
            asset("a", 10, None, None),
            asset("c", 20, None, None),
        ];
        let sizes = |assets: &[AssetInfo]| assets.iter().map(|a| a.size).collect::<Vec<_>>();
        sort_assets(
            &mut assets,
            Sort {
                column: Column::Size,
                ascending: true,
            },
        );
        assert_eq!(sizes(&assets), [10, 20, 30]);
        sort_assets(
            &mut assets,
            Sort {
                column: Column::Size,
                ascending: false,
            },
        );
        assert_eq!(sizes(&assets), [30, 20, 10]);
    }

    #[test]
    fn test_sort_unknown_times_last() {
        let mut assets = vec![
            asset("none", 0, None, None),
            asset("new", 0, None, Some(200)),
            asset("old", 0, None, Some(100)),
        ];
        sort_assets(
            &mut assets,
            Sort {
                column: Column::Modified,
                ascending: true,
            },
        );
        let names: Vec<_> = assets.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["old", "new", "none"]);
    }

    #[test]
    fn test_cycle_sort() {
        let mut layout = TableLayout::default();
        layout.cycle_sort(Column::Size);
        assert_eq!(
            layout.sort,
            Some(Sort {
                column: Column::Size,
                ascending: true
            })
        );
        layout.cycle_sort(Column::Size);
        assert_eq!(
            layout.sort,
            Some(Sort {
                column: Column::Size,
                ascending: false
            })
        );
        layout.cycle_sort(Column::Size);
        assert_eq!(layout.sort, None);
        layout.cycle_sort(Column::Size);
        layout.cycle_sort(Column::Name);
        assert_eq!(
            layout.sort,
            Some(Sort {
                column: Column::Name,
                ascending: true
            })
        );
    }

    #[test]
    fn test_shown_columns_fill_width() {
        let layout = TableLayout::default();
        let shown = layout.shown_columns(1000.0, Category::Sounds);
        assert_eq!(shown.len(), Column::ALL.len());
        let total: f32 = shown.iter().map(|c| c.width).sum();
        assert!((total - 1000.0).abs() < 0.01);
        assert_eq!(shown[0].x, 0.0);
    }

    #[test]
    fn test_shown_columns_drop_least_important_when_narrow() {
        let layout = TableLayout::default();
        // Room for Name (120) + Size (60) + Type (60) only
        let shown: Vec<_> = layout
            .shown_columns(250.0, Category::Images)
            .iter()
            .map(|c| c.column)
            .collect();
        assert_eq!(shown, [Column::Name, Column::Type, Column::Size]);
        // Name is always kept
        let shown: Vec<_> = layout
            .shown_columns(10.0, Category::Images)
            .iter()
            .map(|c| c.column)
            .collect();
        assert_eq!(shown, [Column::Name]);
    }

    #[test]
    fn test_length_only_for_audio() {
        let layout = TableLayout::default();
        let has_length = |category| {
            layout
                .shown_columns(1000.0, category)
                .iter()
                .any(|c| c.column == Column::Length)
        };
        assert!(has_length(Category::Sounds));
        assert!(has_length(Category::Music));
        assert!(!has_length(Category::Images));
    }

    #[test]
    fn test_move_column() {
        let mut layout = TableLayout::default();
        layout.move_column(Column::Modified, Some(Column::Name));
        let order: Vec<_> = layout.columns.iter().map(|l| l.column).collect();
        assert_eq!(order[0], Column::Modified);
        assert_eq!(order[1], Column::Name);
        layout.move_column(Column::Modified, None);
        assert_eq!(layout.columns.last().unwrap().column, Column::Modified);
    }

    #[test]
    fn test_resize_keeps_pair_width_and_respects_minimum() {
        let mut layout = TableLayout::default();
        let shown = layout.shown_columns(1000.0, Category::Sounds);
        let (name, kind) = (shown[0], shown[1]);
        layout.resize(name, kind, 20.0, 1000.0);
        let after = layout.shown_columns(1000.0, Category::Sounds);
        assert!((after[0].width - (name.width + 20.0)).abs() < 0.01);
        assert!((after[0].width + after[1].width - (name.width + kind.width)).abs() < 0.01);

        // Can't shrink the right column below its minimum
        layout.resize(after[0], after[1], 10_000.0, 1000.0);
        let after = layout.shown_columns(1000.0, Category::Sounds);
        assert!((after[1].width - Column::Type.min_width()).abs() < 0.01);
    }

    #[test]
    fn test_repaired_restores_missing_and_duplicate_columns() {
        let mut layout = TableLayout::default();
        layout.columns.remove(1);
        layout.columns.push(layout.columns[0].clone());
        layout.columns[0].visible = false;
        let layout = layout.repaired();
        assert_eq!(layout.columns.len(), Column::ALL.len());
        assert!(layout.columns.iter().any(|l| l.column == Column::Type));
        assert!(
            layout
                .columns
                .iter()
                .find(|l| l.column == Column::Name)
                .unwrap()
                .visible
        );
    }
}
