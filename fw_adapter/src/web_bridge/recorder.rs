use regex::Regex;
use std::sync::OnceLock;

pub trait ErrLogRecorder {
    fn log_record(&self, max_frames: isize);
}

static OL_PROJECT_NAME: OnceLock<String> = OnceLock::new();
static OL_FRAME_EXTRACT_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn init_project_name(name: impl Into<String>) {
    let name = name.into();
    let _ = OL_PROJECT_NAME.set(name.clone());

    let pattern = format!(r"(?m)^\s*\d+: {}.*$", regex::escape(&name));

    let _ = OL_FRAME_EXTRACT_REGEX.set(Regex::new(&pattern).unwrap());
}

fn _get_project_name() -> &'static str {
    OL_PROJECT_NAME.get().map(|s| s.as_str()).unwrap_or("")
}

fn _get_extract_regex() -> Option<&'static Regex> {
    OL_FRAME_EXTRACT_REGEX.get()
}

// 规避日志噪音
impl ErrLogRecorder for anyhow::Error {
    fn log_record(&self, max_frames: isize) {
        let err_msg = format!("{:#}", self);

        if max_frames == 0 {
            tracing::error!(%err_msg);
            return;
        }

        let full_stacks = format!("{:?}", self);

        let Some(re_frame) = _get_extract_regex() else {
            let err_msg = format!("{:#}", self);
            tracing::error!(err_msg);
            return;
        };

        let mut frames = Vec::new();
        let lines: Vec<&str> = full_stacks.lines().collect();
        let mut captured_count = 0;
        for i in 0..lines.len() {
            if re_frame.is_match(lines[i]) {
                frames.push(lines[i].trim_end());
                // 捕获紧随其后的地址行 "at ..."
                if i + 1 < lines.len() && lines[i + 1].trim_start().starts_with("at ") {
                    frames.push(lines[i + 1].trim_end());
                }

                captured_count += 1;

                if max_frames > 0 && captured_count >= (max_frames as usize) {
                    break;
                }
            }
        }

        if frames.is_empty() {
            tracing::error!(err_msg);
        } else {
            let err_stack = frames.join("\n");
            tracing::error!("err_msg={}, err_stack:\n{}", err_msg, err_stack);
        }
    }
}
