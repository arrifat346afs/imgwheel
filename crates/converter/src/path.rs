use std::path::{Path, PathBuf};

pub fn resolve_output_path(
    parent_dir: &Path,
    file_stem: &str,
    target_ext: &str,
    overwrite: bool,
) -> PathBuf {
    let base_name = format!("{}.{}", file_stem, target_ext);
    let initial_path = parent_dir.join(&base_name);

    if overwrite || !initial_path.exists() {
        return initial_path;
    }

    let mut counter = 1;
    loop {
        let candidate_name = format!("{} ({}).{}", file_stem, counter, target_ext);
        let candidate_path = parent_dir.join(&candidate_name);
        if !candidate_path.exists() {
            return candidate_path;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_collision_naming() {
        let dir = tempdir().unwrap();
        let parent = dir.path();

        // 1. Doesn't exist yet
        let p1 = resolve_output_path(parent, "photo", "webp", false);
        assert_eq!(p1, parent.join("photo.webp"));
        File::create(&p1).unwrap();

        // 2. Already exists, should append (1)
        let p2 = resolve_output_path(parent, "photo", "webp", false);
        assert_eq!(p2, parent.join("photo (1).webp"));
        File::create(&p2).unwrap();

        // 3. (1) exists, should append (2)
        let p3 = resolve_output_path(parent, "photo", "webp", false);
        assert_eq!(p3, parent.join("photo (2).webp"));

        // 4. If overwrite is true, always returns photo.webp
        let p_over = resolve_output_path(parent, "photo", "webp", true);
        assert_eq!(p_over, parent.join("photo.webp"));
    }
}
