// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The services the runtime's C defines, found as `src/w2m/guest_runtime.py` finds them: a definition
//! `u64 <dll prefix>_<import name>(...) {` at a line's start, `W2M_VOID` before it marking a function Windows declares
//! void. The Python matched `^(W2M_VOID\s+)?u64\s+(ws2_32|[a-z0-9]+)_(\w+)\([^;{]*\)\s*\{` (re.M); this scanner takes
//! the same text, with no regex crate (no crate but serde_json, Decision 2026-10-24).

use std::collections::BTreeMap;

/// One service: its DLL ("KERNEL32", "WS2_32.DLL", ...), import name, C symbol, and whether it sets RAX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub dll: String,
    pub name: String,
    pub symbol: String,
    pub sets_rax: bool,
}

/// Python's `\s`: space, tab, newline, carriage return, form feed, vertical tab.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b)
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The definition starting at `start` (a line's start), and where the match ends.
fn match_at(text: &[u8], start: usize) -> Option<(bool, String, String, usize)> {
    let mut p = start;
    let mut void = false;
    if text[p..].starts_with(b"W2M_VOID") {
        let mut q = p + 8;
        let spaces = text[q..].iter().take_while(|b| is_space(**b)).count();
        q += spaces;
        if spaces > 0 && text[q..].starts_with(b"u64") {
            void = true;
            p = q;
        }
    }
    if !text[p..].starts_with(b"u64") {
        return None;
    }
    p += 3;
    let spaces = text[p..].iter().take_while(|b| is_space(**b)).count();
    if spaces == 0 {
        return None;
    }
    p += spaces;
    // (ws2_32|[a-z0-9]+)_ : the alternation tries ws2_32 first
    let prefix_len = if text[p..].starts_with(b"ws2_32_") {
        6
    } else {
        text[p..]
            .iter()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            .count()
    };
    if prefix_len == 0 || text.get(p + prefix_len) != Some(&b'_') {
        return None;
    }
    let prefix = String::from_utf8_lossy(&text[p..p + prefix_len]).into_owned();
    p += prefix_len + 1;
    let name_len = text[p..].iter().take_while(|b| is_word(**b)).count();
    if name_len == 0 || text.get(p + name_len) != Some(&b'(') {
        return None;
    }
    let name = String::from_utf8_lossy(&text[p..p + name_len]).into_owned();
    p += name_len + 1;
    // \([^;{]*\)\s*\{ : the first ';' or '{' after the parenthesis must be the '{', with only spaces between it and
    // the last ')'
    let stop = p + text[p..].iter().position(|b| *b == b';' || *b == b'{')?;
    if text[stop] != b'{' {
        return None;
    }
    let inner = &text[p..stop];
    let trimmed = inner.len() - inner.iter().rev().take_while(|b| is_space(**b)).count();
    if trimmed == 0 || inner[trimmed - 1] != b')' {
        return None;
    }
    Some((void, prefix, name, stop + 1))
}

/// Every service in `texts` (file name -> text), files in name order: `guest_runtime.service_table`, refusing a
/// service defined twice by its DLL and name.
pub fn services(texts: &BTreeMap<String, String>) -> Result<Vec<Service>, String> {
    let mut out: Vec<Service> = Vec::new();
    for (file, text) in texts {
        let bytes = text.as_bytes();
        let mut p = 0;
        while p < bytes.len() {
            let at_line_start = p == 0 || bytes[p - 1] == b'\n';
            if at_line_start && let Some((void, prefix, name, end)) = match_at(bytes, p) {
                p = end;
                if prefix == "w2m" {
                    continue;
                }
                let dll = if prefix == "k32" {
                    "KERNEL32".to_string()
                } else {
                    format!("{}.DLL", prefix.to_ascii_uppercase())
                };
                if out.iter().any(|s| s.dll == dll && s.name == name) {
                    return Err(format!("{file}: {prefix}_{name} is defined twice"));
                }
                out.push(Service {
                    dll,
                    symbol: format!("{prefix}_{name}"),
                    name,
                    sets_rax: !void,
                });
                continue;
            }
            p += 1;
        }
    }
    Ok(out)
}

/// The ring's entries that are not services (`guest_runtime.py` `INTERNAL_APIS`).
const INTERNAL_APIS: [&str; 2] = ["Cpuid", "Exception"];

/// `api_ids.h`: `#define W2M_API_<name> <n>`, 1 up in name order (`guest_runtime.api_header`).
pub fn api_header(texts: &BTreeMap<String, String>) -> Result<String, String> {
    let mut names: Vec<String> = services(texts)?.into_iter().map(|s| s.name).collect();
    names.extend(INTERNAL_APIS.iter().map(|n| n.to_string()));
    let count = names.len();
    names.sort();
    names.dedup();
    if names.len() != count {
        return Err("two services share an import name: their W2M_API_ ids would collide".to_string());
    }
    Ok(names
        .iter()
        .enumerate()
        .map(|(i, n)| format!("#define W2M_API_{n} {}\n", i + 1))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn definitions_are_found_as_the_python_pattern_found_them() {
        let t = texts(&[(
            "a.c",
            "u64 k32_GetTickCount(void) {\n\
             W2M_VOID\nu64 k32_Sleep(u64 ms)\n{\n\
             u64 ws2_32_WSAStartup(u64 a, u64 b) {\n\
             u64 shell32_X(u64 a);\n\
             static u64 k32_Hidden(void) {\n\
             u64 w2m_internal(void) {\n\
             u64 oleaut32_SysFreeString(u64 s /* a { */) {\n",
        )]);
        let found: Vec<(String, String, bool)> = services(&t)
            .unwrap()
            .into_iter()
            .map(|s| (s.dll, s.name, s.sets_rax))
            .collect();
        assert_eq!(
            found,
            [
                ("KERNEL32".to_string(), "GetTickCount".to_string(), true),
                ("KERNEL32".to_string(), "Sleep".to_string(), false),
                ("WS2_32.DLL".to_string(), "WSAStartup".to_string(), true),
            ]
        );
    }

    #[test]
    fn a_service_defined_twice_is_refused_by_name() {
        let t = texts(&[("a.c", "u64 k32_Beep(void) {\n"), ("b.c", "u64 k32_Beep(u64 x) {\n")]);
        assert_eq!(services(&t).unwrap_err(), "b.c: k32_Beep is defined twice");
    }

    #[test]
    fn api_ids_count_from_one_in_name_order_with_the_internal_entries() {
        let t = texts(&[("a.c", "u64 k32_Sleep(u64 ms) {\nu64 user32_Beep(void) {\n")]);
        assert_eq!(
            api_header(&t).unwrap(),
            "#define W2M_API_Beep 1\n#define W2M_API_Cpuid 2\n#define W2M_API_Exception 3\n#define W2M_API_Sleep 4\n"
        );
    }
}
