use std::io;
use std::path::Path;
use std::path::PathBuf;

use toml::Value as TomlValue;

const DEFAULT_PROJECT_ROOT_MARKERS: &[&str] = &[".git"];

/// Reads `project_root_markers` from a merged `config.toml` [toml::Value].
///
/// Invariants:
/// - If `project_root_markers` is not specified, returns `Ok(None)`.
/// - If `project_root_markers` is specified, returns `Ok(Some(markers))` where
///   `markers` is a `Vec<String>` (including `Ok(Some(Vec::new()))` for an
///   empty array, which indicates that root detection should be disabled).
/// - Returns an error if `project_root_markers` is specified but is not an
///   array of strings.
pub fn project_root_markers_from_config(config: &TomlValue) -> io::Result<Option<Vec<String>>> {
    let Some(table) = config.as_table() else {
        return Ok(None);
    };
    let Some(markers_value) = table.get("project_root_markers") else {
        return Ok(None);
    };
    let TomlValue::Array(entries) = markers_value else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "project_root_markers must be an array of strings",
        ));
    };
    if entries.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let mut markers = Vec::new();
    for entry in entries {
        let Some(marker) = entry.as_str() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "project_root_markers must be an array of strings",
            ));
        };
        markers.push(marker.to_string());
    }
    Ok(Some(markers))
}

pub fn default_project_root_markers() -> Vec<String> {
    DEFAULT_PROJECT_ROOT_MARKERS
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Finds the nearest ancestor that contains one of the configured project root markers.
///
/// If `project_root_markers` is empty, root detection is disabled and `cwd` is
/// returned unchanged. If no marker is found, `cwd` is returned unchanged.
pub fn find_project_root(cwd: &Path, project_root_markers: &[String]) -> PathBuf {
    if project_root_markers.is_empty() {
        return cwd.to_path_buf();
    }

    for ancestor in cwd.ancestors() {
        for marker in project_root_markers {
            if ancestor.join(marker).exists() {
                return ancestor.to_path_buf();
            }
        }
    }

    cwd.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn find_project_root_returns_nearest_marker_ancestor() {
        let tmp = TempDir::new().expect("tempdir");
        let project_root = tmp.path().join("project");
        let nested = project_root.join("child").join("grandchild");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(project_root.join(".hg"), "").expect("write marker");

        assert_eq!(
            find_project_root(&nested, &[".hg".to_string()]),
            project_root
        );
    }

    #[test]
    fn find_project_root_returns_cwd_when_markers_are_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("child");
        std::fs::create_dir_all(&nested).expect("create nested dir");

        assert_eq!(find_project_root(&nested, &[]), nested);
    }

    #[test]
    fn find_project_root_returns_cwd_when_no_marker_matches() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("child");
        std::fs::create_dir_all(&nested).expect("create nested dir");

        assert_eq!(find_project_root(&nested, &[".hg".to_string()]), nested);
    }
}
