use convert_case::{Case, Casing};

use crate::iota::Iota;

pub fn to_pascal_case_or_unknown(text: &str, iota: &mut Iota) -> String {
    let text = clean(text);
    match text.is_empty() {
        true => format!("Unknown{}", iota.next()),
        false => text.to_case(Case::Pascal),
    }
}

pub fn to_camel_case_or_unknown(text: &str, iota: &mut Iota) -> String {
    let text = clean(text);
    match text.is_empty() {
        true => format!("unknown{}", iota.next()),
        false => text.to_case(Case::Camel),
    }
}

pub fn to_snake_case_or_unknown(text: &str, iota: &mut Iota) -> String {
    let text = clean(text);
    match text.is_empty() {
        true => format!("unknown_{}", iota.next()),
        false => text.to_case(Case::Snake),
    }
}

/// keep only ascii alphanumeric, ascii whitespace and underscore.
/// there will only be atmost one whitespace between two words.
/// there won't be any leading or trailing whitespaces
/// there won't be any leading digits
fn clean(text: &str) -> String {
    let text: String = text.replace(|c: char| !(c.is_ascii_alphanumeric() || c == '_'), " ");
    let segments: Vec<&str> = text
        .split_ascii_whitespace()
        .filter(|s| !s.is_empty())
        .collect();
    let segments = trim_leading_digits(&segments);
    segments.join(" ")
}

fn trim_leading_digits<'s>(segments: &[&'s str]) -> Vec<&'s str> {
    match segments {
        [] => vec![],
        [first, rest @ ..] => {
            let first = first.trim_start_matches(|c: char| c.is_ascii_digit());
            match first.is_empty() {
                true => trim_leading_digits(rest),
                false => {
                    let mut v = vec![first];
                    v.extend(rest);
                    v
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    struct TestCase<'a> {
        input: &'a str,
        pascal: &'a str,
        camel: &'a str,
        snake: &'a str,
    }

    impl<'a> TestCase<'a> {
        fn assert(self) {
            assert_eq!(
                self.pascal,
                to_pascal_case_or_unknown(self.input, &mut Iota::new()),
                "mismatch pascal"
            );
            assert_eq!(
                self.camel,
                to_camel_case_or_unknown(self.input, &mut Iota::new()),
                "mismatch camel"
            );
            assert_eq!(
                self.snake,
                to_snake_case_or_unknown(self.input, &mut Iota::new()),
                "mismatch snake"
            );
        }
    }

    #[test]
    fn test() {
        TestCase {
            input: "basic",
            pascal: "Basic",
            camel: "basic",
            snake: "basic",
        }
        .assert();

        TestCase {
            input: "RubberDuck",
            pascal: "RubberDuck",
            camel: "rubberDuck",
            snake: "rubber_duck",
        }
        .assert();

        TestCase {
            input: "rubberDuck",
            pascal: "RubberDuck",
            camel: "rubberDuck",
            snake: "rubber_duck",
        }
        .assert();

        TestCase {
            input: "rubber_duck",
            pascal: "RubberDuck",
            camel: "rubberDuck",
            snake: "rubber_duck",
        }
        .assert();

        TestCase {
            input: "",
            pascal: "Unknown0",
            camel: "unknown0",
            snake: "unknown_0",
        }
        .assert();

        TestCase {
            input: "    ",
            pascal: "Unknown0",
            camel: "unknown0",
            snake: "unknown_0",
        }
        .assert();

        TestCase {
            input: "こんにちは",
            pascal: "Unknown0",
            camel: "unknown0",
            snake: "unknown_0",
        }
        .assert();

        TestCase {
            input: "spaces    between",
            pascal: "SpacesBetween",
            camel: "spacesBetween",
            snake: "spaces_between",
        }
        .assert();

        TestCase {
            input: "123digits",
            pascal: "Digits",
            camel: "digits",
            snake: "digits",
        }
        .assert();

        TestCase {
            input: "123 digits",
            pascal: "Digits",
            camel: "digits",
            snake: "digits",
        }
        .assert();

        TestCase {
            input: "   123  56foo88  33  こんにちは  ",
            pascal: "Foo8833",
            camel: "foo8833",
            snake: "foo_88_33",
        }
        .assert();
    }
}
