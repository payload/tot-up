use std::{
    collections::HashMap,
    io::stdin,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{DirEntry, WalkBuilder, WalkState};
use internment::ArcIntern;

use prettytable::{
    format::{FormatBuilder, LinePosition, LineSeparator},
    Cell, Row, Table,
};

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
    path: PathBuf,
    term_count: HashMap<Term, u64>,
}

fn main() {
    let root_path = std::env::args().nth(1).unwrap_or("./".into());

    let data = SessionData::default();
    let data_locked = Arc::new(RwLock::new(data));

    let handle_dir_entry = |result| {
        match result {
            Ok(entry) => search(entry, data_locked.clone()),
            Err(err) => eprintln!("ERROR: {}", err),
        }

        WalkState::Continue
    };

    WalkBuilder::new(&root_path)
        // .threads(8) // TODO does it really use all cores?
        .build_parallel()
        .run(|| Box::new(handle_dir_entry));

    let data = data_locked.read().expect("unlock");

    let (_w, h) = term_size::dimensions().unwrap_or((80, 40));
    let top_n = h - 2; // TODO this fails for tiniest terminals

    let cols: Vec<Cell> = data
        .entries
        .iter()
        .map(|e| Cell::new(&e.display_histogram(top_n)))
        .collect();

    let mut table = Table::new();
    table.set_format(
        FormatBuilder::new()
            .borders('│')
            .padding(1, 1)
            .separators(
                &[LinePosition::Title],
                LineSeparator::new('─', '─', '├', '┤'),
            )
            .separators(&[LinePosition::Top], LineSeparator::new('─', '─', '┌', '┐'))
            .separators(
                &[LinePosition::Bottom],
                LineSeparator::new('─', '─', '└', '┘'),
            )
            .build(),
    );
    table.add_row(Row::new(cols));
    table.printstd();
}

impl EntryData {
    fn display_histogram(&self, height: usize) -> String {
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

fn tot_up(dest: &mut HashMap<Term, u64>, src: &HashMap<Term, u64>) {
    for (key, value) in src.iter() {
        *dest.entry(key.clone()).or_default() += *value;
    }
}

fn search(entry: DirEntry, data_sink: Arc<RwLock<SessionData>>) {
    if !entry.file_type().unwrap().is_file() {
        return;
    }

    let path = entry.path();
    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(r"\w{4}\w*")
        .expect("good regex");
    let collect_data = CollectData {
        matcher: matcher.clone(),
        entry_data: EntryData {
            path: path.to_path_buf(),
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

impl Sink for CollectData {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        sink_match: &SinkMatch,
    ) -> Result<bool, Self::Error> {
        let term_count = &mut self.entry_data.term_count;

        let _ = self.matcher.find_iter(sink_match.bytes(), |mat| {
            let slice = &sink_match.bytes()[mat];
            let string: &str = &String::from_utf8_lossy(slice);
            let term = Term::from(string);

            // counting the terms per file
            term_count.entry(term).and_modify(|x| *x += 1).or_insert(1);

            true
        });

        Ok(true)
    }

    fn finish(&mut self, _searcher: &Searcher, _: &SinkFinish) -> Result<(), Self::Error> {
        self.sink
            .write()
            .expect("write")
            .insert_entry_data(&self.entry_data);

        Ok(())
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
