use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatter {
    fields: Vec<(String, String)>,
}

impl FrontMatter {
    pub fn new(fields: Vec<(String, String)>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field_key, _)| field_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        if let Some((_, current_value)) = self
            .fields
            .iter_mut()
            .find(|(field_key, _)| field_key == &key)
        {
            *current_value = value;
        } else {
            self.fields.push((key, value));
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        let index = self
            .fields
            .iter()
            .position(|(field_key, _)| field_key == key)?;
        Some(self.fields.remove(index).1)
    }

    pub fn merge_missing_from(&mut self, other: &FrontMatter) {
        for (key, value) in &other.fields {
            if self.get(key).is_none() {
                self.fields.push((key.clone(), value.clone()));
            }
        }
    }

    pub fn to_note_text(&self, body: &str) -> String {
        if self.fields.is_empty() {
            return body.to_string();
        }

        let mut output = self.to_string();
        if !body.starts_with('\n') && !body.is_empty() {
            output.push('\n');
        }
        output.push_str(body);
        output
    }

    pub fn to_map(&self) -> BTreeMap<String, FrontMatterValue> {
        self.fields
            .iter()
            .map(|(key, raw)| (key.clone(), parse_value(raw)))
            .collect()
    }
}

impl std::fmt::Display for FrontMatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fields.is_empty() {
            return Ok(());
        }

        writeln!(f, "---")?;
        for (key, value) in &self.fields {
            writeln!(f, "{key}: {}", quote_if_needed(value))?;
        }
        write!(f, "---")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontMatterValue {
    String(String),
    Bool(bool),
    Integer(i64),
    Array(Vec<String>),
}

pub fn parse_front_matter(text: &str) -> (Option<FrontMatter>, String) {
    let mut lines = text.lines();
    let Some(first_line) = lines.next() else {
        return (None, String::new());
    };

    if first_line.trim() != "---" {
        return (None, text.to_string());
    }

    let mut fields = Vec::new();
    let mut body_start = None;
    let all_lines = text.lines().collect::<Vec<_>>();

    for (index, line) in all_lines.iter().enumerate().skip(1) {
        let trimmed = line.trim();
        if trimmed == "---" {
            body_start = Some(index + 1);
            break;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        fields.push((key.to_string(), unquote(value.trim())));
    }

    let Some(body_start) = body_start else {
        return (None, text.to_string());
    };

    let body = all_lines[body_start..].join("\n");
    (Some(FrontMatter::new(fields)), body)
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }

    value.to_string()
}

fn quote_if_needed(value: &str) -> String {
    let needs_quotes = value.is_empty()
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.chars().any(|ch| {
            matches!(
                ch,
                ':' | '#'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | ','
                    | '&'
                    | '*'
                    | '!'
                    | '|'
                    | '>'
                    | '\''
                    | '"'
                    | '%'
                    | '@'
            )
        });

    if needs_quotes {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn parse_value(raw: &str) -> FrontMatterValue {
    if raw == "true" {
        return FrontMatterValue::Bool(true);
    }
    if raw == "false" {
        return FrontMatterValue::Bool(false);
    }
    if let Ok(value) = raw.parse::<i64>() {
        return FrontMatterValue::Integer(value);
    }
    if raw.starts_with('[') && raw.ends_with(']') {
        let items = raw[1..raw.len() - 1]
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(unquote)
            .collect::<Vec<_>>();
        return FrontMatterValue::Array(items);
    }

    FrontMatterValue::String(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::{FrontMatter, FrontMatterValue, parse_front_matter};

    #[test]
    fn returns_original_body_when_front_matter_missing() {
        let text = "# Title\n\nBody";
        let (front_matter, body) = parse_front_matter(text);

        assert!(front_matter.is_none());
        assert_eq!(body, text);
    }

    #[test]
    fn parses_simple_front_matter() {
        let text = "---\ntitle: Test\ndraft: true\n---\n# Title\n";
        let (front_matter, body) = parse_front_matter(text);
        let front_matter = front_matter.expect("front matter should parse");

        assert_eq!(front_matter.get("title"), Some("Test"));
        assert_eq!(front_matter.get("draft"), Some("true"));
        assert_eq!(body, "# Title");
    }

    #[test]
    fn parses_quoted_strings_and_arrays() {
        let text = "---\ntitle: \"Hello: world\"\ntags: [\"rust\", bear]\n---\nBody";
        let (front_matter, body) = parse_front_matter(text);
        let front_matter = front_matter.expect("front matter should parse");

        assert_eq!(front_matter.get("title"), Some("Hello: world"));
        assert_eq!(body, "Body");

        let map = front_matter.to_map();
        assert_eq!(
            map.get("tags"),
            Some(&FrontMatterValue::Array(vec![
                "rust".to_string(),
                "bear".to_string()
            ]))
        );
    }

    #[test]
    fn ignores_unclosed_front_matter_block() {
        let text = "---\ntitle: Test\nbody";
        let (front_matter, body) = parse_front_matter(text);

        assert!(front_matter.is_none());
        assert_eq!(body, text);
    }

    #[test]
    fn serializes_front_matter_back_to_note_text() {
        let mut front_matter = FrontMatter::new(vec![
            ("title".to_string(), "Test".to_string()),
            ("tags".to_string(), "[rust, bear]".to_string()),
        ]);
        front_matter.set("draft", "false");
        let text = front_matter.to_note_text("# Title\n\nBody");

        assert_eq!(
            text,
            "---\ntitle: Test\ntags: \"[rust, bear]\"\ndraft: false\n---\n# Title\n\nBody"
        );
    }

    #[test]
    fn preserves_field_order_when_updating() {
        let mut front_matter = FrontMatter::new(vec![
            ("title".to_string(), "One".to_string()),
            ("draft".to_string(), "true".to_string()),
        ]);

        front_matter.set("title", "Two");
        front_matter.set("tags", "bear");

        assert_eq!(
            front_matter.fields(),
            &[
                ("title".to_string(), "Two".to_string()),
                ("draft".to_string(), "true".to_string()),
                ("tags".to_string(), "bear".to_string()),
            ]
        );
    }
}
