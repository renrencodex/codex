//! Live compaction status. Its wall clock is separate from the turn's running time,
//! and only a matching live completion contributes a duration to the transcript.

use super::*;

pub(super) const COMPACTION_HEADER: &str = "正在压缩上下文";
pub(super) const COMPACTION_DETAILS: &str = "正在腾出空间以继续。";

#[derive(Debug)]
pub(super) struct ActiveCompaction {
    pub(super) id: String,
    pub(super) started_at: Instant,
}

impl ChatWidget {
    pub(super) fn on_context_compaction_started(&mut self, id: String, elapsed: Duration) {
        if self
            .status_state
            .compaction
            .as_ref()
            .is_some_and(|active| active.id == id)
        {
            return;
        }
        self.flush_answer_stream_with_separator();
        let now = Instant::now();
        let started_at = now.checked_sub(elapsed).unwrap_or(now);
        self.status_state.compaction = Some(ActiveCompaction { id, started_at });
        self.bottom_pane.set_status_timer_origin(Some(started_at));
        self.bottom_pane.ensure_status_indicator();
        self.set_status_header(COMPACTION_HEADER.to_string());
    }

    pub(super) fn clear_context_compaction(&mut self) {
        if self.status_state.compaction.take().is_some() {
            self.bottom_pane
                .set_status_timer_origin(/*started_at*/ None);
            self.set_status_header("工作中".to_string());
        }
    }

    pub(super) fn on_context_compaction_completed(&mut self, id: &str, from_replay: bool) {
        let mut message = "上下文已压缩".to_string();
        if let Some(active) = self.status_state.compaction.as_ref()
            && active.id == id
        {
            if !from_replay {
                let elapsed = crate::status_indicator_widget::fmt_elapsed_compact(
                    active.started_at.elapsed().as_secs(),
                );
                message = format!("上下文已压缩 · {elapsed}");
            }
            self.clear_context_compaction();
        }
        self.add_info_message(message, /*hint*/ None);
    }
}
