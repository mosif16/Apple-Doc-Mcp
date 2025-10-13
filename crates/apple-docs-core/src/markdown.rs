pub fn header(level: usize, text: &str) -> String {
    let level = level.max(1);
    format!("{} {}", "#".repeat(level), text)
}

pub fn bold(label: &str, value: &str) -> String {
    format!("**{}:** {}", label, value)
}

pub fn blank_line() -> String {
    String::new()
}

pub fn paragraph(text: &str) -> String {
    text.to_string()
}
