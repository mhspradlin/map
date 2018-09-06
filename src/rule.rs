use std::fmt;
use std::path::PathBuf;
use regex::Regex;

use context::MapFileContext;

pub trait MapRule: fmt::Debug {
    fn file_matches_rule(&self, file: &PathBuf, file_context: &MapFileContext) -> bool;
}

#[derive(Debug)]
pub struct RegexRule {
    rule: Regex,
}

impl RegexRule {
    pub fn new(regex: Regex) -> RegexRule {
        RegexRule { rule: regex }
    }
}

impl MapRule for RegexRule {
    fn file_matches_rule(&self, file: &PathBuf, _file_context: &MapFileContext) -> bool {
        let file_name = file.file_name().unwrap();
        self.rule.is_match(&file_name.to_string_lossy())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn regex_rule_with_matching_file() {
        let rule = RegexRule { rule: Regex::new("match").unwrap() };
        let file = PathBuf::from("./is/a/match.txt");
        let is_match = rule.file_matches_rule(&file, &dummy_map_file_context());
        assert_eq!(is_match, true);
    }

    #[test]
    fn regex_rule_no_match_on_path_parents() {
        let rule = RegexRule { rule: Regex::new("nomatch").unwrap() };
        let file = PathBuf::from("./nomatch/does/not/match.txt");
        let is_match = rule.file_matches_rule(&file, &dummy_map_file_context());
        assert_eq!(is_match, false);
    }

    fn dummy_map_file_context() -> MapFileContext {
        MapFileContext {
            source_dir: PathBuf::from("dummy-source-dir"),
            dest_dir: PathBuf::from("dummy-dest-dir"),
            dry_run: false
        }
    }
}