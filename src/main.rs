use std::sync::{Arc, RwLock};

use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{DirEntry, WalkBuilder, WalkState};

use prettytable::{
    format::{FormatBuilder, LinePosition, LineSeparator},
    Cell, Row, Table,
};

mod entry_data;
use entry_data::*;

// SessionData holds terms, data hierarchy, filters
// Walker iterates directories
// CollectData collects EntryData from files
// EntryData holds terms and counts for an entry
// Term is not a String, but a ArcIntern<String>, so equal strings a globally unique

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
        entry_data: EntryData::new(&path.to_string_lossy()),
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
        let entry_data = &mut self.entry_data;

        let _ = self.matcher.find_iter(sink_match.bytes(), |mat| {
            let slice = &sink_match.bytes()[mat];
            let string: &str = &String::from_utf8_lossy(slice);
            entry_data.inc_term(string);
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
