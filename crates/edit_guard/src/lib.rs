// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Multi-line edits to the project's files go through the Edit tool, whose old and new text is checked against the
//! file before it lands (WORKING_AGREEMENT; the Drift watchlist). Sprints 189, 191 and 194 each made one with the
//! shell instead, a third within three sprints, so the rule became this check (Sprint 195): `refusal` names a Bash
//! command that writes a multi-line change to a file outside the temporary directories, by a heredoc redirected or
//! piped to `tee`, or by `sed -i` with any command that changes the line count or writes another file (Sprint 277:
//! `a`, `i`, `c`, `r`, `R`, `G`, `N`, `D`, `p`, `P`, `=`, `l`, `F`, `q`, `Q`, `w`, `W`, `e`, `!`, an `s` flag `p`,
//! `w` or `e`, an address range), a newline in its script, `-n` or `-f`. Sprint 267
//! (reviewer-3's Medium 1 at 266: Python started in Sprints 238, 240, 250 and 266, shell edits in 262, 263 and 266)
//! adds an in-place `perl` or `awk` edit, and any start of a Python interpreter or installer, the user's Decision "No
//! Python runs", by a command's program and not by a word that only names it.

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

/// A `sed -i` script that changes more than one line: a newline, or a `\n` escape, which macOS's sed writes as a
/// newline in a replacement (Sprint 212: Sprint 211's substitution made two lines one each by `\n`, and the classifier
/// passed it); an address range; or a command that adds, joins, prints, reads or cuts lines, or writes another file
/// (Sprint 277).
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
    // each command, inside braces too: [address[,address]][!]command[arguments], as `man sed` (BSD) and GNU sed's
    // manual set them (Sprint 277: `343r file` passed the split on `;`, and so did a command inside braces)
    let s: Vec<char> = script.chars().collect();
    let mut i = 0;
    while i < s.len() {
        if s[i].is_whitespace() || matches!(s[i], ';' | '{' | '}') {
            i += 1;
            continue;
        }
        let mut addresses = 0;
        loop {
            let start = i;
            if s[i] == '/' || s[i] == '\\' {
                // a /regex/, or \cregexc
                let delimiter = if s[i] == '\\' {
                    i += 1;
                    *s.get(i)?
                } else {
                    '/'
                };
                i += 1;
                while i < s.len() && s[i] != delimiter {
                    i += if s[i] == '\\' { 2 } else { 1 };
                }
                i += 1;
                while i < s.len() && matches!(s[i], 'I' | 'M') {
                    i += 1;
                }
            } else {
                while i < s.len() && (s[i].is_ascii_digit() || matches!(s[i], '$' | '~' | '+')) {
                    i += 1;
                }
            }
            if i == start {
                break;
            }
            addresses += 1;
            while i < s.len() && s[i].is_whitespace() {
                i += 1;
            }
            if addresses == 1 && s.get(i) == Some(&',') {
                i += 1;
                continue;
            }
            break;
        }
        while i < s.len() && s[i].is_whitespace() {
            i += 1;
        }
        let Some(&c) = s.get(i) else { break };
        if c == '!' {
            return Some("a ! command, which acts on every line but its address's");
        }
        if c == '{' {
            if addresses == 2 {
                return Some("an address range");
            }
            i += 1;
            continue;
        }
        if addresses == 2 && c != 'p' {
            return Some("an address range");
        }
        match c {
            'a' | 'i' | 'c' => return Some("an a, i or c command"),
            'r' | 'R' => return Some("an r or R command, which reads another file's lines in"),
            'G' | 'N' | 'D' => return Some("a G, N or D command, which adds or joins lines"),
            'p' | 'P' | '=' | 'l' | 'F' => return Some("a p, P, =, l or F command, which prints a line into the file"),
            'q' | 'Q' => return Some("a q or Q command, which cuts the file"),
            'w' | 'W' | 'e' => return Some("a w, W or e command, which writes another file or runs one"),
            's' | 'y' => {
                // three delimiters, then for `s` its flags
                let delimiter = *s.get(i + 1)?;
                i += 2;
                let mut seen = 1;
                while i < s.len() && seen < 3 {
                    if s[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if s[i] == delimiter {
                        seen += 1;
                    }
                    i += 1;
                }
                let flags_start = i;
                while i < s.len() && !matches!(s[i], ';' | '}' | '\n') {
                    i += 1;
                }
                let flags: String = s[flags_start..i].iter().collect();
                if c == 's' && flags.chars().any(|f| matches!(f, 'p' | 'w' | 'e')) {
                    return Some("an s command with a p, w or e flag");
                }
            }
            _ => {
                // b, t and : take a label; d, g, h, H, n, x, z and the rest are one character
                i += 1;
                if matches!(c, 'b' | 't' | 'T' | ':') {
                    while i < s.len() && !matches!(s[i], ';' | '}') {
                        i += 1;
                    }
                }
            }
        }
    }
    None
}

