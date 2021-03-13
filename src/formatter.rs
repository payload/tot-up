use crate::{entry_data::EntryData, session_data::SessionData};

use prettytable::{
    format::{FormatBuilder, LinePosition, LineSeparator},
    Cell, Row, Table,
};

pub struct FormatSession;

impl FormatSession {
    pub fn new() -> Self {
        Self
    }

    pub fn print_stdout(&self, data: &SessionData, (_w, h): (usize, usize)) {
        // filter, tot up, sort, limit, format & print
        let cells = data
            .entries
            .iter()
            .take(3)
            .map(|e| Cell::new(&self.display_histogram(e, h)));
        let table = self.prettytable(cells.collect());
        table.printstd();
    }

    fn display_histogram(&self, data: &EntryData, height: usize) -> String {
        // self.path
        // ... bars count term
        let line_one = Some(format!("{}:\n", data.path())).into_iter();

        let term_counts = data.sorted_term_counts();
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

    fn prettytable(&self, cells: Vec<Cell>) -> Table {
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
        table.add_row(Row::new(cells));
        table
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
