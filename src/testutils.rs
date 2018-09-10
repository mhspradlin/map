use super::*;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

pub fn with_default_test_file<F>(test_dir: &PathBuf, test_method: F)
where
    F: Fn(&PathBuf),
{
    let test_file_name = random_string() + "test_file.test";
    let test_file: &PathBuf = &test_dir.join(test_file_name);
    with_test_file(test_file, test_method);
}

pub fn with_test_file<F>(test_file: &PathBuf, test_method: F)
where
    F: Fn(&PathBuf),
{
    // Make sure file test file exists
    if !test_file.is_file() {
        File::create(test_file).unwrap();
    }
    assert_eq!(test_file.is_file(), true);

    test_method(test_file);

    // Clean up if the test file wasn't deleted
    if test_file.is_file() {
        fs::remove_file(test_file).unwrap();
    }
}

pub fn dummy_map_file_context() -> MapFileContext {
    MapFileContext {
        source_dir: PathBuf::from("dummy-source-dir"),
        dest_dir: PathBuf::from("dummy-dest-dir"),
        dry_run: false
    }
}

pub fn with_default_test_directory<F>(test_method: F)
where
    F: Fn(&PathBuf),
{
    let test_dir_name = "./test_output/output".to_owned() + &random_string();
    with_test_directory(&PathBuf::from(test_dir_name), test_method);
}

pub fn with_test_directory<F>(test_dir: &PathBuf, test_method: F)
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

pub fn with_default_output_directory<F>(test_dir: &PathBuf, test_method: F)
where
    F: Fn(&PathBuf),
{
    let test_output_name = random_string() + "test_output";
    with_output_directory(&test_dir.join(test_output_name), test_method);
}

pub fn with_output_directory<F>(output_directory: &PathBuf, test_method: F)
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

pub fn random_string() -> String {
    let random_number = rand::random::<u32>();
    println!("Using random number: {:?}", random_number);
    random_number.to_string()
}