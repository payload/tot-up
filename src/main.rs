use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use formatter::{DisplayStyle, FormatSession};
use grep::{
    matcher::{Captures, Matcher},
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{WalkBuilder, WalkState};

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

    #[structopt(short, long, default_value = r"\w{4,}")]
    term: String,

    #[structopt(short, long, default_value = r"^$")]
    exclude: String,

    root_paths: Vec<String>,

    #[structopt(long, possible_values = &DisplayStyle::variants(), case_insensitive = true, default_value = "grid")]
    display_style: DisplayStyle,
}

struct CollectData {
    exclude: RegexMatcher,
    matcher: RegexMatcher,
    entry_data: Option<EntryData>,
    sink: Arc<RwLock<SessionData>>,
}

fn main() {
    let opt: Opt = Opt::from_args();

    let chars: &[_] = &['\\', '/'];
    let root_paths: Vec<String> = opt
        .root_paths
        .iter()
        .map(|p| p.trim_end_matches(chars).to_owned())
        .collect();

    let data = SessionData {
        root_paths: root_paths.clone(),
        ..Default::default()
    };

    let data_locked = Arc::new(RwLock::new(data));
    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(&opt.term)
        .expect("term regex");
    let exclude = grep::regex::RegexMatcherBuilder::new()
        .build(&opt.exclude)
        .expect("exclude regex");

    for root_path in root_paths.iter() {
        let root_path = Path::new(root_path);
        match std::fs::metadata(root_path) {
            Ok(m) if m.is_file() => grep_file(
                root_path,
                matcher.clone(),
                exclude.clone(),
                data_locked.clone(),
            ),
            Ok(m) if m.is_dir() => walk_dir_and_grep_files(
                root_path,
                matcher.clone(),
                exclude.clone(),
                data_locked.clone(),
            ),
            Ok(_m) => (),
            Err(_e) => (),
        }
    }

    let data = data_locked.read().expect("unlock");

    let formatter = FormatSession {
        count: opt.count,
        style: opt.display_style,
    };
    formatter.print_stdout(&data, term_size::dimensions().unwrap_or((80, 40)));
}

fn walk_dir_and_grep_files(
    root_path: &Path,
    matcher: RegexMatcher,
    exclude: RegexMatcher,
    data: Arc<RwLock<SessionData>>,
) {
    WalkBuilder::new(root_path)
        // .threads(8) // TODO does it really use all cores?
        .build_parallel()
        .run(|| {
            Box::new(|result| {
                match result {
                    Ok(entry) if entry.file_type().map_or(false, |t| t.is_file()) => {
                        grep_file(entry.path(), matcher.clone(), exclude.clone(), data.clone())
                    }
                    Ok(_) => (),
                    Err(err) => eprintln!("ERROR: {}", err),
                }
                WalkState::Continue
            })
        });
}

fn grep_file(
    path: &Path,
    matcher: RegexMatcher,
    exclude: RegexMatcher,
    data: Arc<RwLock<SessionData>>,
) {
    let collect_data = CollectData {
        exclude: exclude,
        matcher: matcher.clone(),
        entry_data: Some(EntryData::new(&path.to_string_lossy())),
        sink: data.clone(),
    };

    grep::searcher::Searcher::new()
        .search_path(matcher, path, collect_data)
        .expect("grep Searcher::search_path");
}

impl Sink for CollectData {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        sink_match: &SinkMatch,
    ) -> Result<bool, Self::Error> {
        let entry_data = self.entry_data.as_mut().unwrap();
        let exclude = &self.exclude;

        // Here we use matcher.captures_iter to optional use the capture group 1
        // to skip/filter all the matching characters at the boundary.
        // For example regex (\w+)[.?!] finds words at the end of sentences,
        // but ignores the end sign when capturing and counting.
        let mut captures = self.matcher.new_captures().expect("CollectData captures");

        let _ = self
            .matcher
            .captures_iter(sink_match.bytes(), &mut captures, |caps| {
                let mat = caps.get(1).unwrap_or_else(|| caps.get(0).unwrap());
                let slice = &sink_match.bytes()[mat];
                let string: &str = &String::from_utf8_lossy(slice);

                if !exclude.is_match(slice).unwrap_or(false) {
                    entry_data.inc_term(string);
                }

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
