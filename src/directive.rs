use std::fmt;
use std::path::PathBuf;
use regex::{Regex, Captures};

use mapping::Mapping;
use rule::*;
use action::*;
use error::*;

pub trait MappingDirective: fmt::Display {
    fn create_mapping(&self, definition: &str) -> Option<Result<Mapping>>;
}

pub struct RegexDirective {
    directive_name: String,
    format: Regex,
    action_factory: Box<Fn(Captures) -> Result<Mapping>>
}

impl fmt::Display for RegexDirective {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.directive_name)
    }
}

impl MappingDirective for RegexDirective {
    fn create_mapping(&self, definition: &str) -> Option<Result<Mapping>> {
        self.format.captures(definition)
                   .map(|captures| (self.action_factory)(captures))
    }
}

pub fn create_directives() -> Vec<Box<dyn MappingDirective>> {
    vec![
        copy_regex_directive()
    ]
}

fn copy_regex_directive() -> Box<dyn MappingDirective> {
    let directive = RegexDirective {
        directive_name: "Copy".to_string(),
        format: Regex::new(r"^\s*c\s*/(?P<regex>.*?)/\s*(?P<destination>.*?)\s*$").unwrap(),
        action_factory: Box::new(|captures: Captures| {
            let regex_string =
                captures.name("regex")
                        .chain_err(|| "No regex found for copy rule. This is a bug.")?;
            let destination_string =
                captures.name("destination")
                        .chain_err(|| "No destination found for copy rule. This is a bug.")?;
            let rule_regex = Regex::new(regex_string.as_str())
                                .chain_err(|| format!("Unable to parse regex for copy rule {}", regex_string.as_str()))?;
            let relative_destination = PathBuf::from(destination_string.as_str());
            Ok(Mapping::new(
                Box::new(RegexRule::new(rule_regex)),
                Box::new(CopyAction::new(relative_destination))
            ))
        })
    };

    Box::new(directive)
}

pub fn mapping_from_string(all_directives: &Vec<Box<dyn MappingDirective>>, directive_definition: &str) -> Option<Result<Mapping>> {
    let mut matched_directives: Vec<&Box<dyn MappingDirective>> = vec![];
    let mut found_mapping: Option<Result<Mapping>> = None;
    for directive in all_directives {
        match directive.create_mapping(directive_definition) {
            Some(mapping_result) => {
                matched_directives.push(directive);
                found_mapping = Some(mapping_result);
            },
            _ => ()
        };
    }

    if matched_directives.len() > 1 {
        let directive_list = matched_directives.iter().fold("".to_string(), |accum, next| format!("{}, {}", accum, next));
        Some(Err(Error::from(format!("Ambiguous directive '{}', which matched {}", directive_definition, directive_list))))
    } else {
        found_mapping
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn mapping_from_string_passes_directive() {
        match mapping_from_string(&create_test_directives(), "directive").unwrap() {
            Err(Error(ErrorKind::Msg(message), _)) => assert_eq!(message, "matches"),
            _ => assert!(false)
        }
    }

    #[test]
    fn mapping_from_string_overlapping_mappings() {
        let mut mappings = create_test_directives();
        mappings.append(&mut create_test_directives());
        match mapping_from_string(&mappings, "directive").unwrap() {
            Err(Error(ErrorKind::Msg(message), _)) => assert_eq!(message.contains("Ambiguous"), true),
            _ => assert!(false)
        }
    }

    fn create_test_directives() -> Vec<Box<dyn MappingDirective>> {
        vec![
            Box::new(RecordingTestDirective {
                expected_definition: "directive".to_string()
            })
        ]
    }

    #[derive(Debug)]
    struct RecordingTestDirective {
        expected_definition: String,
    }

    impl fmt::Display for RecordingTestDirective {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", "Test directive")
    }
    }

    impl MappingDirective for RecordingTestDirective {
        fn create_mapping(&self, definition: &str) -> Option<Result<Mapping>> {
            if definition == &self.expected_definition {
                Some(Err(Error::from("matches")))
            } else {
                Some(Err(Error::from("Does not match")))
            }
        }
    }

    #[test]
    fn copy_regex_directive_create_mapping_no_match() {
        let copy_regex_directive = copy_regex_directive();
        assert_eq!(copy_regex_directive.create_mapping("").is_none(), true);
    }

    #[test]
    fn copy_regex_directive_create_mapping_invalid_regex() {
        let copy_regex_directive = copy_regex_directive();
        assert_eq!(copy_regex_directive.create_mapping("c/(/ destination").unwrap().is_err(), true);
    }

    #[test]
    fn copy_regex_directive_create_mapping_valid() {
        let copy_regex_directive = copy_regex_directive();
        assert_eq!(copy_regex_directive.create_mapping("c/regex/ destination").unwrap().is_ok(), true);
    }
}