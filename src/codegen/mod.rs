mod java;
mod rust;

pub use java::java;
pub use rust::rust;

struct CaseConverter {
    counter: usize,
}

impl CaseConverter {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn pascal_case(&mut self, text: &str) -> String {
        let clean_text: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        let words: Vec<String> = clean_text
            .split(|c: char| c == '_' || c.is_whitespace())
            .map(|word| {
                let mut chars = word.chars();
                let first_char = chars.next().unwrap_or_default().to_ascii_uppercase();
                let rest: String = chars.collect();
                format!("{}{}", first_char, rest)
            })
            .collect();

        let result = words.concat();
        match result.is_empty() {
            true => self.unknown_pascal_case(),
            false => result,
        }
    }

    fn camel_case(&mut self, text: &str) -> String {
        let clean_text: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        let mut words: Vec<String> = clean_text
            .split(|c: char| c == '_' || c.is_whitespace())
            .map(|word| {
                let mut chars = word.chars();
                let first_char = chars.next().unwrap_or_default().to_ascii_uppercase();
                let rest: String = chars.collect();
                format!("{}{}", first_char, rest)
            })
            .collect();

        if let Some(first_word) = words.iter_mut().next() {
            let mut chars = first_word.chars();
            let first_char = chars.next().unwrap_or_default().to_ascii_lowercase();
            let rest: String = chars.collect();
            *first_word = format!("{}{}", first_char, rest);
        }

        let result = words.concat();
        match result.is_empty() {
            true => self.unknown_camel_case(),
            false => result,
        }
    }

    fn snake_case(&mut self, text: &str) -> String {
        let clean_text: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .map(|c| c.to_ascii_lowercase())
            .collect();

        let words: Vec<String> = clean_text
            .split(|c: char| c.is_whitespace())
            .map(|s| s.into())
            .collect();

        let result = words.join("_");
        match result.is_empty() {
            true => self.unknown_snake_case(),
            false => result,
        }
    }

    fn unknown_pascal_case(&mut self) -> String {
        let text = format!("Unknown{}", self.counter);
        self.counter += 1;
        text
    }

    fn unknown_camel_case(&mut self) -> String {
        let text = format!("unknown{}", self.counter);
        self.counter += 1;
        text
    }

    fn unknown_snake_case(&mut self) -> String {
        let text = format!("unknown_{}", self.counter);
        self.counter += 1;
        text
    }
}
