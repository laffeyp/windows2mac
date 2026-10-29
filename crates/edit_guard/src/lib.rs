// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Multi-line edits to the project's files go through the Edit tool, whose old and new text is checked against the
//! file before it lands (WORKING_AGREEMENT; the Drift watchlist). Sprints 189, 191 and 194 each made one with the
//! shell instead, a third within three sprints, so the rule became this check (Sprint 195): `refusal` names a Bash
//! command that writes a multi-line change to a file outside the temporary directories, by a heredoc redirected or
//! piped to `tee`, or by `sed -i` with `a\`, `i\` or `c\`, an address range, or a newline in its script.

/// Paths a command may write freely: the temporary directories and devices.
const FREE: [&str; 4] = ["/tmp/", "/private/tmp/", "/var/folders/", "/dev/"];

/// The command's words, as a POSIX shell splits them, with each quoted span kept whole; `;`, `|`, `&&`, `||`, `>` and
/// `>>` are words of their own when unquoted.
pub fn words(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                started = true;
                for d in chars.by_ref() {
                    if d == '\'' {
                        break;
                    }
                    word.push(d);
                }
            }
            '"' => {
                started = true;
                while let Some(d) = chars.next() {
                    match d {
                        '"' => break,
                        '\\' => {
                            if let Some(&e) = chars.peek()
                                && matches!(e, '"' | '\\' | '$' | '`')
                            {
                                word.push(e);
                                chars.next();
                            } else {
                                word.push('\\');
                            }
                        }
                        _ => word.push(d),
                    }
                }
            }
            '\\' => {
                started = true;
                if let Some(d) = chars.next()
                    && d != '\n'
                {
                    word.push(d);
                }
            }
            ' ' | '\t' | '\n' | ';' | '|' | '&' | '>' | '<' => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
                if !matches!(c, ' ' | '\t') {
                    let mut op = c.to_string();
                    while let Some(&d) = chars.peek() {
                        if d == c || (c == '>' && d == '|') {
                            op.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    out.push(op);
                }
            }
            _ => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        out.push(word);
    }
    out
}

