use regex::Regex;
use std::sync::OnceLock;

static URL_RE: OnceLock<Regex> = OnceLock::new();
static EMAIL_RE: OnceLock<Regex> = OnceLock::new();
static COLOR_RE: OnceLock<Regex> = OnceLock::new();

fn url_re() -> &'static Regex {
    URL_RE.get_or_init(|| Regex::new(r"^https?://\S+").unwrap())
}

fn email_re() -> &'static Regex {
    EMAIL_RE.get_or_init(|| Regex::new(r"^[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}$").unwrap())
}

fn color_re() -> &'static Regex {
    COLOR_RE.get_or_init(|| Regex::new(r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$").unwrap())
}

pub fn detect_content_type(content: &str) -> &'static str {
    let trimmed = content.trim();
    if url_re().is_match(trimmed) {
        return "url";
    }
    if email_re().is_match(trimmed) {
        return "email";
    }
    if color_re().is_match(trimmed) {
        return "color";
    }
    // Heuristic for code: has braces/semicolons/parens on multiple lines
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() > 1 {
        let code_chars = trimmed.chars().filter(|c| matches!(c, '{' | '}' | ';' | '(' | ')')).count();
        if code_chars > 3 {
            return "code";
        }
    }
    "text"
}

pub fn make_preview(content: &str, max_chars: usize) -> String {
    let trimmed = content.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let truncated: String = trimmed.chars().take(max_chars).collect();
    format!("{}…", truncated)
}
