//! The standard is embedded only in the `i` executable.
use std::io::{IsTerminal, Write};
use std::process::{Command, ExitCode, Stdio};

// The coordinator will move this file to /SPEC.md; change this one path then.
const STANDARD: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/spec.md"));

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("selection").args(["section", "section_option", "search", "toc", "version"]).multiple(false)))]
pub struct SpecArgs {
    /// Show a numbered section and its subsections.
    section: Option<String>,
    /// Show a numbered section and its subsections.
    #[arg(long = "section")]
    section_option: Option<String>,
    /// Find sections containing all these words (case insensitive).
    #[arg(long, num_args = 1..)]
    search: Vec<String>,
    /// Show the table of contents.
    #[arg(long)]
    toc: bool,
    /// Show the standard's version and date.
    #[arg(long)]
    version: bool,
    /// Print Markdown even when stdout is a terminal.
    #[arg(long)]
    raw: bool,
}

struct Heading<'a> {
    number: &'a str,
    title: &'a str,
    start: usize,
    end: usize,
}

fn headings(text: &str) -> Vec<Heading<'_>> {
    let mut result = Vec::new();
    let mut offset = 0;
    let mut fence: Option<(char, usize)> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(ch @ ('`' | '~')) = trimmed.chars().next() {
            let count = trimmed.chars().take_while(|&c| c == ch).count();
            if count >= 3 {
                match fence {
                    None => fence = Some((ch, count)),
                    Some((open, n)) if ch == open && count >= n => fence = None,
                    _ => {}
                }
                offset += line.len();
                continue;
            }
        }
        if fence.is_none() {
            let level = line.bytes().take_while(|&c| c == b'#').count();
            if (2..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
                let title = line[level + 1..].trim();
                let first = title.split_whitespace().next().unwrap_or("");
                let number = if first == "Appendix" {
                    title
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .trim_end_matches('.')
                } else {
                    first.trim_end_matches('.')
                };
                if number.starts_with(|c: char| c.is_ascii_digit())
                    || (number.len() == 1 && number.starts_with(|c: char| c.is_ascii_uppercase()))
                    || number.starts_with(|c: char| c.is_ascii_uppercase()) && number.contains('.')
                {
                    result.push(Heading {
                        number,
                        title,
                        start: offset,
                        end: text.len(),
                    });
                }
            }
        }
        offset += line.len();
    }
    for i in 0..result.len() {
        if let Some(next) = result[i + 1..].iter().find(|h| {
            !h.number
                .strip_prefix(result[i].number)
                .is_some_and(|rest| rest.starts_with('.'))
        }) {
            result[i].end = next.start;
        }
    }
    result
}

fn table_of_contents(text: &str) -> String {
    let mut output = String::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(ch @ ('`' | '~')) = trimmed.chars().next() {
            let count = trimmed.chars().take_while(|&c| c == ch).count();
            if count >= 3 {
                match fence {
                    None => fence = Some((ch, count)),
                    Some((open, n)) if ch == open && count >= n => fence = None,
                    _ => {}
                }
                continue;
            }
        }
        let level = line.bytes().take_while(|&c| c == b'#').count();
        if fence.is_none() && (2..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ')
        {
            output.push_str(&"  ".repeat(level - 2));
            output.push_str(line[level + 1..].trim());
            output.push('\n');
        }
    }
    output
}

fn distance(a: &str, b: &str) -> usize {
    let mut row: Vec<usize> = (0..=b.chars().count()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j + 1] + 1)
                .min(row[j] + 1)
                .min(previous + usize::from(ca != cb));
            previous = old;
        }
    }
    *row.last().unwrap()
}

fn render(text: &str) -> String {
    let mut out = String::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(ch @ ('`' | '~')) = trimmed.chars().next() {
            let count = trimmed.chars().take_while(|&c| c == ch).count();
            if count >= 3 {
                match fence {
                    None => {
                        fence = Some((ch, count));
                        continue;
                    }
                    Some((open, n)) if ch == open && count >= n => {
                        fence = None;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        if fence.is_some() {
            out.push_str("    ");
            out.push_str(line);
        } else if line.starts_with('#') && line.trim_start_matches('#').starts_with(' ') {
            out.push_str("\x1b[1m");
            out.push_str(
                line.trim_end_matches('\n')
                    .trim_start_matches('#')
                    .trim_start(),
            );
            out.push_str("\x1b[0m\n");
        } else {
            out.push_str(line);
        }
    }
    out
}

fn display(text: &str, raw: bool) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    if raw || !stdout.is_terminal() {
        stdout.write_all(text.as_bytes())?;
        return Ok(());
    }
    let rendered = render(text);
    let pager = std::env::var("PAGER")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "less -R".into());
    // The shell permits conventional PAGER values containing arguments.
    if let Ok(mut child) = Command::new("sh")
        .arg("-c")
        .arg(format!("exec {pager}"))
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        let written = child.stdin.take().unwrap().write_all(rendered.as_bytes());
        let status = child.wait()?;
        if status.success()
            || (status.code() != Some(127)
                && written
                    .as_ref()
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe))
        {
            return Ok(());
        }
    }
    stdout.write_all(rendered.as_bytes())?;
    Ok(())
}

pub fn run(args: SpecArgs) -> anyhow::Result<ExitCode> {
    let headings = headings(STANDARD);
    let output = if let Some(section) = args.section.or(args.section_option) {
        if let Some(h) = headings.iter().find(|h| h.number == section) {
            STANDARD[h.start..h.end].to_owned()
        } else {
            let mut nearest: Vec<_> = headings.iter().collect();
            nearest.sort_by_key(|h| distance(&section, h.number));
            anyhow::bail!(
                "unknown section {section:?}; nearest headings:\n{}",
                nearest
                    .iter()
                    .take(5)
                    .map(|h| h.title)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    } else if !args.search.is_empty() {
        let words: Vec<_> = args
            .search
            .iter()
            .flat_map(|s| s.split_whitespace())
            .map(str::to_lowercase)
            .collect();
        let mut output = String::new();
        for (i, h) in headings.iter().enumerate() {
            // Attribute each line to its most specific section, not every ancestor.
            let end = headings
                .get(i + 1)
                .map_or(STANDARD.len(), |next| next.start);
            let body = &STANDARD[h.start..end];
            if words.iter().all(|w| body.to_lowercase().contains(w)) {
                output.push_str(h.title);
                output.push('\n');
                for line in body.lines().skip(1).filter(|line| {
                    let line = line.to_lowercase();
                    words.iter().any(|w| line.contains(w))
                }) {
                    output.push_str("  ");
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }
        output
    } else if args.toc {
        table_of_contents(STANDARD)
    } else if args.version {
        STANDARD
            .lines()
            .find(|line| line.starts_with("Version: "))
            .ok_or_else(|| anyhow::anyhow!("standard has no version header"))?
            .to_owned()
            + "\n"
    } else {
        STANDARD.to_owned()
    };
    display(&output, args.raw)?;
    Ok(ExitCode::SUCCESS)
}
