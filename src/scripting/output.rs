use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone)]
pub struct OutputEntry {
    pub id: u64,
    pub run_id: u64,
    pub time: f64,
    pub level: OutputLevel,
    pub source: String,
    pub line: Option<u32>,
    pub message: String,
    pub traceback: Option<String>,
}

#[derive(Clone)]
pub struct RunInfo {
    pub id: u64,
    pub label: String,
    pub started_at: f64,
    pub ended_at: Option<f64>,
    pub script_count: usize,
}

pub struct OutputBuffer {
    pub entries: VecDeque<OutputEntry>,
    pub runs: Vec<RunInfo>,
    pub current_run: u64,
    pub script_starts: u64,
    pub max_entries: usize,
    created: Instant,
    next_entry_id: u64,
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            runs: vec![RunInfo {
                id: 0,
                label: "Studio".to_string(),
                started_at: 0.0,
                ended_at: None,
                script_count: 0,
            }],
            current_run: 0,
            script_starts: 0,
            max_entries: 2000,
            created: Instant::now(),
            next_entry_id: 1,
        }
    }

    fn now_secs(&self) -> f64 {
        self.created.elapsed().as_secs_f64()
    }

    pub fn start_run(&mut self, label: &str) -> u64 {
        let id = self.runs.len() as u64;
        self.runs.push(RunInfo {
            id,
            label: label.to_string(),
            started_at: self.now_secs(),
            ended_at: None,
            script_count: 0,
        });
        self.current_run = id;
        id
    }

    pub fn end_run(&mut self) {
        let ended_at = self.now_secs();
        if let Some(run) = self.runs.iter_mut().find(|r| r.id == self.current_run) {
            run.ended_at = Some(ended_at);
        }
    }

    pub fn push(&mut self, level: OutputLevel, source: &str, line: Option<u32>, message: String, traceback: Option<String>) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(OutputEntry {
            id: self.next_entry_id,
            run_id: self.current_run,
            time: self.now_secs(),
            level,
            source: source.to_string(),
            line,
            message,
            traceback,
        });
        self.next_entry_id += 1;
    }

    pub fn record_script_start(&mut self) {
        self.script_starts += 1;
        if let Some(run) = self.runs.iter_mut().find(|r| r.id == self.current_run) {
            run.script_count += 1;
        }
    }

    pub fn clear_entries(&mut self) {
        self.entries.clear();
    }
}

pub fn buffer() -> &'static Arc<Mutex<OutputBuffer>> {
    static BUFFER: OnceLock<Arc<Mutex<OutputBuffer>>> = OnceLock::new();
    BUFFER.get_or_init(|| Arc::new(Mutex::new(OutputBuffer::new())))
}

pub fn start_run(label: &str) -> u64 {
    buffer().lock().unwrap().start_run(label)
}

pub fn end_run() {
    buffer().lock().unwrap().end_run();
}

pub fn push(level: OutputLevel, source: &str, line: Option<u32>, message: String, traceback: Option<String>) {
    buffer().lock().unwrap().push(level, source, line, message, traceback);
}

pub fn push_error(source: &str, detail: String) {
    let (message, traceback) = split_error(&detail);
    push(OutputLevel::Error, source, None, message, traceback);
}

pub fn record_script_start() {
    buffer().lock().unwrap().record_script_start();
}

pub fn clear_entries() {
    buffer().lock().unwrap().clear_entries();
}

pub fn split_error(detail: &str) -> (String, Option<String>) {
    if let Some(pos) = detail.find("stack traceback:") {
        let message = detail[..pos].trim_end().to_string();
        let traceback = detail[pos..].trim().to_string();
        (message, Some(traceback))
    } else {
        (detail.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_error_separates_message_and_traceback() {
        let detail = "runtime error: boom\nstack traceback:\n\t[C]: in function 'error'";
        let (message, traceback) = split_error(detail);
        assert_eq!(message, "runtime error: boom");
        assert!(traceback.unwrap().starts_with("stack traceback:"));
    }

    #[test]
    fn split_error_without_traceback_keeps_full_message() {
        let (message, traceback) = split_error("just a message");
        assert_eq!(message, "just a message");
        assert!(traceback.is_none());
    }

    #[test]
    fn runs_are_numbered_and_entries_tagged_with_run() {
        let mut buf = OutputBuffer::new();
        let run_id = buf.start_run("Playtest");
        buf.push(OutputLevel::Info, "S", None, "first".to_string(), None);
        buf.push(OutputLevel::Error, "S", None, "second".to_string(), Some("trace".to_string()));
        buf.end_run();

        assert_eq!(run_id, 1);
        assert_eq!(buf.runs.len(), 2);
        assert!(buf.runs[1].ended_at.is_some());
        assert!(buf.entries.iter().all(|e| e.run_id == run_id));
        assert_eq!(buf.entries.len(), 2);
        assert_eq!(buf.entries[0].id, 1);
        assert_eq!(buf.entries[1].id, 2);
    }

    #[test]
    fn ring_buffer_caps_entry_count() {
        let mut buf = OutputBuffer::new();
        buf.max_entries = 5;
        for i in 0..10 {
            buf.push(OutputLevel::Info, "S", None, i.to_string(), None);
        }
        assert_eq!(buf.entries.len(), 5);
        assert_eq!(buf.entries.front().unwrap().message, "5");
    }

    #[test]
    fn script_starts_tracked_per_run() {
        let mut buf = OutputBuffer::new();
        buf.record_script_start();
        buf.record_script_start();
        assert_eq!(buf.runs[0].script_count, 2);
        buf.start_run("Playtest");
        buf.record_script_start();
        assert_eq!(buf.runs[1].script_count, 1);
        assert_eq!(buf.script_starts, 3);
    }

    #[test]
    fn clear_entries_keeps_runs() {
        let mut buf = OutputBuffer::new();
        buf.push(OutputLevel::Info, "S", None, "x".to_string(), None);
        buf.clear_entries();
        assert!(buf.entries.is_empty());
        assert_eq!(buf.runs.len(), 1);
    }
}
