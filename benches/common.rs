use std::path::{Path, PathBuf};
use uuid::Uuid;
pub const BENCH_ITEMS_AMOUNT: usize = 1_000_000;

/// Get a random generated temp file path + prefix(directory).
pub fn get_path_prefix(prefix: impl AsRef<str>) -> PathBuf {
  std::env::temp_dir()
    .join(prefix.as_ref())
    .join(Uuid::now_v7().to_string())
}

// Return file paths for dbs to use.
pub fn get_db_paths(dir_target: impl AsRef<Path>, amount_max: usize) -> Vec<PathBuf> {
  let dir_target = dir_target.as_ref();
  (0..amount_max)
    .map(|i| dir_target.join(format!("db_{i}.db")))
    .collect()
}
