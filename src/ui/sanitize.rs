/// Sanitizes untrusted text for safe terminal display.
///
/// Goals:
/// - prevent terminal escape injection (CSI/OSC/etc)
/// - remove other control characters (except optional newlines)
/// - keep output reasonably readable/debuggable via placeholders
pub fn sanitize_for_terminal(input: &str, allow_newlines: bool) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // ESC
            '\x1b' => {
                match chars.peek().copied() {
                    // CSI: ESC [ ... <final>
                    Some('[') => {
                        let _ = chars.next();
                        for c in chars.by_ref() {
                            if ('@'..='~').contains(&c) {
                                break;
                            }
                        }
                        out.push_str("[CSI]");
                    }
                    // OSC: ESC ] ... BEL | ESC \
                    Some(']') => {
                        let _ = chars.next();
                        while let Some(c) = chars.next() {
                            if c == '\x07' {
                                break;
                            }
                            if c == '\x1b' && matches!(chars.peek().copied(), Some('\\')) {
                                let _ = chars.next();
                                break;
                            }
                        }
                        out.push_str("[OSC]");
                    }
                    // Other escape: consume a single following char if present
                    Some(_) => {
                        let _ = chars.next();
                        out.push_str("[ESC]");
                    }
                    None => out.push_str("[ESC]"),
                }
            }

            // Other control characters
            c if c.is_control() => {
                if (allow_newlines && c == '\n') || c == '\t' || c == '\r' {
                    out.push(c);
                } else {
                    out.push('�');
                }
            }

            _ => out.push(ch),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::sanitize_for_terminal;

    #[test]
    fn strips_csi_and_osc_escape_sequences() {
        assert_eq!(
            sanitize_for_terminal("hello\x1b[31mred\x1b[0m world", false),
            "hello[CSI]red[CSI] world"
        );
        assert_eq!(
            sanitize_for_terminal("title\x1b]0;tab title\x07done", false),
            "title[OSC]done"
        );
    }

    #[test]
    fn preserves_allowed_whitespace_and_replaces_other_controls() {
        assert_eq!(sanitize_for_terminal("a\nb\t\rc\x00", true), "a\nb\t\rc�");
        assert_eq!(sanitize_for_terminal("a\nb", false), "a�b");
    }

    #[test]
    fn handles_unknown_escape_sequences() {
        assert_eq!(sanitize_for_terminal("x\x1bz", false), "x[ESC]");
        assert_eq!(sanitize_for_terminal("x\x1b", false), "x[ESC]");
    }
}
