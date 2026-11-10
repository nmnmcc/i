//! Header-only regeneration for A2. Source after the header stays byte-identical.

pub fn rewrite(source: &str, comment: &str, first: &str, at: &[String]) -> Result<String, String> {
    for value in std::iter::once(first).chain(at.iter().map(String::as_str)) {
        if value.trim() != value || value.contains(['\r', '\n']) {
            return Err(
                "a regenerated header value has leading/trailing whitespace or a line break".into(),
            );
        }
    }
    let mut out = String::new();
    let mut rest = source;
    let mut position = 0;
    let mut stop = false;
    while !rest.is_empty() {
        let end = rest.find('\n').map_or(rest.len(), |i| i + 1);
        let line = &rest[..end];
        let (text, ending) = if let Some(s) = line.strip_suffix("\r\n") {
            (s, "\r\n")
        } else if let Some(s) = line.strip_suffix('\n') {
            (s, "\n")
        } else {
            (line, "")
        };
        let header = match text.strip_prefix(comment) {
            Some(s) => s.trim(),
            None if text.trim().is_empty() => "",
            None => {
                out.push_str(rest);
                break;
            }
        };
        if header.starts_with("expect: stop ") {
            if stop {
                return Err("the header has more than one stop expectation".into());
            }
            stop = true;
            out.push_str(&format!("{comment} expect: stop \"{first}\"{ending}"));
        } else if header.starts_with("at:") {
            let value = at
                .get(position)
                .ok_or("the number of stop positions changed")?;
            out.push_str(&format!("{comment} at: {value}{ending}"));
            position += 1;
        } else {
            out.push_str(line);
        }
        rest = &rest[end..];
    }
    if !stop || position != at.len() {
        return Err("the stop header or its number of positions changed".into());
    }
    Ok(out)
}
