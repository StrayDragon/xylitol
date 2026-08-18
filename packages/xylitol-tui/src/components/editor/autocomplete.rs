use super::types::AutocompleteMode;
use crate::autocomplete::{
    AutocompleteItem, AutocompleteSuggestions, CombinedAutocompleteProvider,
};
use crate::completion::{AtPathSource, CompletionContext, CompletionSource, SlashCommandSource};
use crate::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};

impl super::Editor {
    // ── c430: autocomplete via CompletionSource registry ───────────────

    pub(super) fn completion_ctx(&self) -> CompletionContext<'_> {
        CompletionContext {
            lines: &self.state.lines,
            cursor_line: self.state.cursor_line,
            cursor_col: self.state.cursor_col,
        }
    }

    /// Primary API: register pluggable completion sources (`/`, `@`, future `$`/`^`).
    pub fn set_completion_sources(&mut self, sources: Vec<Box<dyn CompletionSource>>) {
        self.cancel_autocomplete();
        self.completion.set_sources(sources);
    }

    /// Compatibility shim: split Combined into Slash + AtPath sources.
    pub fn set_autocomplete_provider(&mut self, provider: Option<CombinedAutocompleteProvider>) {
        self.cancel_autocomplete();
        match provider {
            None => self.completion.clear(),
            Some(p) => {
                let (commands, base, fd) = p.into_parts();
                let at: Box<dyn CompletionSource> = match fd {
                    Some(fd_path) => Box::new(AtPathSource::new_with_fd(base, fd_path)),
                    None => Box::new(AtPathSource::new(base)),
                };
                self.completion
                    .set_sources(vec![Box::new(SlashCommandSource::new(commands)), at]);
            }
        }
    }

    /// Whether the autocomplete popup is currently open (host Enter routing).
    pub fn is_showing_autocomplete(&self) -> bool {
        self.autocomplete_state.is_some() && self.autocomplete_list.is_some()
    }

    /// After text/cursor edits: dismiss, refresh, or open a matching source.
    pub(super) fn handle_autocomplete_on_edit(&mut self) {
        if self.completion.is_empty() {
            return;
        }
        if self.is_showing_autocomplete() {
            let ctx = self.completion_ctx();
            if self.completion.should_dismiss_active(&ctx) {
                let prev = self.completion.active_index();
                self.cancel_autocomplete();
                // Lone `/` stays closed; `/model␠` must hand off to model-id.
                if let Some((i, _)) = self.completion.probe_first(&self.completion_ctx())
                    && prev != Some(i)
                {
                    self.request_autocomplete(false, false);
                }
                return;
            }
            self.request_autocomplete(false, false);
        } else if self
            .completion
            .probe_first(&self.completion_ctx())
            .is_some()
        {
            self.request_autocomplete(false, false);
        }
    }

    pub(super) fn handle_tab_completion(&mut self) {
        if self.completion.is_empty() {
            return;
        }
        // Force=true for non-slash probes (file path Tab); slash stays regular.
        let ctx = self.completion_ctx();
        let force = match self.completion.probe_first(&ctx) {
            Some((_, m)) => !m.prefix.starts_with('/'),
            None => true,
        };
        self.request_autocomplete(force, true);
    }

    pub(super) fn request_autocomplete(&mut self, force: bool, explicit_tab: bool) {
        if self.completion.is_empty() {
            return;
        }
        self.autocomplete_start_token = self.autocomplete_start_token.wrapping_add(1);
        let start_token = self.autocomplete_start_token;
        self.start_autocomplete_request(start_token, force, explicit_tab);
    }

    pub(super) fn start_autocomplete_request(
        &mut self,
        start_token: usize,
        force: bool,
        explicit_tab: bool,
    ) {
        if start_token != self.autocomplete_start_token {
            return;
        }

        let ctx = CompletionContext {
            lines: &self.state.lines,
            cursor_line: self.state.cursor_line,
            cursor_col: self.state.cursor_col,
        };
        let suggestions = self.completion.open_or_refresh(&ctx);

        if start_token != self.autocomplete_start_token {
            return;
        }

        match suggestions {
            Some(s) if !s.items.is_empty() => {
                if force && explicit_tab && s.items.len() == 1 {
                    let item = s.items[0].clone();
                    let prefix = s.prefix.clone();
                    let source_index = self.completion.active_index().unwrap_or(0);
                    self.push_undo();
                    self.last_action = None;
                    let (new_lines, nl, nc) = self.completion.apply_probed(
                        source_index,
                        &self.state.lines,
                        self.state.cursor_line,
                        self.state.cursor_col,
                        &item,
                        &prefix,
                    );
                    self.state.lines = new_lines;
                    self.state.cursor_line = nl;
                    self.set_cursor_col(nc);
                    self.cancel_autocomplete();
                    self.on_changed();
                    // Chain: `/model` slash apply → `/model ` → open arg catalog.
                    self.handle_autocomplete_on_edit();
                } else {
                    self.apply_autocomplete_suggestions(
                        s,
                        if force {
                            AutocompleteMode::Force
                        } else {
                            AutocompleteMode::Regular
                        },
                    );
                }
            }
            _ => {
                self.cancel_autocomplete();
            }
        }
    }

    pub(super) fn apply_autocomplete_suggestions(
        &mut self,
        suggestions: AutocompleteSuggestions,
        mode: AutocompleteMode,
    ) {
        self.autocomplete_prefix = suggestions.prefix.clone();
        let items: Vec<SelectItem> = suggestions
            .items
            .into_iter()
            .map(|i| SelectItem {
                value: i.value,
                label: i.label,
                description: i.description,
            })
            .collect();

        let best_idx = self.get_best_autocomplete_match_index(&items, &self.autocomplete_prefix);
        let layout = if self.autocomplete_prefix.starts_with('/') {
            SelectListLayoutOptions {
                min_primary_column_width: Some(12),
                max_primary_column_width: Some(32),
                truncate_primary: None,
            }
        } else {
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            }
        };
        let mut sl = SelectList::new(
            items,
            self.autocomplete_max_visible,
            SelectListTheme::default(),
            layout,
        );
        if best_idx < sl.filtered_items.len() {
            sl.set_selected_index(best_idx);
        }
        self.autocomplete_list = Some(sl);
        self.autocomplete_state = Some(mode);
    }

    pub(super) fn get_best_autocomplete_match_index(
        &self,
        items: &[SelectItem],
        prefix: &str,
    ) -> usize {
        if prefix.is_empty() {
            return 0;
        }
        // Strip leading trigger for value compare (slash items store bare names).
        let needle = prefix
            .strip_prefix('/')
            .or_else(|| prefix.strip_prefix('@'))
            .or_else(|| prefix.strip_prefix('$'))
            .unwrap_or(prefix);
        let mut first_prefix = items.len();
        for (i, item) in items.iter().enumerate() {
            if item.value == needle || item.value == prefix {
                return i;
            }
            if first_prefix == items.len()
                && (item.value.starts_with(needle) || item.value.starts_with(prefix))
            {
                first_prefix = i;
            }
        }
        if first_prefix < items.len() {
            first_prefix
        } else {
            0
        }
    }

    pub(super) fn cancel_autocomplete_request(&mut self) {
        self.autocomplete_start_token = self.autocomplete_start_token.wrapping_add(1);
    }

    pub(super) fn clear_autocomplete_ui(&mut self) {
        self.autocomplete_state = None;
        self.autocomplete_list = None;
        self.autocomplete_prefix.clear();
        self.completion.clear_active();
    }

    pub(super) fn cancel_autocomplete(&mut self) {
        self.cancel_autocomplete_request();
        self.clear_autocomplete_ui();
    }

    /// Apply the highlighted autocomplete item into the editor buffer only
    /// (no submit). Used by product host Idle-Enter before slash parse.
    pub fn confirm_autocomplete_selection(&mut self) -> bool {
        self.apply_selected_autocomplete(/* chain_next */ false)
    }

    /// Apply the highlighted autocomplete item into the editor.
    ///
    /// Returns `true` when text was updated. When `chain_next` is true (Tab),
    /// re-probe sources after apply (e.g. `/model ` → model ids). Enter on
    /// slash uses `chain_next=false` then submits (pi `tui.select.confirm`).
    pub(super) fn apply_selected_autocomplete(&mut self, chain_next: bool) -> bool {
        let apply_data = if let Some(ref list) = self.autocomplete_list {
            list.get_selected_item().map(|i| {
                (
                    i.value.clone(),
                    i.label.clone(),
                    i.description.clone(),
                    self.autocomplete_prefix.clone(),
                )
            })
        } else {
            None
        };
        let Some((val, lbl, desc, prefix)) = apply_data else {
            self.cancel_autocomplete();
            return false;
        };
        let ai = AutocompleteItem {
            value: val,
            label: lbl,
            description: desc,
        };
        if let Some((new_lines, nl, nc)) = self.completion.apply_active(
            &self.state.lines,
            self.state.cursor_line,
            self.state.cursor_col,
            &ai,
            &prefix,
        ) {
            self.push_undo();
            self.last_action = None;
            self.state.lines = new_lines;
            self.state.cursor_line = nl;
            self.set_cursor_col(nc);
            self.cancel_autocomplete();
            self.on_changed();
            if chain_next {
                self.handle_autocomplete_on_edit();
            }
            true
        } else {
            self.cancel_autocomplete();
            false
        }
    }
}