/// A Python interpreter or installer by its program name: `python`, `python2`, `python3`, `python3.<n>`, `pip`, `pip3`
/// (the user's Decision "No Python runs"; Sprint 267: Sprints 238, 240, 250 and 266 each started one).
fn python(program: &str) -> bool {
    let name = program.rsplit('/').next().unwrap_or(program);
    matches!(name, "python" | "python2" | "python3" | "pip" | "pip3")
        || name
            .strip_prefix("python3.")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// An option word that edits in place for `perl` (`-i`, `-pi`, `-0pi`, `-i.bak`): a cluster of option letters holding
/// `i`, not `-I`, which names an include directory, and not a long `--` option.
fn perl_in_place(word: &str) -> bool {
    let Some(cluster) = word.strip_prefix('-') else {
        return false;
    };
    !cluster.starts_with('-')
        && cluster
            .split('.')
            .next()
            .is_some_and(|letters| letters.contains('i') && letters.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// Why the simple command `span` (its words, its program first) is refused for its program: a Python start, directly or
/// by `uv run` or `uvx`, or an in-place `perl` or `awk` edit, which is a multi-line edit made outside the Edit tool.
fn program_refusal(span: &[String]) -> Option<String> {
    // `NAME=value` words before the program are the environment's; a command substitution's program follows its `$(`
    // or backtick (`x=$(python3 -c 1)`)
    let assignment = |w: &str| {
        w.split_once('=')
            .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            && !w.contains("$(")
            && !w.contains('`')
    };
    let span = &span[span.iter().position(|w| !assignment(w))?..];
    let first = span.first()?.as_str();
    let program = first
        .rsplit_once("$(")
        .map(|(_, p)| p)
        .or_else(|| first.rsplit_once('`').map(|(_, p)| p))
        .unwrap_or(first)
        .trim_start_matches('(');
    let name = program.rsplit('/').next().unwrap_or(program);
    // the program after uv's options, skipping the value of each that takes one (`uv run --with x python3`)
    let after_options = |rest: &[String]| {
        let mut words = rest.iter();
        while let Some(w) = words.next() {
            if matches!(
                w.as_str(),
                "--with" | "--python" | "-p" | "--from" | "--project" | "--directory" | "--with-requirements"
            ) {
                words.next();
            } else if !w.starts_with('-') {
                return Some(w.clone());
            }
        }
        None
    };
    let run = match name {
        "uv" if span.get(1).is_some_and(|w| w == "run") => after_options(&span[2..]),
        "uvx" => after_options(&span[1..]),
        _ => Some(program.to_string()),
    };
    if let Some(p) = run
        && python(&p)
    {
        return Some(format!(
            "{p} starts Python, and no Python runs in w2m (the Decision \"No Python runs\")"
        ));
    }
    let args = &span[1..];
    if name == "perl" && args.iter().any(|a| perl_in_place(a)) {
        return Some("perl editing in place: a multi-line edit goes through the Edit tool".to_string());
    }
    let awk_in_place = args.windows(2).any(|p| p[0] == "-i" && p[1] == "inplace")
        || args.iter().any(|a| a == "-iinplace" || a == "--inplace");
    if matches!(name, "awk" | "gawk") && awk_in_place {
        return Some("awk editing in place: a multi-line edit goes through the Edit tool".to_string());
    }
    None
}

/// Why `command` is refused, or None.
pub fn refusal(command: &str) -> Option<String> {
    let w = words(&without_heredoc_bodies(command));
    // each simple command's program, the first word after a start or a `;`, `&&`, `||`, `&`, `|` or newline
    for (k, _) in w.iter().enumerate() {
        if k > 0 && !(ends_command(&w[k - 1]) || w[k - 1] == "|") {
            continue;
        }
        let end = w[k..]
            .iter()
            .position(|x| ends_command(x) || x == "|")
            .map_or(w.len(), |e| k + e);
        if let Some(why) = program_refusal(&w[k..end]) {
            return Some(why);
        }
    }
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
            // `-n` keeps only the printed lines; `-f` reads a script the guard cannot see (Sprint 277). An `-i` word
            // carries its backup suffix, any letters, so it is not read for flags.
            let flag = |a: &str, short: char, long: &[&str]| {
                long.iter().any(|l| a.starts_with(l))
                    || (a.starts_with('-') && !a.starts_with("--") && !a.starts_with("-i") && a[1..].contains(short))
            };
            if args.iter().any(|a| flag(a, 'n', &["--quiet", "--silent"])) {
                return Some("sed -i with -n, which keeps only the printed lines".into());
            }
            if args.iter().any(|a| flag(a, 'f', &["--file"])) {
                return Some("sed -i with -f, a script the guard cannot read".into());
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
                .contains("an a, i or c command")
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

    /// Sprint 277: every `sed -i` command that changes the line count or writes another file is refused, each with
    /// its reason; Sprint 276's tail roll inserted a file's lines by `343r` (plant 1), and a command inside braces
    /// passed the split on `;` (plant 2).
    #[test]
    fn every_line_changing_sed_command_is_refused() {
        let refused = [
            ("sed -i '' '343r /tmp/entry.txt' docs/a.md", "an r or R command"),
            ("sed -i '/^x/R f' a.md", "an r or R command"),
            ("sed -i '$a last line' f.rs", "an a, i or c command"),
            ("sed -i '' '5i text' f.rs", "an a, i or c command"),
            ("sed -i '' '/^x/c y' f.rs", "an a, i or c command"),
            ("sed -i '' '5G' f.rs", "a G, N or D command"),
            ("sed -i '' '/x/{N;s/a/b/}' f.rs", "a G, N or D command"),
            ("sed -i '' '$!N;P;D' f.rs", "a ! command"),
            ("sed -i '' '/x/!d' f.rs", "a ! command"),
            ("sed -i '' '5p' f.rs", "a p, P, =, l or F command"),
            ("sed -i '' '=' f.rs", "a p, P, =, l or F command"),
            ("sed -i '' 's/a;b/c/p' f.rs", "an s command with a p, w or e flag"),
            ("sed -i '' 's/a/b/w out.txt' f.rs", "an s command with a p, w or e flag"),
            ("sed -i '' '10q' f.rs", "a q or Q command"),
            ("sed -i '' '/x/w out.txt' f.rs", "a w, W or e command"),
            ("sed -i '' '/a/,/b/{s/x/y/}' f.rs", "an address range"),
            ("sed -i -n '5p' f.rs", "-n, which keeps only the printed lines"),
            ("sed -i '' -f edits.sed f.rs", "-f, a script the guard cannot read"),
        ];
        for (command, why) in refused {
            let r = refusal(command).unwrap_or_else(|| panic!("allowed: {command}"));
            assert!(r.contains(why), "{command}: {r}");
        }
    }

    #[test]
    fn one_line_seds_and_reading_seds_are_allowed() {
        assert!(refusal("sed -i '' 's/a;b/c;d/' f.rs").is_none());
        assert!(refusal("sed -i '' '/^x/{s/a/b/;s/c/d/}' f.rs").is_none());
        assert!(refusal("sed -i '' 'y/abc/xyz/' f.rs").is_none());
        assert!(refusal("sed -i.new 's/a/b/' f.rs").is_none());
        assert!(refusal("sed -i '' 's/^status: active$/status: done/' sprints/x.md").is_none());
        assert!(refusal("sed -i '' '623s/gates running)/closed)/' BLACKBOARD.md").is_none());
        assert!(refusal("sed -i '' '/^pub const A: u64 = 1;$/d; /^pub const B: u64 = 2;$/d' p.rs").is_none());
        assert!(refusal("sed -n 1,40p f.rs").is_none());
        assert!(refusal("sed -n '/a/,/b/p' f.rs").is_none());
    }

    /// Sprint 267 (reviewer-3's Medium 1 at 266): a Python start, by any route, is refused; Sprint 266's own,
    /// `python3 -` at a command's head, among them.
    #[test]
    fn a_python_start_is_refused() {
        for command in [
            "python3 - 2>/dev/null; perl -0pi -e 's/a/b/' f.rs",
            "python3 -c 1",
            "uv run python -c 1",
            "uv run --with x python3 tool.py",
            "uvx python -V",
            "cd /x && python script.py",
            "ls | python3 -c 'import sys'",
            "/usr/bin/python3.12 -V",
            "pip3 install x",
            "x=$(python3 -c 1)",
        ] {
            let why = refusal(command).unwrap_or_else(|| panic!("{command:?} allowed"));
            assert!(why.contains("starts Python"), "{command:?}: {why}");
        }
    }

    /// Sprint 267: an in-place `perl` or `awk` edit is refused; Sprint 266's `perl -0pi` among them.
    #[test]
    fn an_in_place_perl_or_awk_edit_is_refused() {
        for command in [
            "perl -0pi -e 's/(a\\n)/$1b/' crates/w2m_vm/src/run.rs",
            "perl -i -pe 's/a/b/' f.rs",
            "perl -pi.bak -e 's/a/b/' f.rs",
            "gawk -i inplace '{print}' f.rs",
            "awk --inplace '{print}' f.rs",
        ] {
            let why = refusal(command).unwrap_or_else(|| panic!("{command:?} allowed"));
            assert!(why.contains("editing in place"), "{command:?}: {why}");
        }
    }

    /// Sprint 267: a word naming Python that starts nothing, and a `perl` or `awk` that only reads, are allowed, as is
    /// the record script's `uv run substrate`.
    #[test]
    fn python_named_but_not_started_and_reading_scripts_are_allowed() {
        for command in [
            "grep -n python BLACKBOARD.md",
            "git commit -m 'no python3 runs'",
            "git commit -F - <<'EOF'\npython3 -c 1\nEOF",
            "uv run substrate run --file f.toml --root r --strict",
            "perl -ne 'print if /x/' f.rs",
            "perl -I lib -e 'print 1'",
            "awk '{print $1}' f.tsv",
            "rg python3 docs",
        ] {
            assert_eq!(refusal(command), None, "{command:?}");
        }
    }
}
