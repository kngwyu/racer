use core::SourceByteRange;

pub fn rejustify(src: &str) -> String {
    let s = &src[1..]; // remove the newline
    let mut sb = String::new();
    for l in s.lines() {
        let tabless = &l[4..];
        sb.push_str(tabless);
        if !tabless.is_empty() {
            sb.push_str("\n");
        }
    }
    let newlen = sb.len() - 1; // remove the trailing newline
    sb.truncate(newlen);
    sb
}

pub fn slice(src: &str, (begin, end): SourceByteRange) -> &str {
    &src[begin..end]
}
