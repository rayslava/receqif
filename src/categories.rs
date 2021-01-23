use radix_trie::Trie;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Category statistics for single item
#[derive(Serialize, Deserialize, Debug)]
pub struct CatStat {
    /// Category name
    category: String,
    /// How many times did the item hit this category
    hits: i64,
}

impl Ord for CatStat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hits.cmp(&other.hits)
    }
}

impl PartialOrd for CatStat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CatStat {
    fn eq(&self, other: &Self) -> bool {
        self.hits == other.hits
    }
}

impl Eq for CatStat {}

/// Insert new `cat` into statistics vector or add single usage to existing cat
fn update_stat(cat: &str, stat: &mut Vec<CatStat>) {
    let existing = stat.iter_mut().find(|stat| stat.category == cat);
    match existing {
        Some(e) => e.hits += 1,
        None => stat.push(CatStat {
            category: String::from(cat),
            hits: 1,
        }),
    }

    stat.sort_by(|a, b| b.cmp(a));
}

/// Set up `cat` as category for `item`: update statistics or create new item
/// in `storage`
pub fn assign_category(item: &str, cat: &str, storage: &mut Trie<String, Vec<CatStat>>) {
    let existing = storage.get_mut(item);
    match existing {
        Some(stat) => update_stat(cat, stat),
        None => {
            let newstat = CatStat {
                category: String::from(cat),
                hits: 1,
            };
            storage.insert(String::from(item), vec![newstat]);
        }
    }
}

/// Return most probable category for provided `item`
pub fn get_top_category<'a>(
    item: &str,
    storage: &'a Trie<String, Vec<CatStat>>,
) -> Option<&'a str> {
    match storage.get(item) {
        Some(stats) => Some(&stats[0].category),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_stat() {
        let mut stat: Vec<CatStat> = Vec::new();
        update_stat("test", &mut stat);
        assert_eq!(stat[0].hits, 1);
        assert_eq!(stat[0].category, "test");
        update_stat("test", &mut stat);
        assert_eq!(stat[0].hits, 2);
        update_stat("test2", &mut stat);
        assert_eq!(stat[1].category, "test2");
        assert_eq!(stat[1].hits, 1);
        update_stat("test2", &mut stat);
        update_stat("test2", &mut stat);
        assert_eq!(stat[0].category, "test2");
        assert_eq!(stat[0].hits, 3);
        assert_eq!(stat[1].hits, 2);
        assert_eq!(stat[1].category, "test");
    }

    #[test]
    fn test_assign_category() {
        let mut cm: Trie<String, Vec<CatStat>> = Trie::new();
        assign_category("item", "category", &mut cm);
        let stats = cm.get("item").unwrap();
        assert_eq!(stats[0].category, "category");
        assert_eq!(stats[0].hits, 1);
        let topcat = get_top_category("item", &mut cm).unwrap();
        assert_eq!(topcat, "category");
    }
}
