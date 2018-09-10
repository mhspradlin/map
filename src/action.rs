use std::path::PathBuf;
use std::fs;

use context::MapFileContext;
use error::*;

pub trait MapAction {
    fn create_task<'a>(&self, file: PathBuf) -> MapFileTask<'a>;
}

pub struct MapFileTask<'a> {
    task: Box<FnMut(&MapFileContext) -> Result<()> + 'a>
}

impl<'a> MapFileTask<'a> {
    pub fn execute(mut self, file_context: &MapFileContext) -> Result<()> {
        let task_function: &mut FnMut(&MapFileContext) -> Result<()> = &mut *self.task;
        task_function(file_context)
    }

    pub fn new<T>(task_function: T) -> MapFileTask<'a> where T: FnMut(&MapFileContext) -> Result<()> + 'a {
        MapFileTask { task: Box::new(task_function) }
    }
}

pub struct CopyAction {
    relative_destination: PathBuf
}

impl CopyAction {
    pub fn new(relative_destination: PathBuf) -> CopyAction {
        CopyAction { relative_destination }
    }
}

impl MapAction for CopyAction {
    fn create_task<'a>(&self, file: PathBuf) -> MapFileTask<'a> {
        let relative_destination = self.relative_destination.clone();
        let task = move |file_context: &MapFileContext| {
            perform_file_operation(&file, file_context, &relative_destination, |destination: &PathBuf| {
                info!("Copying {} -> {}", file.to_string_lossy(), destination.to_string_lossy());
                if !file_context.dry_run {
                    fs::copy(&file, &destination)
                        .chain_err(|| format!("Unable to copy file {} to destination {}", file.to_string_lossy(),
                                            &destination.to_string_lossy()))?;
                }
                Ok(())
            })
        };

        MapFileTask::new(task)
    }
}

pub struct MoveAction {
    relative_destination: PathBuf
}

impl MoveAction {
    pub fn new(relative_destination: PathBuf) -> MoveAction {
        MoveAction { relative_destination }
    }
}

impl MapAction for MoveAction {
    fn create_task<'a>(&self, file: PathBuf) -> MapFileTask<'a> {
        let relative_destination = self.relative_destination.clone();
        let task = move |file_context: &MapFileContext| {
            perform_file_operation(&file, file_context, &relative_destination, |destination: &PathBuf| {
                info!("Moving {} -> {}", file.to_string_lossy(), destination.to_string_lossy());
                if !file_context.dry_run {
                    fs::rename(&file, &destination)
                        .chain_err(|| format!("Unable to move file {} to destination {}", file.to_string_lossy(),
                                            &destination.to_string_lossy()))?;
                }
                Ok(())
            })
        };

        MapFileTask::new(task)
    }
}

fn perform_file_operation(file: &PathBuf, file_context: &MapFileContext, relative_destination: &PathBuf, 
                          mut operation: impl FnMut(&PathBuf) -> Result<()>) -> Result<()> {
    let output_directory = create_output_directory(&file_context.dest_dir, relative_destination, file_context.dry_run)?;
    let file_name = match file.file_name() {
        Some(name) => name,
        None => bail!("Internal failure: File {} does not have a file name. This is a bug.", file.to_string_lossy())
    };
    let destination: PathBuf = output_directory.join(file_name);
    operation(&destination)
}

fn create_output_directory(
    destination_directory: &PathBuf,
    relative_output_directory: &PathBuf,
    dry_run: bool,
) -> Result<PathBuf> {
    let destination_directory: PathBuf = destination_directory.join(&relative_output_directory);
    if !destination_directory.is_dir() {
        info!("Creating destination directory: {}", destination_directory.to_string_lossy());
        if !dry_run {
            fs::create_dir_all(&destination_directory)
                .chain_err(|| format!("Unable to create destination directory: {}",
                                      destination_directory.to_string_lossy()))?
        }
    }

    Ok(destination_directory)
}

#[cfg(test)]
mod test {
    use super::*;
    use testutils::*;

    #[test]
    fn copy_action_task_dry_run_does_not_create_output_directory() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: true
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(output_directory.is_dir(), false);
                })
            })
        });
    }

    #[test]
    fn copy_action_task_creates_output_directory() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(output_directory.is_dir(), true);
                })
            })
        });
    }

    #[test]
    fn copy_action_task_create_output_directory_failure() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_i,l|l;e:g'al\"name"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.clone());
                    let result = task.execute(&map_file_context);
                    assert_eq!(result.is_err(), true);
                })
            })
        });
    }

    #[test]
    fn copy_action_task_file_has_no_file_name() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.join(PathBuf::from("..")));
                    let result = task.execute(&map_file_context);
                    assert_eq!(result.is_err(), true);
                })
            })
        });
    }

    #[test]
    fn copy_action_task_dry_run_does_not_copy_file() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: true
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(action.relative_destination.join(test_file.file_name().unwrap()).is_file(),
                               false);
                })
            })
        });
    }

    #[test]
    fn copy_action_task_file_copy_failure() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.join("_i,l|l;e:g'al\"name"));
                    let result = task.execute(&map_file_context);
                    assert_eq!(result.is_err(), true);
                })
            })
        });
    }

    #[test]
    fn copy_action_task_file_copies_file() {
        let action = CopyAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(output_directory.join(action.relative_destination.join(test_file.file_name().unwrap())).is_file(), true);
                    assert_eq!(test_file.is_file(), true);
                })
            })
        });
    }

    #[test]
    fn move_action_task_dry_run_does_not_create_output_directory() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: true
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(output_directory.is_dir(), false);
                })
            })
        });
    }

    #[test]
    fn move_action_task_creates_output_directory() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(output_directory.is_dir(), true);
                })
            })
        });
    }

    #[test]
    fn move_action_task_create_output_directory_failure() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_i,l|l;e:g'al\"name"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.clone());
                    let result = task.execute(&map_file_context);
                    assert_eq!(result.is_err(), true);
                })
            })
        });
    }

    #[test]
    fn move_action_task_file_has_no_file_name() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.join(PathBuf::from("..")));
                    let result = task.execute(&map_file_context);
                    assert_eq!(result.is_err(), true);
                })
            })
        });
    }

    #[test]
    fn move_action_task_dry_run_does_not_move_file() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: true
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(action.relative_destination.join(test_file.file_name().unwrap()).is_file(),
                               false);
                    assert_eq!(test_file.is_file(), true);
                })
            })
        });
    }

    #[test]
    fn move_action_task_file_move_failure() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.join("_i,l|l;e:g'al\"name"));
                    let result = task.execute(&map_file_context);
                    assert_eq!(result.is_err(), true);
                })
            })
        });
    }

    #[test]
    fn move_action_task_file_moves_file() {
        let action = MoveAction::new(PathBuf::from(random_string() + "_destination"));
        with_default_test_directory(|test_directory| {
            with_default_test_file(test_directory, |test_file| {
                with_default_output_directory(test_directory, |output_directory| {
                    let map_file_context = MapFileContext {
                        source_dir: test_directory.clone(),
                        dest_dir: output_directory.clone(),
                        dry_run: false
                    };
                    let task = action.create_task(test_file.clone());
                    task.execute(&map_file_context).unwrap();
                    assert_eq!(output_directory.join(action.relative_destination.join(test_file.file_name().unwrap())).is_file(), true);
                    assert_eq!(test_file.is_file(), false);
                })
            })
        });
    }
}