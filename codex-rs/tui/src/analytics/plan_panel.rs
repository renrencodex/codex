//! Five-hour and weekly period tables share grouping, with independent row expansion.
use super::AnalyticsView;
use super::data;
use super::plan;
use super::render::columns;
use super::render::join_columns;
use super::sections::Section;
use super::styles::secondary_style;
use crate::keymap::ListAction;
use crate::style::accent_style;
use codex_backend_client::PlanLimitDimension;
use ratatui::style::Styled;
use ratatui::style::Stylize;
use ratatui::text::Line;
use std::ops::Range;

impl AnalyticsView {
    pub(super) fn plan_action(&mut self, action: Option<ListAction>) {
        let window = self.plan.window;
        let periods = self
            .plan
            .report
            .ready()
            .map(|report| &report.periods[window]);
        let count = periods.map_or(/*default*/ 0, Vec::len);
        let cursor = &mut self.plan.cursor[window];
        let previous_cursor = *cursor;
        match action {
            Some(ListAction::MoveLeft | ListAction::MoveRight) => self.plan.window ^= 1,
            Some(ListAction::MoveUp) => *cursor = cursor.saturating_sub(/*rhs*/ 1),
            Some(ListAction::MoveDown) => {
                *cursor = (*cursor + 1).min(count.saturating_sub(/*rhs*/ 1))
            }
            Some(ListAction::JumpTop) => *cursor = 0,
            Some(ListAction::JumpBottom) => *cursor = count.saturating_sub(/*rhs*/ 1),
            Some(ListAction::Accept) => {
                if let Some(period) = periods.and_then(|periods| periods.get(*cursor)) {
                    self.plan.expanded[window] =
                        if self.plan.expanded[window].as_ref() == Some(&period.id) {
                            None
                        } else {
                            Some(period.id.clone())
                        };
                }
            }
            Some(ListAction::Cancel) => {
                self.is_done |= !self.zoomed || self.plan.expanded[window].take().is_none();
            }
            Some(ListAction::PageDown) => {
                self.scroll_offset += self.viewport_height;
                self.follow_selection = false;
            }
            Some(ListAction::PageUp) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(self.viewport_height);
                self.follow_selection = false;
            }
            None => {}
        }
        if *cursor != previous_cursor {
            self.plan.expanded[window] = None;
        }
    }

    pub(super) fn plan_lines(&self, width: usize) -> (Vec<Line<'static>>, Range<usize>) {
        let side_by_side = width >= 100;
        let table_width = if side_by_side { (width - 4) / 2 } else { width };
        let group = self.sections[Section::Plan].group;
        let dimension = match group {
            1 => PlanLimitDimension::ThreadSource,
            2 => PlanLimitDimension::Model,
            3 => PlanLimitDimension::TurnTrigger,
            _ => PlanLimitDimension::Surface,
        };
        let mut tables = Vec::new();
        let mut selections = Vec::new();
        for (window, title) in ["5 小时限额", "每周限额"].into_iter().enumerate() {
            let heading = format!(
                "{} {title}",
                if self.plan.window == window {
                    "›"
                } else {
                    " "
                }
            );
            let mut lines = vec![heading.bold().into(), Line::default()];
            let mut selection = 0..2;
            if let Some(report) = self.plan.report.ready() {
                if report.periods[window].is_empty() {
                    lines.push(
                        if report.coverage_complete {
                            "此日期范围内没有限额周期"
                        } else {
                            "限额历史暂不可用"
                        }
                        .set_style(secondary_style())
                        .into(),
                    );
                }
                for (index, period) in report.periods[window]
                    .iter()
                    .take(if self.zoomed { usize::MAX } else { 1 })
                    .enumerate()
                {
                    let start = lines.len();
                    let selected = self.plan.cursor[window] == index;
                    let label = if !self.zoomed {
                        format!("最新周期 · 共 {} 个", report.periods[window].len())
                    } else {
                        format!(
                            "{} {} – {}{}",
                            if selected { "›" } else { " " },
                            period.start.format("%b %-d %H:%M"),
                            period.end.format("%b %-d %H:%M"),
                            if period.start <= report.as_of && period.end > report.as_of {
                                " *"
                            } else {
                                ""
                            }
                        )
                    };
                    let amount = period
                        .used
                        .map(|value| format!("{}%", data::amount(value / 100.0)))
                        .unwrap_or_else(|| "不可用".into());
                    let row = columns(label.into(), amount.into(), table_width);
                    lines.push(if selected && self.plan.window == window {
                        row.set_style(accent_style())
                    } else {
                        row
                    });
                    if self.zoomed && self.plan.expanded[window].as_ref() == Some(&period.id) {
                        if !period.complete {
                            lines.push("  部分用量不可用；此周期的数据可能不完整".dim().into());
                        }
                        if let Some(breakdown) = period.breakdowns.as_ref().and_then(|groups| {
                            groups.iter().find(|group| group.dimension == dimension)
                        }) {
                            for row in &breakdown.rows {
                                let key = if dimension == PlanLimitDimension::TurnTrigger {
                                    format!("start-{}", row.key)
                                } else {
                                    row.key.clone()
                                };
                                let label = if dimension == PlanLimitDimension::Model {
                                    self.model_name(&key)
                                } else {
                                    super::normalize::label(&key)
                                };
                                lines.push(columns(
                                    format!("  {label}").into(),
                                    format!("{}%", data::amount(row.basis_points / 100.0)).into(),
                                    table_width,
                                ));
                            }
                        } else {
                            lines.push("  此周期的明细不可用".dim().into());
                        }
                    }
                    if selected {
                        selection = start..lines.len();
                    }
                }
            } else {
                lines.push(
                    match &self.plan.report {
                        data::Load::Unavailable => "限额历史暂不可用",
                        state => state.message().unwrap_or("限额历史暂不可用"),
                    }
                    .to_string()
                    .set_style(secondary_style())
                    .into(),
                );
            }
            // Wrap each column before joining so long timestamps never collide with its neighbor.
            let mut wrapped = Vec::new();
            let mut wrapped_selection = 0..0;
            for (index, line) in lines.into_iter().enumerate() {
                if index == selection.start {
                    wrapped_selection.start = wrapped.len();
                }
                wrapped.extend(crate::wrapping::word_wrap_lines(
                    [line],
                    crate::wrapping::RtOptions::new(table_width.max(/*other*/ 1)),
                ));
                if index + 1 == selection.end {
                    wrapped_selection.end = wrapped.len();
                }
            }
            tables.push(wrapped);
            selections.push(wrapped_selection);
        }
        let mut selection = selections[self.plan.window].clone();
        let mut lines = if side_by_side {
            join_columns(&tables[0], &tables[1], table_width)
        } else {
            if self.plan.window == 1 {
                selection =
                    selection.start + tables[0].len() + 1..selection.end + tables[0].len() + 1;
            }
            let mut lines = std::mem::take(&mut tables[0]);
            lines.push(Line::default());
            lines.append(&mut tables[1]);
            lines
        };
        if group == 3 {
            lines.push(
                "按回合启动方式统计时仅包含任务；百分比按完整周期限额计算"
                    .dim()
                    .into(),
            );
        }
        if let Some(plan::Report {
            as_of,
            coverage_start,
            coverage_complete,
            approximate,
            ..
        }) = self.plan.report.ready()
        {
            if !*coverage_complete {
                lines.push("部分周期暂不可用".dim().into());
            }
            lines.push(
                format!(
                    "用量截至 {} UTC{}",
                    as_of.format("%b %-d %H:%M"),
                    if self.zoomed {
                        " · * 最近更新时仍为当前周期"
                    } else {
                        ""
                    }
                )
                .dim()
                .into(),
            );
            if let Some(start) = coverage_start {
                lines.push(
                    format!("可用数据始于 {} UTC", start.format("%b %-d %H:%M"))
                        .dim()
                        .into(),
                );
            }
            if *approximate {
                lines.push("数值和周期边界为近似值；近期活动可能有延迟".dim().into());
            }
        }
        (lines, selection)
    }
}
