use std::{collections::HashMap, path::PathBuf};

use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{DirEntry, WalkBuilder};

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
    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(r"\w{3}\w*")
        .expect("good regex");
    let collect_data = CollectData {
        matcher: matcher.clone(),
        path: path.to_owned(),
        term_count: Default::default(),
    };

    grep::searcher::Searcher::new()
        .search_path(matcher, path, collect_data)
        .expect("search path");
}

struct CollectData {
    matcher: RegexMatcher,
    path: PathBuf,
    term_count: HashMap<String, u64>,
}

impl Sink for CollectData {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        sink_match: &SinkMatch,
    ) -> Result<bool, Self::Error> {
        let term_count = &mut self.term_count;

        let _ = self.matcher.find_iter(sink_match.bytes(), |mat| {
            let slice = &sink_match.bytes()[mat];
            let term = String::from_utf8_lossy(slice);

            if let Some(count) = term_count.get_mut(term.as_ref()) {
                *count += 1;
            } else {
                term_count.insert(term.to_string(), 1);
            }

            true
        });

        Ok(true)
    }

    fn finish(&mut self, _searcher: &Searcher, _: &SinkFinish) -> Result<(), Self::Error> {
        println!("{}:", self.path.display());

        let mut term_counts: Vec<_> = self.term_count.drain().collect();
        term_counts.sort_by_key(|entry| entry.1);

        println!(" terms: {}", term_counts.len());
        println!(" top 5:");
        for (term, count) in term_counts[term_counts.len().saturating_sub(5)..].iter() {
            println!("  {} {}", term, count);
        }

        Ok(())
    }
}
