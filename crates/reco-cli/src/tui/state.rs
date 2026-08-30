use reco_core::Recommendation;

#[derive(Debug, Clone)]
pub struct AiTui {
    recs: Vec<Recommendation>,
    filtered: Vec<usize>,
    selected: usize,
    query: String,
    pub searching: bool,
    pub show_help: bool,
    pub downloaded_only: bool,
    downloaded: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiAction {
    None,
    Confirm,
    Quit,
}

impl AiTui {
    pub fn new(recs: Vec<Recommendation>, downloaded: std::collections::HashSet<String>) -> Self {
        let len = recs.len();
        Self {
            recs,
            filtered: (0..len).collect(),
            selected: 0,
            query: String::new(),
            searching: false,
            show_help: false,
            downloaded_only: false,
            downloaded,
        }
    }

    pub fn is_downloaded(&self, rec: &Recommendation) -> bool {
        self.downloaded
            .contains(&format!("{}:{}", rec.repo_id, rec.filename))
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn visible(&self) -> impl Iterator<Item = (usize, &Recommendation)> {
        self.filtered
            .iter()
            .enumerate()
            .filter_map(|(pos, idx)| self.recs.get(*idx).map(|rec| (pos, rec)))
    }

    pub fn current(&self) -> Option<&Recommendation> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.recs.get(*idx))
    }

    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn down(&mut self) {
        let max = self.filtered.len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
        }
    }

    pub fn handle_char(&mut self, ch: char) -> TuiAction {
        if self.searching {
            if ch == '\n' {
                self.searching = false;
            } else {
                self.query.push(ch);
                self.apply_filter();
            }
            return TuiAction::None;
        }
        match ch {
            'q' | 'Q' => TuiAction::Quit,
            '?' => {
                self.show_help = !self.show_help;
                TuiAction::None
            }
            'd' | 'D' => {
                self.downloaded_only = !self.downloaded_only;
                self.apply_filter();
                TuiAction::None
            }
            '/' => {
                self.searching = true;
                TuiAction::None
            }
            'j' => {
                self.down();
                TuiAction::None
            }
            'k' => {
                self.up();
                TuiAction::None
            }
            '\n' => TuiAction::Confirm,
            _ => TuiAction::None,
        }
    }

    pub fn backspace(&mut self) {
        if self.searching {
            self.query.pop();
            self.apply_filter();
        }
    }

    pub fn cancel_search(&mut self) {
        if self.searching {
            self.searching = false;
            if !self.query.is_empty() {
                self.query.clear();
                self.apply_filter();
            }
        }
    }

    fn apply_filter(&mut self) {
        let q = self.query.to_ascii_lowercase();
        self.filtered = self
            .recs
            .iter()
            .enumerate()
            .filter(|(_, rec)| {
                if self.downloaded_only && !self.is_downloaded(rec) {
                    return false;
                }
                if q.is_empty() {
                    return true;
                }
                rec.repo_id.to_ascii_lowercase().contains(&q)
                    || rec.filename.to_ascii_lowercase().contains(&q)
                    || rec.quant.label().to_ascii_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reco_core::{GgufQuant, ModelParams, Recommendation, Scores};

    fn rec(repo: &str, file: &str) -> Recommendation {
        Recommendation {
            repo_id: repo.into(),
            filename: file.into(),
            quant: GgufQuant::Q4Km,
            size_bytes: 4_000_000_000,
            size_estimated: true,
            params: Some(ModelParams {
                total_billions: 7.0,
                active_billions: None,
            }),
            downloads: 1,
            scores: Scores {
                compatibility: 90.0,
                speed: 80.0,
                quality: 70.0,
                popularity: 60.0,
            },
            total: 80.0,
            why: "test".into(),
        }
    }

    #[test]
    fn navigate_filter_and_confirm() {
        let mut tui = AiTui::new(
            vec![
                rec("Qwen/Qwen2.5-7B-Instruct-GGUF", "q4.gguf"),
                rec("bartowski/Llama-3.1-8B-Instruct-GGUF", "q4.gguf"),
                rec("unsloth/Llama-3.2-3B-Instruct-GGUF", "q4.gguf"),
            ],
            std::collections::HashSet::new(),
        );
        tui.down();
        assert!(tui.current().unwrap().repo_id.contains("Llama-3.1"));
        tui.handle_char('/');
        for ch in "qwen".chars() {
            tui.handle_char(ch);
        }
        assert_eq!(tui.visible().count(), 1);
        assert!(tui.current().unwrap().repo_id.contains("Qwen"));
        assert_eq!(tui.handle_char('\n'), TuiAction::None); // end search
        assert_eq!(tui.handle_char('\n'), TuiAction::Confirm);
        tui.handle_char('/');
        tui.cancel_search();
        assert_eq!(tui.visible().count(), 3);
        assert_eq!(tui.handle_char('q'), TuiAction::Quit);
    }
}
