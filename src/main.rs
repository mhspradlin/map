// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate clap;
extern crate regex;
#[macro_use]
extern crate log;
extern crate log4rs;
#[macro_use]
extern crate error_chain;

// For testing in submodules
#[cfg(test)]
extern crate rand;

use clap::{App, Arg, ArgMatches};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

mod action;
mod context;
mod directive;
mod error;
mod mapping;
mod rule;

#[cfg(test)]
mod testutils;

use action::*;
use context::MapFileContext;
use directive::*;
use error::*;
use mapping::*;

use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

fn main() {
    let argument_matches = create_app().get_matches();

    configure_logging(argument_matches.occurrences_of("v"));

    // If there was an error, nicely print it and the related causes
    if let Err(ref error) = run(argument_matches) {
        error!("Error: {}", error);
        for cause in error.iter().skip(1) {
            error!("caused by: {}", cause);
        }
        ::std::process::exit(1);
    } else {
        ::std::process::exit(0);
    }
}

fn create_app<'a,'b>() -> App<'a,'b> {
    // Feature ideas:
    // Delete behavior (don't do it (default), do it during, do it at end)
    // Clobber behavior (don't do it and don't fail (default), don't do it and fail, do it)
    // Allow passing rules xor source file list
    // Copy in parallel
    App::new("map")
        .version("1.0")
        .author("Mitch S. <mitch+map@applicative.us>")
        .about("A program to copy files into folders based on name matches")
        .arg(
            Arg::with_name("rules-file")
                .short("r")
                .long("rules")
                .value_name("FILE")
                .help("Sets the file to read for file -> directory mapping rules")
                .takes_value(true)
                .required_unless("rules-arg"),
        )
        .arg(
            Arg::with_name("rules-arg")
                .help("Specifies the rule to use for file -> directory mapping")
                .index(1)
                .required_unless("rules-file"),
        )
        .arg(
            Arg::with_name("source-dir")
                .short("s")
                .long("source-dir")
                .value_name("DIRECTORY")
                .help("Sets the directory to look for files to copy into directories")
                .takes_value(true)
                .default_value(r".\"),
        )
        .arg(
            Arg::with_name("dest-dir")
                .short("d")
                .long("dest-dir")
                .value_name("DIRECTORY")
                .help("Sets the directory to create the directories to move files into")
                .takes_value(true)
                .default_value(r".\"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("dry-run")
                .short("n")
                .long("dry-run")
                .help("Sets whether or not to actually write to the filesystem"),
        )
}

fn configure_logging(verbosity: u64) {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({m})}{n}")))
        .build();
    let level = match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        3 | _ => LevelFilter::Trace,
    };
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(level))
        .unwrap();
    log4rs::init_config(config).unwrap();
}

fn run(matches: ArgMatches) -> Result<()> {
    let mappings: Vec<Mapping> = match matches.value_of("rules-file") {
        Some(file) => mappings_from_file(&create_directives(), &PathBuf::from(file))?,
        None => {
            match mapping_from_string(&create_directives(), matches.value_of("rules-arg").unwrap())
            {
                Some(result) => vec![result?],
                None => vec![],
            }
        }
    };

    let dry_run = matches.is_present("dry-run");

    // Safe to unwrap these, as we have defaults
    let source_dir = PathBuf::from(matches.value_of("source-dir").unwrap());
    let dest_dir = PathBuf::from(matches.value_of("dest-dir").unwrap());

    let file_context = MapFileContext {
        source_dir: source_dir.clone(),
        dest_dir: dest_dir.clone(),
        dry_run: dry_run,
    };

    // Get all the paths that are files
    let file_paths: Vec<PathBuf> = get_file_paths(&source_dir)?;

    // Get all the tasks for those files
    let mut tasks: Vec<MapFileTask> = determine_tasks(&mappings, &file_paths, &file_context)?;

    // Execute all the tasks
    while let Some(task) = tasks.pop() {
        task.execute(&file_context)?;
    }

    Ok(())
}

fn mappings_from_file(
    all_directives: &Vec<Box<dyn MappingDirective>>,
    file: &PathBuf,
) -> Result<Vec<Mapping>> {
    let f = fs::File::open(file)
        .chain_err(|| format!("Unable to open directive file {}", file.to_string_lossy()))?;
    let mut mappings = vec![];
    for line_result in BufReader::new(f).lines() {
        let line = line_result
            .chain_err(|| format!("Error reading directive file {}", file.to_string_lossy()))?;
        match mapping_from_string(all_directives, &line) {
            Some(result) => mappings.push(result?),
            None => (),
        };
    }

    Ok(mappings)
}

fn get_file_paths(directory: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut file_paths: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(directory).chain_err(|| {
        format!(
            "Unable to read entries of directory {}",
            directory.to_string_lossy()
        )
    })? {
        let dir_entry = entry.chain_err(|| {
            format!(
                "Unable to read entry of directory {}",
                directory.to_string_lossy()
            )
        })?;
        let file_path = dir_entry.path();
        if dir_entry.path().is_file() {
            trace!("Regular file: {}", file_path.to_string_lossy());
            file_paths.push(dir_entry.path());
        } else {
            trace!("Not a file: {}", file_path.to_string_lossy());
        }
    }

    Ok(file_paths)
}

#[cfg(test)]
mod test {
    extern crate rand;

    use super::*;
    use std::path::PathBuf;
    use testutils::*;

    #[test]
    fn get_file_paths_dir_does_not_exist() {
        match get_file_paths(&PathBuf::from("does-not-exist")) {
            Ok(_) => panic!("No results should be returned"),
            Err(_) => (),
        }
    }

    #[test]
    fn get_file_paths_no_files() {
        with_default_test_directory(|test_directory| {
            let paths: Vec<PathBuf> = get_file_paths(test_directory).unwrap();
            assert_eq!(paths.len(), 0);
        });
    }

    #[test]
    fn get_file_paths_with_file_and_directory() {
        with_default_test_directory(|test_directory| {
            with_test_directory(&test_directory.join("not-a-file"), |_inner_directory| {
                with_default_test_file(test_directory, |test_file| {
                    let mut paths: Vec<PathBuf> = get_file_paths(test_directory).unwrap();
                    assert_eq!(paths.len(), 1);
                    assert_eq!(&paths.pop().unwrap(), test_file);
                })
            })
        });
    }
}
