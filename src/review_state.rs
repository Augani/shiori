use adabraka_ui::components::input::{InputEvent, InputState};
use gpui::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const REVIEW_DIR: &str = ".shiori";
const REVIEW_FILE: &str = "review.json";
const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentSide {
    Old,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    Open,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: u64,
    pub file: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line_end: Option<u32>,
    pub side: CommentSide,
    pub body: String,
    pub context: String,
    pub status: CommentStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReviewFile {
    version: u32,
    next_id: u64,
    comments: Vec<ReviewComment>,
}

impl Default for ReviewFile {
    fn default() -> Self {
        Self {
            version: FILE_VERSION,
            next_id: 1,
            comments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommentDraft {
    pub file: String,
    pub line_start: u32,
    pub line_end: u32,
    pub side: CommentSide,
    pub context: String,
    pub body: String,
    pub row_index: usize,
}

pub struct ReviewState {
    workspace_root: Option<PathBuf>,
    data: ReviewFile,
    pub active_draft: Option<CommentDraft>,
    pub draft_input: Option<Entity<InputState>>,
}

impl ReviewState {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            workspace_root: None,
            data: ReviewFile::default(),
            active_draft: None,
            draft_input: None,
        }
    }

    pub fn set_workspace(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        self.workspace_root = Some(root);
        self.load();
        cx.notify();
    }

    fn review_path(&self) -> Option<PathBuf> {
        self.workspace_root
            .as_ref()
            .map(|r| r.join(REVIEW_DIR).join(REVIEW_FILE))
    }

    fn load(&mut self) {
        let path = match self.review_path() {
            Some(p) => p,
            None => return,
        };
        if !path.exists() {
            self.data = ReviewFile::default();
            return;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<ReviewFile>(&content) {
                Ok(file) => self.data = file,
                Err(e) => {
                    eprintln!("shiori: failed to parse review file: {e}");
                    self.data = ReviewFile::default();
                }
            },
            Err(e) => {
                eprintln!("shiori: failed to read review file: {e}");
                self.data = ReviewFile::default();
            }
        }
    }

    fn save(&self) {
        let path = match self.review_path() {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("shiori: failed to create review dir: {e}");
                return;
            }
        }
        match serde_json::to_string_pretty(&self.data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    eprintln!("shiori: failed to write review file: {e}");
                }
            }
            Err(e) => eprintln!("shiori: failed to serialize review data: {e}"),
        }
    }

    pub fn add_comment(
        &mut self,
        file: String,
        line: u32,
        line_end: Option<u32>,
        side: CommentSide,
        body: String,
        context: String,
        cx: &mut Context<Self>,
    ) {
        let id = self.data.next_id;
        self.data.next_id += 1;
        let now = chrono_now();
        self.data.comments.push(ReviewComment {
            id,
            file,
            line,
            line_end,
            side,
            body,
            context,
            status: CommentStatus::Open,
            created_at: now,
        });
        self.save();
        cx.notify();
    }

    pub fn resolve_comment(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(comment) = self.data.comments.iter_mut().find(|c| c.id == id) {
            comment.status = CommentStatus::Resolved;
            self.save();
            cx.notify();
        }
    }

    pub fn reopen_comment(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(comment) = self.data.comments.iter_mut().find(|c| c.id == id) {
            comment.status = CommentStatus::Open;
            self.save();
            cx.notify();
        }
    }

    pub fn remove_comment(&mut self, id: u64, cx: &mut Context<Self>) {
        self.data.comments.retain(|c| c.id != id);
        self.save();
        cx.notify();
    }

    pub fn clear_resolved(&mut self, cx: &mut Context<Self>) {
        self.data
            .comments
            .retain(|c| c.status != CommentStatus::Resolved);
        self.save();
        cx.notify();
    }

    pub fn start_draft(
        &mut self,
        file: String,
        line: u32,
        side: CommentSide,
        context: String,
        row_index: usize,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(InputState::new);
        cx.subscribe(&input, |this, input_entity, event, cx| {
            if matches!(event, InputEvent::Change) {
                let content = input_entity.read(cx).content.clone();
                if let Some(draft) = &mut this.active_draft {
                    draft.body = content.to_string();
                }
            }
        })
        .detach();
        self.draft_input = Some(input);
        self.active_draft = Some(CommentDraft {
            file,
            line_start: line,
            line_end: line,
            side,
            context,
            body: String::new(),
            row_index,
        });
        cx.notify();
    }

    pub fn extend_draft_range(&mut self, end_line: u32, cx: &mut Context<Self>) {
        if let Some(draft) = &mut self.active_draft {
            let orig = draft.line_start;
            draft.line_start = orig.min(end_line);
            draft.line_end = orig.max(end_line);
            cx.notify();
        }
    }

    pub fn cancel_draft(&mut self, cx: &mut Context<Self>) {
        self.active_draft = None;
        self.draft_input = None;
        cx.notify();
    }

    pub fn submit_draft(&mut self, cx: &mut Context<Self>) {
        let draft = match self.active_draft.take() {
            Some(d) => d,
            None => return,
        };
        self.draft_input = None;
        if draft.body.trim().is_empty() {
            cx.notify();
            return;
        }
        let line_end = if draft.line_end != draft.line_start {
            Some(draft.line_end)
        } else {
            None
        };
        self.add_comment(
            draft.file,
            draft.line_start,
            line_end,
            draft.side,
            draft.body,
            draft.context,
            cx,
        );
    }

    pub fn comments_for_file(&self, file: &str) -> Vec<&ReviewComment> {
        self.data
            .comments
            .iter()
            .filter(|c| c.file == file)
            .collect()
    }

    pub fn comments_by_file(&self) -> HashMap<String, Vec<&ReviewComment>> {
        let mut map: HashMap<String, Vec<&ReviewComment>> = HashMap::new();
        for comment in &self.data.comments {
            map.entry(comment.file.clone()).or_default().push(comment);
        }
        map
    }
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let (year, month, day) = days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
