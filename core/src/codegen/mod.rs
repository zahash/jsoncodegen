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

    // TODO: PascalCase string must NOT start with a number
    fn pascal_case(&mut self, text: &str) -> String {
        let clean_text: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        let words: Vec<String> = clean_text
            .split(|c: char| c == '_' || c.is_whitespace())
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                let first_char = chars.next().unwrap().to_ascii_uppercase();
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

    // TODO: camelCase string must NOT start with a number
    fn camel_case(&mut self, text: &str) -> String {
        let clean_text: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        let mut words: Vec<String> = clean_text
            .split(|c: char| c == '_' || c.is_whitespace())
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                let first_char = chars.next().unwrap().to_ascii_uppercase();
                let rest: String = chars.collect();
                format!("{}{}", first_char, rest)
            })
            .collect();

        if let Some(first_word) = words.iter_mut().next() {
            let mut chars = first_word.chars();
            let first_char = chars.next().unwrap().to_ascii_lowercase();
            let rest: String = chars.collect();
            *first_word = format!("{}{}", first_char, rest);
        }

        let result = words.concat();
        match result.is_empty() {
            true => self.unknown_camel_case(),
            false => result,
        }
    }

    // TODO: snake_case string must NOT start with a number
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::CaseConverter;

    #[test]
    fn test() {
        let mut case_converter = CaseConverter::new();
        assert_eq!("Unknown0", case_converter.pascal_case("て"));
        assert_eq!("unknown1", case_converter.camel_case("て"));
        assert_eq!("unknown_2", case_converter.snake_case("て"));
    }
}
