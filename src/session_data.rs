use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::entry_data::{EntryData, Term};

#[derive(Default)]
pub struct SessionData {
    pub term_regex: String,
    pub root_path: String,
    pub entries: Vec<EntryData>,
    pub terms: HashSet<Term>,
    pub directories: HashMap<String, EntryData>,
}

impl SessionData {
    pub fn insert_entry_data(&mut self, entry: EntryData) {
        #[cfg(feature = "record-terms")]
        for (key, _value) in data.term_count.iter() {
            self.terms.insert(key.clone());
        }

        self.entries.push(entry.clone());

        // You have a path ./foo/bar/baz.txt /home/x/y/z.txt
        // and a root_path like ., ./foo or /home/x
        // and you want to insert a new entry of term counts
        // and sum up all the entries of the directories below.
        //
        // Probably you don't even want that here, just eventually you need some sum somewhere.
        // This is the first solution, which worked. Probably there are better ones.

        let root_path = Path::new(&self.root_path);
        let mut path = Path::new(entry.path());
        let recursive_paths = Some(path).into_iter().chain(std::iter::from_fn(|| {
            if path != root_path {
                path = path.parent().unwrap();
                Some(path)
            } else {
                None
            }
        }));

        for path in recursive_paths {
            let key = path.to_string_lossy();
            self.directories
                .entry(key.to_string())
                .or_insert_with(|| EntryData::new(&key))
                .tot_up(&entry);
        }
    }
}
