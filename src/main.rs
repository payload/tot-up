use std::{
    collections::HashMap,
    io::stdin,
    sync::{Arc, RwLock},
};

use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{DirEntry, WalkBuilder, WalkState};
use internment::ArcIntern;

// SessionData holds terms, data hierarchy, filters
// Walker iterates directories
// CollectData collects EntryData from files
// EntryData holds terms and counts for an entry

/// trade higher runtime with lower peak memory usage
type Term = ArcIntern<String>;

#[derive(Default)]
struct SessionData {
    #[cfg(feature = "record-terms")]
    terms: HashSet<Term>,

    entries: Vec<EntryData>,
}

struct CollectData {
    matcher: RegexMatcher,
    entry_data: EntryData,
    sink: Arc<RwLock<SessionData>>,
}

#[derive(Clone, Debug, Default)]
struct EntryData {
    path: String,
    term_count: HashMap<Term, usize>,
    sorted_vec: Option<Vec<(Term, usize)>>,
}

fn main() {
    let data = SessionData::default();
    let data_locked = Arc::new(RwLock::new(data));

    let handle_dir_entry = |result| {
        match result {
            Ok(entry) => search(entry, data_locked.clone()),
            Err(err) => eprintln!("ERROR: {}", err),
        }

        WalkState::Continue
    };

    WalkBuilder::new("./")
        // .threads(8) // TODO does it really use all cores?
        .build_parallel()
        .run(|| Box::new(handle_dir_entry));

    let data = data_locked.read().expect("unlock");

    if true {
        let (_w, h) = term_size::dimensions().unwrap_or((80, 40));

        let mut sum = EntryData {
            path: "./".to_string(),
            ..Default::default()
        };

        for entry in data.entries.iter() {
            println!("{}", entry.path);
            merge(&mut sum.term_count, &entry.term_count);
        }

        sum.sort();
        let panel = sum.build_histogram(10, h - 1);

        //

        let entry = &mut data.entries[0];
        entry.sort();
        let panel2 = entry.build_histogram(10, h - 1);

        let lines_n = panel.lines.len().max(panel2.lines.len());
        for line in panel.lines.iter() {
            println!("{}", line);
        }

        //

        if false {
            let mut buf = String::new();
            let _ = stdin().read_line(&mut buf);
        }
    }
}

fn merge(dest: &mut HashMap<Term, usize>, src: &HashMap<Term, usize>) {
    for (key, value) in src.iter() {
        *dest.entry(key.clone()).or_default() += *value;
    }
}

fn search(entry: DirEntry, data_sink: Arc<RwLock<SessionData>>) {
    if !entry.file_type().unwrap().is_file() {
        return;
    }

    let path = entry.path();
    let path_str = path.to_string_lossy().to_string();
    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(r"\w{3}\w*")
        .expect("good regex");
    let collect_data = CollectData {
        matcher: matcher.clone(),
        entry_data: EntryData {
            path: path_str,
            ..Default::default()
        },
        sink: data_sink.clone(),
    };

    grep::searcher::Searcher::new()
        .search_path(matcher, path, collect_data)
        .expect("search path");
}

impl SessionData {
    fn insert_entry_data(&mut self, data: &EntryData) {
        #[cfg(feature = "record-terms")]
        for (key, _value) in data.term_count.iter() {
            self.terms.insert(key.clone());
        }

        self.entries.push(data.clone());
    }
}

impl CollectData {
    fn exclude(&self, _string: &str) -> bool {
        false
    }
}

impl Sink for CollectData {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        sink_match: &SinkMatch,
    ) -> Result<bool, Self::Error> {
        let mut matches = Vec::new();

        let _ = self.matcher.find_iter(sink_match.bytes(), |mat| {
            let slice = &sink_match.bytes()[mat];
            matches.push(String::from_utf8_lossy(slice));
            true
        });

        for mat in matches {
            if !self.exclude(&mat) {
                let term = Term::from(&mat as &str);
                *self.entry_data.term_count.entry(term).or_default() += 1;
            }
        }

        Ok(true)
    }

    fn finish(&mut self, _searcher: &Searcher, _: &SinkFinish) -> Result<(), Self::Error> {
        if false {
            println!("{}:", self.entry_data.path);

            let mut term_counts: Vec<_> = self.entry_data.term_count.iter().collect();
            term_counts.sort_by_key(|entry| entry.1);

            println!(" terms: {}", term_counts.len());
            println!(" top 5:");
            for (term, count) in term_counts[term_counts.len().saturating_sub(5)..]
                .iter()
                .rev()
            {
                let bar = pct_to_bar(**count as f64, 50);
                println!("  {} {} {}", term, count, bar);
            }
        }

        self.sink
            .write()
            .expect("write")
            .insert_entry_data(&self.entry_data);

        Ok(())
    }
}

impl EntryData {
    fn sort(&mut self) -> &Vec<(Term, usize)> {
        let mut sorted: Vec<(Term, usize)> = self
            .term_count
            .iter()
            .map(|e| ((*e.0).clone(), *e.1))
            .collect();
        sorted.sort_by_key(|entry| std::usize::MAX - entry.1);
        self.sorted_vec = Some(sorted);
        self.sorted_vec.as_ref().unwrap()
    }

    fn build_histogram(&self, bar_width: usize, height: usize) -> Panel {
        let mut panel = Panel::default();
        let top_n = height - 1;

        panel
            .lines
            .push(format!("{} {} terms:", self.path, self.term_count.len()));

        if let Some(sorted) = &self.sorted_vec {
            for (term, count) in sorted[..top_n].iter().rev() {
                let bar = pct_to_bar(*count as f64, bar_width);
                let line = format!("{} {}", bar, term);
                panel.width = panel.width.max(line.len());
                panel.lines.push(line);
            }
        }

        panel
    }
}

#[derive(Default)]
struct Panel {
    width: usize,
    lines: Vec<String>,
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
