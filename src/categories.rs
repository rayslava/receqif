#[cfg(feature = "telegram")]
use crate::ui::input_category;
use libc::isatty;
use radix_trie::Trie;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;

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

pub type CatStats = Trie<String, Vec<CatStat>>;

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
pub fn assign_category(item: &str, cat: &str, storage: &mut CatStats) {
    if cat.is_empty() {
        panic!("Do not assign empty category!")
    }
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
pub fn get_top_category<'a>(item: &str, storage: &'a CatStats) -> Option<&'a str> {
    storage.get(item).map(|s| -> &'a str { &s[0].category })
}

/// Choose proper category or ask user
pub fn get_category(item: &str, storage: &mut CatStats, accounts: &HashSet<String>) -> String {
    let istty = unsafe { isatty(libc::STDOUT_FILENO) } != 0;
    if istty {
        let topcat = match get_top_category(item, storage) {
            Some(cat) => String::from(cat),
            None => String::new(),
        };
        let cats: Vec<&String> = accounts
            .iter()
            .filter(|acc| acc.contains("Expenses:"))
            .collect();
        let cat = input_category(item, &topcat, &cats);
        if cat.is_empty() {
            topcat
        } else {
            assign_category(item, &cat, storage);
            cat
        }
    } else {
        match get_top_category(item, storage) {
            Some(cat) => String::from(cat),
            None => String::new(),
        }
    }
}

pub struct LineFilter<F>
where
    F: Fn(&str) -> &str,
{
    filter: F,
}

impl LineFilter<fn(&str) -> &str> {
    pub fn new() -> Self {
        Self {
            filter: |input| input,
        }
    }
}

impl<F> LineFilter<F>
where
    F: Fn(&str) -> &str,
{
    pub fn numfilter(self) -> LineFilter<impl Fn(&str) -> &str> {
        LineFilter {
            filter: move |input| {
                let intermediate = (self.filter)(input);
                intermediate
                    .trim_start()
                    .trim_start_matches(char::is_numeric)
                    .trim_start()
            },
        }
    }

    pub fn perekrestok_filter(self) -> LineFilter<impl Fn(&str) -> &str> {
        LineFilter {
            filter: move |input| {
                let intermediate = (self.filter)(input);
                intermediate
                    .trim_start()
                    .trim_start_matches(char::is_numeric)
                    .trim_start_matches(['*', ':', ' '])
                    .trim_start()
            },
        }
    }

    pub fn trim_units_from_end(self) -> LineFilter<impl Fn(&str) -> &str> {
        LineFilter {
            filter: move |input| {
                let intermediate = (self.filter)(input);
                let units = ["кг", "г", "мл", "л"];

                let mut trimmed = intermediate;

                for unit in &units {
                    if trimmed.ends_with(unit) {
                        trimmed = trimmed.trim_end_matches(unit).trim_end();
                        break;
                    }
                }

                // Trim any numeric characters at the end.
                trimmed = trimmed.trim_end_matches(char::is_numeric).trim_end();

                trimmed
            },
        }
    }

    pub fn build(self) -> impl Fn(&str) -> &str {
        self.filter
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
        let topcat = get_top_category("item", &cm).unwrap();
        assert_eq!(topcat, "category");
    }

    #[test]
    fn test_new() {
        let filter = LineFilter::new();
        assert_eq!(filter.build()("Hello"), "Hello");
    }

    #[test]
    fn test_numfilter() {
        let filter = LineFilter::new().numfilter();
        assert_eq!(filter.build()("123Hello"), "Hello");
    }

    #[test]
    fn test_perekrestok_filter() {
        let filter = LineFilter::new().perekrestok_filter();
        assert_eq!(filter.build()("123: *Hello"), "Hello");
    }

    #[test]
    fn test_chaining() {
        let filter = LineFilter::new().numfilter().perekrestok_filter();
        assert_eq!(filter.build()("123: *Hello"), "Hello");
    }

    #[test]
    fn test_trim_no_unit() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Apple Juice"), "Apple Juice");
    }

    #[test]
    fn test_trim_kg() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Oranges 2кг"), "Oranges");
    }

    #[test]
    fn test_trim_g() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Salt 500 г"), "Salt");
    }

    #[test]
    fn test_trim_ml() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Water 150мл"), "Water");
    }

    #[test]
    fn test_trim_l() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Milk 2 л"), "Milk");
    }

    #[test]
    fn test_trim_multiple_spaces() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Honey   100  г"), "Honey");
    }

    #[test]
    fn test_trim_with_other_text() {
        let filter = LineFilter::new().trim_units_from_end().build();
        assert_eq!(filter("Bread 300г Extra"), "Bread 300г Extra");
    }
}
