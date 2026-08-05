use crate::ui::debug_ui;
use crate::ui::native_app::NativeApp;
use crate::ui::overlay_recommend_ui::PatternTabInfo;
use overmax_core::{Difficulty, GameSessionState};
use overmax_data::{RecommendContext, RecommendResult, RecordSource};

impl NativeApp {
    pub(crate) fn drain_detection_results(&mut self, ctx: &egui::Context) {
        let mut changed = false;
        while let Ok(output) = self.detection_rx.try_recv() {
            if let Ok(mut rate_ocr) = self.debug_state.rate_ocr.lock() {
                *rate_ocr = output.rate_telemetry.clone();
            }
            if let Ok(mut r) = self.game_rect.lock() {
                *r = output.game_rect;
            }
            self.window_snapshot = output.window_snapshot;
            self.capture_fatal = output.capture_fatal.clone();
            if !output.is_song_select {
                self.recorded_states.clear();
            }

            if output.state.scene.is_result() {
                if let Some(ctx_val) = &output.state.context {
                    if self.session_initial_record.is_none() {
                        let song_id = ctx_val.song_id;
                        let rate_map = self.record_manager.get_rate_map(&[song_id]);
                        if let Some(&(r, mc)) = rate_map.get(&(song_id, ctx_val.mode, ctx_val.diff))
                        {
                            self.session_initial_record = Some((r, mc));
                        } else {
                            self.session_initial_record = Some((0.0, false));
                        }
                    }
                }
            } else {
                self.session_initial_record = None;
            }

            self.confidence = output.confidence;
            if self.session != output.state {
                changed = true;
                self.session = output.state.clone();
            }

            if output.state.is_valid() {
                if let Some(ctx_val) = &output.state.context {
                    if ctx_val.rate >= overmax_engine::detector::play_state::MIN_VALID_RATE {
                        let key = (ctx_val.song_id, ctx_val.mode, ctx_val.diff);
                        let should_upsert =
                            if let Some(&(prev_rate, prev_mc)) = self.recorded_states.get(&key) {
                                ctx_val.rate > prev_rate || (ctx_val.is_max_combo && !prev_mc)
                            } else {
                                true
                            };

                        if should_upsert {
                            debug_ui::push_log(
                                &self.debug_state.log_lines,
                                self.max_log_lines(),
                                format!(
                                    "[Main] 기록 저장: {}, {}, {}, {:.2}%, MaxCombo: {}",
                                    key.0, key.1, key.2, ctx_val.rate, ctx_val.is_max_combo
                                ),
                            );
                            let is_result = output.is_result;
                            if self.record_manager.upsert(
                                key.0,
                                key.1,
                                key.2,
                                ctx_val.rate,
                                ctx_val.is_max_combo,
                                is_result,
                            ) {
                                self.recorded_states
                                    .insert(key, (ctx_val.rate, ctx_val.is_max_combo));
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        if changed {
            self.refresh_overlay_data();
            self.log_overlay_state();
            ctx.request_repaint();
        }
    }

    pub(crate) fn current_song_label(&self) -> String {
        let Some(ctx) = &self.session.context else {
            return "곡을 선택하세요".into();
        };
        let Some(song) = self.varchive_db.search_by_id(ctx.song_id) else {
            return format!("Song #{}", ctx.song_id);
        };
        song.name.to_string()
    }

    pub(crate) fn refresh_overlay_data(&mut self) {
        self.pattern_tabs = self.pattern_tabs_for_state(&self.session);
        self.recommendations = self.recommend_for_state(&self.session);
    }

    fn recommend_for_state(&self, state: &GameSessionState) -> RecommendResult {
        let Some(ctx) = &state.context else {
            return RecommendResult::empty();
        };
        let rec_ctx = RecommendContext {
            song_id: ctx.song_id,
            button_mode: ctx.mode,
            difficulty: ctx.diff,
            floor_range: 0.0,
            max_results: 6,
            same_mode_only: true,
            v_id: None,
        };
        self.recommender
            .recommend_panel(&rec_ctx)
            .as_legacy_result()
    }

    fn pattern_tabs_for_state(&self, state: &GameSessionState) -> Vec<PatternTabInfo> {
        let Some(ctx) = &state.context else {
            return Vec::new();
        };
        let Some(song) = self.varchive_db.search_by_id(ctx.song_id) else {
            return Vec::new();
        };
        let m = ctx.mode;
        let patterns = &song.patterns[m as usize];
        Difficulty::ALL
            .iter()
            .filter_map(|&d| {
                let pattern = patterns[d as usize].as_ref()?;
                let meta = self.sheet_meta.get(&song.title, m, d);
                Some(PatternTabInfo {
                    diff: d,
                    level: pattern.level,
                    floor_name: pattern.floor_name.as_ref().map(|s| s.to_string()),
                    gold: meta.gold.as_str().to_string(),
                    note: meta.note,
                    assist_key: meta.assist_key.as_str().to_string(),
                    keypart: meta.keypart,
                })
            })
            .collect()
    }

    fn log_overlay_state(&self) {
        debug_ui::push_log(
            &self.debug_state.log_lines,
            self.max_log_lines(),
            format!(
                "[UI] overlay state <- {} / recs={}",
                self.session,
                self.recommendations.entries.len()
            ),
        );
    }
}
