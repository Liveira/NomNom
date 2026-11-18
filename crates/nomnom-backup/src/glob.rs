/// True when `name` matches `pattern`, where `*` is any run of characters and `?` is one character.
/// Names are compared without regard to case.
pub fn matches_name(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::matches_name;

    #[test]
    fn wildcards_work() {
        assert!(matches_name("*.tmp", "a.TMP"));
        assert!(matches_name("node_modules", "Node_Modules"));
        assert!(matches_name("a?c", "abc"));
        assert!(matches_name("*", "anything"));
        assert!(matches_name("f*o*r", "foobar"));
        assert!(!matches_name("*.tmp", "a.tmp.bak"));
        assert!(!matches_name("a?c", "ac"));
        assert!(!matches_name("git", ".git"));
    }
}
