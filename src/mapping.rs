use std::path::PathBuf;

use rule::MapRule;
use action::{MapAction, MapFileTask};
use context::MapFileContext;
use error::*;

pub struct Mapping {
    rule: Box<dyn MapRule>,
    action: Box<dyn MapAction>
}

impl Mapping {
    pub fn new(rule: Box<dyn MapRule>, action: Box<dyn MapAction>) -> Mapping {
        Mapping { rule, action }
    }
}

pub fn determine_tasks<'a>(mappings: &Vec<Mapping>, files: &Vec<PathBuf>, file_context: &MapFileContext) -> Result<Vec<MapFileTask<'a>>> {
    let mut tasks: Vec<MapFileTask<'static>> = Vec::new();
    for file_path in files {
        let task: Option<MapFileTask<'static>> = determine_task(&mappings, file_path.clone(), file_context.clone())?;
        match task {
            Some(function) => tasks.push(function),
            None => debug!("No rule matches for file: {}", file_path.to_string_lossy())
        }
    }

    Ok(tasks)
}

fn determine_task<'a>(mappings: &Vec<Mapping>, file: PathBuf, file_context: MapFileContext) -> Result<Option<MapFileTask<'a>>> {
    let mut task: Option<MapFileTask<'a>> = None;
    let mut found_mapping: Option<&Mapping> = None;
    for mapping in mappings {
        if mapping.rule.file_matches_rule(&file, &file_context) {
            if task.is_none() {
                task = Some(mapping.action.create_task(file.clone()));
                found_mapping = Some(&mapping);
            } else {
                bail!("Duplicate rules {:?} and {:?} match file {}", found_mapping.unwrap().rule,
                      mapping.rule, file.to_string_lossy())
            }
        }
    }

    Ok(task)
}

#[cfg(test)]
mod test {
    use super::*;
    use testutils::*;

    #[test]
    fn determine_tasks_no_mappings() {
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                let tasks = determine_tasks(&vec![], &vec![test_file.clone()], &dummy_map_file_context()).unwrap();
                assert_eq!(tasks.len(), 0);
            })
        });
    }

    #[test]
    fn determine_tasks_no_files() {
        let mappings = vec![
            Mapping {
                rule: Box::new(TestMapRule(PathBuf::from("not-used"))),
                action: Box::new(TestMapAction())
            }
        ];
        let tasks = determine_tasks(&mappings, &vec![], &dummy_map_file_context()).unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[test]
    fn determine_tasks_with_mappings() {
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                let mappings = vec![
                    Mapping {
                        rule: Box::new(TestMapRule(PathBuf::from("does-not-match"))),
                        action: Box::new(TestErrorMapAction())
                    },
                    Mapping {
                        rule: Box::new(TestMapRule(test_file.clone())),
                        action: Box::new(TestMapAction())
                    }
                ];
                let mut tasks = determine_tasks(&mappings, &vec![test_file.clone()], &dummy_map_file_context()).unwrap();
                assert_eq!(tasks.len(), 1);
                assert_eq!(tasks.pop().unwrap().execute(&dummy_map_file_context()).is_ok(), true);
            })
        });
    }

    #[test]
    fn determine_tasks_overlapping_mappings() {
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                let mappings = vec![
                    Mapping {
                        rule: Box::new(TestMapRule(test_file.clone())),
                        action: Box::new(TestMapAction())
                    },
                    Mapping {
                        rule: Box::new(TestMapRule(test_file.clone())),
                        action: Box::new(TestMapAction())
                    }
                ];

                let tasks = determine_tasks(&mappings, &vec![test_file.clone()], &dummy_map_file_context());
                assert_eq!(tasks.is_err(), true);
            })
        });
    }

    #[derive(Debug)]
    struct TestMapRule(PathBuf);

    impl MapRule for TestMapRule {
        fn file_matches_rule(&self, file: &PathBuf, _file_context: &MapFileContext) -> bool {
            file == &self.0
        }
    }

    struct TestMapAction();
    struct TestErrorMapAction();

    impl MapAction for TestMapAction {
        fn create_task<'a>(&self, _file: PathBuf) -> MapFileTask<'a> {
            MapFileTask::new(|_file_context| Ok(()))
        }
    }

    impl MapAction for TestErrorMapAction {
        fn create_task<'a>(&self, _file: PathBuf) -> MapFileTask<'a> {
            MapFileTask::new(|_file_context| bail!("Always returns an error"))
        }
    }
}