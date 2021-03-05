use std::{collections::HashMap, path::{Path, PathBuf}};

use grep::{matcher::Matcher, regex::RegexMatcher, searcher::{Searcher, SearcherBuilder, Sink, SinkError, SinkMatch}};
use ignore::{DirEntry, Walk, WalkBuilder};

fn main() {
    for result in WalkBuilder::new("./").build() {
        match result {
            Ok(entry) => search(entry),
            Err(err) => eprintln!("ERROR: {}", err),
        }
    }
}

fn search(entry: DirEntry) {
    if !entry.file_type().unwrap().is_file() {
        return;
    }

    let path = entry.path();

    println!("{}", path.display());

    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(r"\w{3}\w*")
        .expect("good regex");

    let search_writer = grep::cli::stdout(termcolor::ColorChoice::Auto);
    let mut search_printer = grep::printer::Standard::new(search_writer);
    let search_sink = search_printer.sink_with_path(matcher.clone(), path);

    let collect_data = CollectData {
        matcher: matcher.clone(),
        path: path.to_owned(),
    };

    grep::searcher::Searcher::new().search_path(matcher, path, collect_data).expect("search path");
}

struct CollectData {
    matcher: RegexMatcher,
    path: PathBuf,
}

impl Sink for CollectData {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        sink_match: &SinkMatch,
    ) -> Result<bool, Self::Error> {
        let mut matches = vec![];

        let _ = self.matcher.find_iter(sink_match.bytes(), |mat| {
            let slice = &sink_match.bytes()[mat];
            let str = String::from_utf8_lossy(slice);
            matches.push(str);
            true
        });

        for mat in matches {
            println!("{} {}", self.path.display(), mat);
        }

        Ok(true)
    }
}
