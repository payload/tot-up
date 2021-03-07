use std::{collections::HashMap, io::stdin, path::PathBuf, sync::{Arc, RwLock}};

use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{Searcher, Sink, SinkFinish, SinkMatch},
};
use ignore::{DirEntry, WalkBuilder, WalkState};

fn main() {
    let data: Vec<Data> = Vec::new();
    let data_locked = Arc::new(RwLock::new(data));

    let handle_dir_entry = |result| {
        match result {
            Ok(entry) => search(entry, data_locked.clone()),
            Err(err) => eprintln!("ERROR: {}", err),
        }

        WalkState::Continue
    };

    WalkBuilder::new("./")
        .build_parallel()
        .run(|| Box::new(handle_dir_entry));

    let data = data_locked.read().expect("unlock");


    println!("visited files {}", data.len());

    if true {
        let (w, h) = term_size::dimensions().unwrap_or((80, 40));

        let mut sum = Default::default();

        for entry in data.iter() {
            merge(&mut sum, &entry.term_count);
        }

        let top_n = h - 2; // TODO this fails for tiniest terminals
        println!("sum top {}:", top_n);

        let mut term_counts: Vec<_> = sum.iter().collect();
        term_counts.sort_by_key(|entry| entry.1);

        if let Some(max_entry) = term_counts.last() {
            let max_count = *max_entry.1 as f64;

            for (term, count) in term_counts[term_counts.len().saturating_sub(top_n)..]
                .iter()
                .rev()
            {
                let bar = pct_to_bar(**count as f64 / max_count, 10);
                println!(" {} {} {}", bar, count, term);
            }
        }

        let mut buf = String::new();
        let _ = stdin().read_line(&mut buf);
    }
}

fn merge(dest: &mut HashMap<String, u64>, src: &HashMap<String, u64>) {
    for (key, value) in src.iter() {
        *dest.entry(key.clone()).or_default() += *value;
    }
}

fn search(entry: DirEntry, data_sink: Arc<RwLock<Vec<Data>>>) {
    if !entry.file_type().unwrap().is_file() {
        return;
    }

    let path = entry.path();
    let matcher = grep::regex::RegexMatcherBuilder::new()
        .build(r"\w{3}\w*")
        .expect("good regex");
    let collect_data = CollectData {
        matcher: matcher.clone(),
        data: Data {
            path: path.to_path_buf(),
            ..Default::default()
        },
        sink: data_sink.clone(),
    };

    grep::searcher::Searcher::new()
        .search_path(matcher, path, collect_data)
        .expect("search path");
}

struct CollectData {
    matcher: RegexMatcher,
    data: Data,
    sink: Arc<RwLock<Vec<Data>>>,
}

#[derive(Clone, Debug, Default)]
struct Data {
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
        let term_count = &mut self.data.term_count;

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
        if false {
            println!("{}:", self.data.path.display());

            let mut term_counts: Vec<_> = self.data.term_count.iter().collect();
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

        self.sink.write().expect("write").push(self.data.clone());

        Ok(())
    }
}

const BARS: &[char] = &[
    ' ',
    '▏',
    '▎',
    '▍',
    '▌',
    '▋',
    '▊',
    '▉',
    '█',
];

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
