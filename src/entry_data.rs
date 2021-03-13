use std::{collections::HashMap, fs::Metadata};

use internment::ArcIntern;

/// trade higher runtime with lower peak memory usage
pub type Term = ArcIntern<String>;

/// An entry is a file or a directory.
/// Data per entry is the map of used terms and their counts.
#[derive(Clone, Debug, Default)]
pub struct EntryData {
    path: String,
    metadata: Option<Metadata>,
    term_count: HashMap<Term, usize>,
}

impl EntryData {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.into(),
            metadata: std::fs::metadata(path).ok(),
            ..Self::default()
        }
    }

    pub fn tot_up(&mut self, other: &Self) {
        tot_up(&mut self.term_count, &other.term_count);
    }

    pub fn inc_term(&mut self, string: &str) {
        let term = Term::from(string);
        self.term_count
            .entry(term)
            .and_modify(|x| *x += 1)
            .or_insert(1);
    }

    pub fn sorted_term_counts(&self) -> Vec<(&Term, &usize)> {
        let mut term_counts: Vec<_> = self.term_count.iter().collect();
        term_counts.sort_by_key(|entry| std::usize::MAX - entry.1);
        term_counts
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }
}

fn tot_up(dest: &mut HashMap<Term, usize>, src: &HashMap<Term, usize>) {
    for (key, value) in src.iter() {
        *dest.entry(key.clone()).or_default() += *value;
    }
}

#[test]
fn EntryData_tot_up_term_counts() {
    let mut foo = EntryData::new("foo");
    foo.inc_term("term1");

    let mut bar = EntryData::new("bar");
    bar.inc_term("term1");
    bar.inc_term("term2");

    foo.tot_up(&bar);
}
