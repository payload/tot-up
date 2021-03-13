use std::{collections::HashMap, path::{Path, PathBuf}};

use internment::ArcIntern;

/// trade higher runtime with lower peak memory usage
pub type Term = ArcIntern<String>;

/// An entry is a file or a directory.
/// Data per entry is the map of used terms and their counts.
#[derive(Clone, Debug, Default)]
pub struct EntryData {
    path: PathBuf,
    term_count: HashMap<Term, u64>,
}

impl EntryData {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            ..Self::default()
        }
    }

    pub fn tot_up(&mut self, other: &Self) {
        tot_up(&mut self.term_count, &other.term_count);
    }

    pub fn inc_term(&mut self, string: &str) {
        let term = Term::from(string);
        self.term_count.entry(term).and_modify(|x| *x += 1).or_insert(1);
    }

    pub fn display_histogram(&self, height: usize) -> String {
        // self.path
        // ... bars count term
        let line_one = Some(format!("{}:\n", self.path.display())).into_iter();

        let mut term_counts: Vec<_> = self.term_count.iter().collect();
        term_counts.sort_by_key(|entry| std::u64::MAX - entry.1);
        let max_count = *term_counts.first().unwrap().1 as f64;

        let bars_counts = term_counts.iter().map(|(term, count)| {
            format!(
                "{} {} {}\n",
                pct_to_bar(**count as f64 / max_count, 10),
                count,
                term
            )
        });

        line_one.chain(bars_counts).take(height).collect()
    }
}

const BARS: &[char] = &[' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

fn pct_to_bar(pct: f64, width: usize) -> String {
    let mult = (BARS.len() - 1) * width;
    let ct = pct * (mult as f64);
    let ct = ct.round();
    let mut ct = ct as usize;

    let mut out = String::with_capacity(width);

    for _ in 0..width {
        let idx = std::cmp::min(ct, BARS.len() - 1);
        ct -= idx;
        out.push(BARS[idx]);
    }

    out
}

fn tot_up(dest: &mut HashMap<Term, u64>, src: &HashMap<Term, u64>) {
    for (key, value) in src.iter() {
        *dest.entry(key.clone()).or_default() += *value;
    }
}
