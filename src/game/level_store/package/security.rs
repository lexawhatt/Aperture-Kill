use std::fs;
use std::path::{Component, Path, PathBuf};

use super::{LevelPackageError, PackageResult};

pub(super) fn validate_package_path(path: &str) -> PackageResult<()> {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.contains(':')
    {
        return Err(LevelPackageError::InvalidPackagePath {
            path: path.to_string(),
        });
    }

    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(LevelPackageError::InvalidPackagePath {
                    path: path.to_string(),
                });
            }
        }
    }

    Ok(())
}

fn package_path(root: &Path, rel: &str) -> PackageResult<PathBuf> {
    validate_package_path(rel)?;
    Ok(root.join(rel))
}

pub(super) fn read_package_string(root: &Path, rel: &str) -> PackageResult<String> {
    let path = package_path(root, rel)?;
    reject_symlink(&path)?;
    Ok(fs::read_to_string(path)?)
}

pub(super) fn read_package_bytes(root: &Path, rel: &str) -> PackageResult<Vec<u8>> {
    let path = package_path(root, rel)?;
    reject_symlink(&path)?;
    Ok(fs::read(path)?)
}

pub(super) fn write_package_string(root: &Path, rel: &str, value: &str) -> PackageResult<()> {
    write_package_bytes(root, rel, value.as_bytes())
}

pub(super) fn write_package_bytes(root: &Path, rel: &str, value: &[u8]) -> PackageResult<()> {
    let path = package_path(root, rel)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(fs::write(path, value)?)
}

fn reject_symlink(path: &Path) -> PackageResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(LevelPackageError::SymlinkPackageEntry {
            path: path.display().to_string(),
        });
    }

    Ok(())
}
