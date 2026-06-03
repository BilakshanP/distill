use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnswerMode {
    Always,
    Never,
    OptIn,
    OptOut,
}

impl AnswerMode {
    pub fn from_config(config: &HashMap<String, String>) -> Self {
        match config.get("answer_mode").map(|s| s.as_str()) {
            Some("always") => Self::Always,
            Some("never") => Self::Never,
            Some("opt-out") => Self::OptOut,
            _ => Self::OptIn,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    Hybrid,
    KeywordOnly,
}

impl SearchMode {
    pub fn from_config(config: &HashMap<String, String>) -> Self {
        match config.get("search_mode").map(|s| s.as_str()) {
            Some("keyword-only") => Self::KeywordOnly,
            _ => Self::Hybrid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RaterContextVisibility {
    Never,
    Always,
    Optional,
}

impl RaterContextVisibility {
    pub fn from_config(config: &HashMap<String, String>) -> Self {
        match config.get("rater_context_visibility").map(|s| s.as_str()) {
            Some("never") => Self::Never,
            Some("always") => Self::Always,
            _ => Self::Optional,
        }
    }
}
