use std::sync::{Arc, RwLock};

use formatter::{DisplayStyle, FormatSession};
use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{DirEntry, WalkBuilder, WalkState};

use structopt::StructOpt;

// SessionData holds terms, data hierarchy, filters
// Walker iterates directories
// CollectData collects EntryData from files
// EntryData holds terms and counts for an entry
// Term is not a String, but a ArcIntern<String>, so equal strings a globally unique
mod entry_data;
mod formatter;
mod session_data;
use entry_data::*;
use session_data::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "totup")]
struct Opt {
    #[structopt(short = "n", long)]
    count: Option<usize>,

    #[structopt(short, long, default_value = r"\w{4}\w*")]
    term: String,

    root_path: String,

    #[structopt(long, possible_values = &DisplayStyle::variants(), case_insensitive = true, default_value = "histograms")]
    display_style: DisplayStyle,
}

struct CollectData {
    matcher: RegexMatcher,
    entry_data: Option<EntryData>,
    sink: Arc<RwLock<SessionData>>,
}

fn main() {
    let opt = Opt::from_args();

    let root_path = std::env::args().nth(1).unwrap_or("./".into());
    let data = SessionData {
        term_regex: opt.term,
        root_path: root_path.clone(),
        ..Default::default()
    };

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

    let formatter = FormatSession {
        count: opt.count,
        style: opt.display_style,
    };
    formatter.print_stdout(&data, term_size::dimensions().unwrap_or((80, 40)));
}

fn search(entry: DirEntry, data_sink: Arc<RwLock<SessionData>>) {
    if !entry.file_type().unwrap().is_file() {
        return;
    }

    let term_regex = data_sink
        .read()
        .expect("search session data read")
        .term_regex
        .clone();
    let path = entry.path();
    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(&term_regex)
        .expect("good regex");
    let collect_data = CollectData {
        matcher: matcher.clone(),
        entry_data: Some(EntryData::new(&path.to_string_lossy())),
        sink: data_sink.clone(),
    };

    grep::searcher::Searcher::new()
        .search_path(matcher, path, collect_data)
        .expect("search path");
}

impl Sink for CollectData {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        sink_match: &SinkMatch,
    ) -> Result<bool, Self::Error> {
        let entry_data = self.entry_data.as_mut().unwrap();

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
            .expect("collect data sink write")
            .insert_entry_data(self.entry_data.take().unwrap());
        Ok(())
    }
}