/// The command with each heredoc's body removed (the lines after a `<<WORD` line up to the line WORD), so the body's
/// text is never read as shell words.
pub fn without_heredoc_bodies(command: &str) -> String {
    let mut out = String::new();
    let mut ends: Vec<(String, bool)> = Vec::new();
    for line in command.split('\n') {
        if let Some((end, tabs)) = ends.first() {
            let l = if *tabs { line.trim_start_matches('\t') } else { line };
            if l == end {
                ends.remove(0);
            }
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find("<<") {
            rest = &rest[i + 2..];
            if rest.starts_with('<') {
                rest = &rest[1..];
                continue;
            }
            let tabs = rest.starts_with('-');
            let r = rest.trim_start_matches('-').trim_start();
            let end: String = r
                .chars()
                .filter(|c| !matches!(c, '\'' | '"' | '\\'))
                .take_while(|c| !c.is_whitespace() && !matches!(c, ';' | '|' | '&' | '>' | '<' | ')'))
                .collect();
            if !end.is_empty() {
                ends.push((end, tabs));
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Words that end a simple command.
fn ends_command(word: &str) -> bool {
    matches!(word, ";" | "&&" | "||" | "&" | "\n")
}

fn free(path: &str) -> bool {
    FREE.iter()
        .any(|f| path.starts_with(f) || path == f.trim_end_matches('/'))
}

/// A `sed` script that changes more than one line: `a\`, `i\` or `c\`, an address range, a newline, or a `\n` escape,
/// which macOS's sed writes as a newline in a replacement (Sprint 212: Sprint 211's substitution made two lines one
/// each by `\n`, and the classifier passed it).
fn multi_line_script(script: &str) -> Option<&'static str> {
    if script.contains('\n') {
        return Some("a newline in its script");
    }
    let bytes = script.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' {
            if bytes[i + 1] == b'n' {
                return Some("a \\n escape in its script");
            }
            i += 2; // an escaped character, `\\` among them
            continue;
        }
        i += 1;
    }
    for command in script.split(';') {
        let command = command.trim_start();
        // the address: a line number, `$`, or a /regex/, then an optional ",address"
        let mut rest = command;
        let mut range = false;
        for part in 0..2 {
            let skipped = if let Some(regex) = rest.strip_prefix('/') {
                regex.find('/').map(|i| i + 2)
            } else {
                Some(
                    rest.find(|c: char| !(c.is_ascii_digit() || c == '$'))
                        .unwrap_or(rest.len()),
                )
            };
            let Some(n) = skipped else { break };
            if n == 0 {
                break;
            }
            rest = rest[n..].trim_start();
            if part == 0 && rest.starts_with(',') {
                range = true;
                rest = &rest[1..];
            } else {
                break;
            }
        }
        if range && !rest.starts_with('p') {
            return Some("an address range");
        }
        let mut cs = rest.chars();
        if let (Some(c), Some('\\')) = (cs.next(), cs.next())
            && matches!(c, 'a' | 'i' | 'c')
        {
            return Some("an a\\, i\\ or c\\ command");
        }
    }
    None
}

/// Why `command` is refused, or None.
pub fn refusal(command: &str) -> Option<String> {
    let w = words(&without_heredoc_bodies(command));
    for (k, word) in w.iter().enumerate() {
        // a heredoc whose text goes to a file: a redirection in the same simple command, or `tee FILE` in its pipe
        if word.starts_with("<<") && !word.starts_with("<<<") {
            let end = w[k..].iter().position(|x| ends_command(x)).map_or(w.len(), |e| k + e);
            let start = w[..k].iter().rposition(|x| ends_command(x)).map_or(0, |i| i + 1);
            let span = &w[start..end];
            for (i, x) in span.iter().enumerate() {
                let target = match x.as_str() {
                    ">" | ">>" | ">|" => span.get(i + 1),
                    "tee" => span[i + 1..].iter().find(|t| !t.starts_with('-')),
                    _ => None,
                };
                if let Some(t) = target
                    && !free(t)
                {
                    return Some(format!(
                        "a heredoc written to {t}: a multi-line edit goes through the Edit tool (Write for a new file)"
                    ));
                }
            }
        }
        // sed -i with a multi-line script
        if word == "sed" || word.ends_with("/sed") {
            let args: Vec<&String> = w[k + 1..]
                .iter()
                .take_while(|x| !ends_command(x) && x.as_str() != "|")
                .collect();
            if !args.iter().any(|a| a.starts_with("-i") || *a == "--in-place") {
                continue;
            }
            let mut scripts = Vec::new();
            let mut i = 0;
            while i < args.len() {
                let a = args[i].as_str();
                if a == "-e" {
                    if let Some(s) = args.get(i + 1) {
                        scripts.push(s.as_str());
                    }
                    i += 2;
                    continue;
                }
                i += 1;
            }
            if scripts.is_empty() {
                // the first word that is not an option or the -i suffix's ''
                let mut skip_next = false;
                for (j, a) in args.iter().enumerate() {
                    if skip_next {
                        skip_next = false;
                        continue;
                    }
                    if *a == "-i" && args.get(j + 1).is_some_and(|n| n.is_empty()) {
                        skip_next = true;
                        continue;
                    }
                    if a.starts_with('-') || a.is_empty() {
                        continue;
                    }
                    scripts.push(a.as_str());
                    break;
                }
            }
            for s in scripts {
                if let Some(why) = multi_line_script(s) {
                    return Some(format!(
                        "sed -i with {why}: a multi-line edit goes through the Edit tool"
                    ));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::refusal;

    #[test]
    fn a_heredoc_appended_to_a_project_file_is_refused() {
        let r = refusal("cat >> crates/w2m_vm/src/built_from.rs <<'EOF'\nmod tests {}\nEOF");
        assert!(r.unwrap().contains("crates/w2m_vm/src/built_from.rs"));
        assert!(refusal("cd /x && cat <<EOF > notes.md\na\nb\nEOF").is_some());
        assert!(refusal("cat <<EOF | tee -a BLACKBOARD.md\na\nEOF").is_some());
    }

    #[test]
    fn a_heredoc_to_a_temporary_file_or_a_program_is_allowed() {
        assert!(refusal("cat > /private/tmp/claude-501/x/scratchpad/a.txt <<EOF\na\nb\nEOF").is_none());
        assert!(refusal("git commit -F - <<'EOF'\nsubject\n\nbody\nEOF").is_none());
        // a body's text is not shell: "> f" and "sed -i '1,2d'" inside it are words of a message
        assert!(refusal("git commit -F - <<'EOF' && git push\na > f\nsed -i '' '1,2d' x\nEOF").is_none());
        assert!(refusal("grep -c x <<< \"$text\" > /dev/null").is_none());
    }

    #[test]
    fn sed_append_insert_change_range_and_newline_are_refused() {
        assert!(refusal("sed -i '' '32a\\\ncalls in flight\t1\n' store/x.tsv").is_some());
        // GNU sed's one-line form, no newline in the script (Sprint 195's plant 3 passed until this case)
        assert!(
            refusal("sed -i '$a\\ last line' f.rs")
                .unwrap()
                .contains("a\\, i\\ or c\\")
        );
        assert!(refusal("sed -i '' '5i\\ text' f.rs").is_some());
        assert!(refusal("sed -i '' '/^x/c\\ y' f.rs").is_some());
        assert!(
            refusal("sed -i '' '695,702d' BLACKBOARD.md")
                .unwrap()
                .contains("address range")
        );
        assert!(refusal("sed -i '' '/^### A/,/^### B/d' BLACKBOARD.md").is_some());
        assert!(refusal("sed -i '' 's/a/b\\\nc/' f.rs").is_some());
        // a `\n` escape in a replacement: macOS's sed writes a newline (Sprint 212)
        assert!(
            refusal("sed -i '' 's/^x$/x\\n        y/' f.rs")
                .unwrap()
                .contains("\\n escape")
        );
        // an escaped backslash before n is a backslash and an n
        assert!(refusal("sed -i '' 's/a/b\\\\n/' f.rs").is_none());
    }

    #[test]
    fn one_line_seds_and_reading_seds_are_allowed() {
        assert!(refusal("sed -i '' 's/^status: active$/status: done/' sprints/x.md").is_none());
        assert!(refusal("sed -i '' '623s/gates running)/closed)/' BLACKBOARD.md").is_none());
        assert!(refusal("sed -i '' '/^pub const A: u64 = 1;$/d; /^pub const B: u64 = 2;$/d' p.rs").is_none());
        assert!(refusal("sed -n 1,40p f.rs").is_none());
        assert!(refusal("sed -n '/a/,/b/p' f.rs").is_none());
    }
}
