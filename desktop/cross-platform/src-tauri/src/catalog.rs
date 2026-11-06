//! Catalog of manually playable clips (`<frames_dir>/extras/catalog.json`), shown in the
//! menu-bar "Play animation" submenu. Frames are loaded by the renderer only when played.
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::warn;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Clip {
    pub id: String,
    pub title: String,
    pub category: String,
    pub frames: u32,
    #[serde(default = "default_fps")]
    pub fps: u32,
}

fn default_fps() -> u32 {
    8
}

#[derive(Deserialize)]
struct CatalogFile {
    clips: Vec<Clip>,
}

/// Read the catalog. A missing or broken file is not fatal: the menu just has no clips.
pub fn load(frames_dir: &str) -> Vec<Clip> {
    let path = Path::new(frames_dir).join("extras").join("catalog.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<CatalogFile>(&text) {
            Ok(file) => file.clips,
            Err(e) => {
                warn!("Cannot parse {}: {}", path.display(), e);
                Vec::new()
            }
        },
        Err(e) => {
            warn!("No animation catalog at {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

/// Categories in first-seen order, clips inside each sorted by title.
pub fn group_by_category(clips: &[Clip]) -> Vec<(String, Vec<Clip>)> {
    let mut groups: Vec<(String, Vec<Clip>)> = Vec::new();
    for clip in clips {
        match groups.iter_mut().find(|(c, _)| *c == clip.category) {
            Some((_, list)) => list.push(clip.clone()),
            None => groups.push((clip.category.clone(), vec![clip.clone()])),
        }
    }
    for (_, list) in &mut groups {
        list.sort_by(|a, b| a.title.cmp(&b.title));
    }
    groups
}

/// Index for the "Random animation" item; `None` when there is nothing to play.
pub fn pick_index(len: usize, seed: u128) -> Option<usize> {
    if len == 0 {
        None
    } else {
        Some((seed % len as u128) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(id: &str, title: &str, category: &str) -> Clip {
        Clip {
            id: id.into(),
            title: title.into(),
            category: category.into(),
            frames: 16,
            fps: 8,
        }
    }

    #[test]
    fn ac_0_6_4_catalog_groups_by_category() {
        let clips = vec![
            clip("c1", "Yawning", "Moods"),
            clip("c2", "Eating lunch", "Meals"),
            clip("c3", "Startled", "Moods"),
        ];
        let g = group_by_category(&clips);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].0, "Moods", "categories keep first-seen order");
        assert_eq!(g[0].1[0].title, "Startled", "clips are sorted by title");
        assert_eq!(g[1].1.len(), 1);
    }

    #[test]
    fn ac_0_6_4_random_pick_stays_in_range() {
        assert_eq!(pick_index(0, 12345), None);
        for seed in [0u128, 1, 105, 106, u128::MAX] {
            assert!(pick_index(106, seed).unwrap() < 106);
        }
    }

    #[test]
    fn ac_0_6_4_missing_catalog_is_empty_not_fatal() {
        assert!(load("/nonexistent/frames").is_empty());
    }
}
