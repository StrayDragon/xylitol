#[cfg(feature = "highlight")]
mod syntect_bridge;

#[cfg(feature = "highlight")]
pub use syntect_bridge::{
    MAX_HIGHLIGHT_BYTES, MAX_HIGHLIGHT_LINES, highlight_code, highlight_code_owned,
};

#[cfg(not(feature = "highlight"))]
pub fn highlight_code(code: &str, _lang: Option<&str>) -> Vec<String> {
    code.lines().map(str::to_string).collect()
}

#[cfg(not(feature = "highlight"))]
pub fn highlight_code_owned(code: String, lang: Option<&str>) -> Vec<String> {
    highlight_code(&code, lang)
}
