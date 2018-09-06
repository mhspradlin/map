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
            let output_directory = create_output_directory(&file_context.dest_dir, &relative_destination, file_context.dry_run)?;
            let file_name = match file.file_name() {
                Some(name) => name,
                None => bail!("Internal failure: File {} does not have a file name. This is a bug.", file.to_string_lossy())
            };
            let destination: PathBuf = output_directory.join(file_name);
            info!("Copying {} -> {}", file.to_string_lossy(), destination.to_string_lossy());
            if !file_context.dry_run {
                fs::copy(&file, &destination)
                    .chain_err(|| format!("Unable to copy file {} to destination {}", file.to_string_lossy(),
                                          &destination.to_string_lossy()))?;
            }

            Ok(())
        };

        MapFileTask::new(task)
    }
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

    use std::fs::File;
    use rand;

    #[test]
    fn copy_action_task_dry_run_does_not_create_output_directory() {
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_destination") };
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
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_destination") };
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
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_i,l|l;e:g'al\"name") };
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
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_destination") };
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
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_destination") };
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
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_destination") };
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
        let action = CopyAction { relative_destination: PathBuf::from(random_string() + "_destination") };
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
                    assert_eq!(action.relative_destination.join(test_file.file_name().unwrap()).is_file(),
                               false);
                })
            })
        });
    }

    fn with_default_test_file<F>(test_dir: &PathBuf, test_method: F)
    where
        F: Fn(&PathBuf),
    {
        let test_file_name = random_string() + "test_file.test";
        let test_file: &PathBuf = &test_dir.join(test_file_name);
        with_test_file(test_file, test_method);
    }

    fn with_test_file<F>(test_file: &PathBuf, test_method: F)
    where
        F: Fn(&PathBuf),
    {
        // Make sure file test file exists
        if !test_file.is_file() {
            File::create(test_file).unwrap();
        }
        assert_eq!(test_file.is_file(), true);

        test_method(test_file);

        // Clean up
        fs::remove_file(test_file).unwrap();
    }

    fn with_default_test_directory<F>(test_method: F)
    where
        F: Fn(&PathBuf),
    {
        let test_dir_name = "./test_output/output".to_owned() + &random_string();
        with_test_directory(&PathBuf::from(test_dir_name), test_method);
    }

    fn with_test_directory<F>(test_dir: &PathBuf, test_method: F)
    where
        F: Fn(&PathBuf),
    {
        // Make sure test directory exists and is empty
        if test_dir.is_dir() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        fs::create_dir_all(test_dir).unwrap();
        assert_eq!(test_dir.is_dir(), true);

        test_method(test_dir);

        // Clean up
        fs::remove_dir_all(test_dir).unwrap();
    }

    fn with_default_output_directory<F>(test_dir: &PathBuf, test_method: F)
    where
        F: Fn(&PathBuf),
    {
        let test_output_name = random_string() + "test_output";
        with_output_directory(&test_dir.join(test_output_name), test_method);
    }

    fn with_output_directory<F>(output_directory: &PathBuf, test_method: F)
    where
        F: Fn(&PathBuf),
    {
        // Make sure output directory (and any files that would be in it) *does not* already exist
        if output_directory.is_dir() {
            fs::remove_dir_all(output_directory).unwrap();
        }
        assert_eq!(output_directory.is_dir(), false);

        test_method(output_directory);

        // Clean up if the output directory was created
        if output_directory.is_dir() {
            fs::remove_dir_all(output_directory).unwrap();
        }
    }

    fn random_string() -> String {
        let random_number = rand::random::<u32>();
        println!("Using random number: {:?}", random_number);
        random_number.to_string()
    }
}