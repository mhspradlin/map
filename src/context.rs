use std::path::PathBuf;

#[derive(Clone)]
pub struct MapFileContext {
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub dry_run: bool
}