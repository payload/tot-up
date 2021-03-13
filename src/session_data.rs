use std::collections::HashSet;

use crate::entry_data::{EntryData, Term};

#[derive(Default)]
pub struct SessionData {
    pub root_path: String,
    pub entries: Vec<EntryData>,
    pub terms: HashSet<Term>,
}
