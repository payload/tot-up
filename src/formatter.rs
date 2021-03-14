use crate::{
    entry_data::{EntryData, Term},
    session_data::SessionData,
};

use clap::arg_enum;
use prettytable::{
    format::{FormatBuilder, LinePosition, LineSeparator},
    Cell, Row, Table,
};

#[derive(Debug)]
pub struct FormatSession {
    pub count: Option<usize>,
    pub style: DisplayStyle,
}

arg_enum! {
    #[derive(Debug)]
    pub enum DisplayStyle {
        Vert,
        Hori,
        Grid,
    }
}

impl FormatSession {
    pub fn print_stdout(&self, data: &SessionData, (w, h): (usize, usize)) {
        let height = self.count.unwrap_or(h);

        let entries = data
            .root_paths
            .iter()
            .filter_map(|path| data.directories.get(path));

        match self.style {
            DisplayStyle::Vert => {
                let hists = entries.map(|entry| self.display_histogram(entry, height));
                hists.for_each(|h| println!("{}", h));
            }
            DisplayStyle::Hori => {
                let hists = entries.map(|entry| self.display_histogram(entry, height));
                let table = self.prettytable(hists.map(|hist| Cell::new(&hist)).collect());
                table.printstd();
            }
            DisplayStyle::Grid => self.print_grid(entries.collect(), w, h),
        }
    }

    fn print_grid(&self, entries: Vec<&EntryData>, width: usize, _height: usize) {
        let sorted: Vec<_> = entries
            .into_iter()
            .map(|entry| (entry.path(), entry.sorted_term_counts()))
            .collect();

        let (count, sorted) = if let Some(count) = self.count {
            (count, sorted)
        } else {
            let count = sorted
                .iter()
                .map(|(_, terms)| terms.len())
                .max()
                .unwrap_or(0);
            let mut sorted = sorted;
            sorted.sort_by_key(|(_, terms)| std::usize::MAX - terms.len());
            (count, sorted)
        };

        let formatted: Vec<Vec<String>> = sorted
            .into_iter()
            .map(|(path, terms)| self.lineformat_entry(path, &terms[0..count.min(terms.len())]))
            .collect();
        let longest = formatted
            .iter()
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);

        if longest > width / 2 {
            for entry in formatted {
                for line in entry {
                    println!("{}", line);
                }
            }
        } else {
            let panels = formatted.len();
            let cols = width / longest;
            let rows = panels / cols;
            let rows = if rows * cols < panels { rows + 1 } else { rows };

            for row in 0..rows {
                for line in 0..count + 1 {
                    let mut empty = true;

                    for col in 0..cols {
                        let index = row * cols + col;
                        if let Some(lines) = formatted.get(index) {
                            if let Some(line) = lines.get(line) {
                                print!("{:width$}", line, width = longest);
                                empty = false;
                            }
                        }
                    }

                    if !empty {
                        println!();
                    }
                }
            }
        }
    }

    fn lineformat_entry(&self, path: &str, terms: &[(&Term, &usize)]) -> Vec<String> {
        let mut lines = Vec::with_capacity(1 + terms.len());
        lines.push(String::with_capacity(path.len()));

        let max_count = terms.first().map_or(0, |t| *t.1);
        lines.extend(
            terms
                .into_iter()
                .map(|(term, &count)| self.format_term_count(max_count, &term, count)),
        );

        let longest = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let chars = path.chars().count();
        lines[0].extend(path.chars().skip(chars.saturating_sub(longest)));
        lines[0].push(' ');

        lines
    }

    fn format_term_count(&self, max_count: usize, term: &Term, count: usize) -> String {
        format!(
            "{} {} {}",
            pct_to_bar(count as f64 / max_count as f64, 10),
            count,
            term
        )
    }

    fn display_histogram(&self, data: &EntryData, height: usize) -> String {
        // self.path
        // ... bars count term
        let line_one = Some(format!("{}:\n", data.path())).into_iter();

        let term_counts = data.sorted_term_counts();
        let max_count = term_counts.first().map(|e| *e.1 as f64).unwrap_or_default();

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
