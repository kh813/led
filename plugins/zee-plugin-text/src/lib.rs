use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineNode {
    pub title: String,
    pub level: usize,
    pub line: usize,
    pub children: Vec<OutlineNode>,
    pub is_expanded: bool,
}

// ---------------------------------------------------------------------------
// Host <-> plugin memory ABI
//
// * The host calls `zee_alloc(len)` to obtain a buffer, writes `len` bytes into
//   it, then passes `(ptr, len)` to an export.
// * Exports return a packed `u64` = `(ptr << 32) | len` (0 on failure).
// * The host frees every buffer with `zee_dealloc(ptr, len)`.
//
// Invariant: every buffer handed to the host has `capacity == len`, so that
// `zee_dealloc` can reconstruct it with `Vec::from_raw_parts(ptr, 0, len)`.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn zee_alloc(len: i32) -> *mut u8 {
    if len <= 0 {
        return std::ptr::null_mut();
    }
    let mut buf = std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(len as usize));
    buf.as_mut_ptr()
}

/// # Safety
/// `ptr` must have been returned by `zee_alloc(len)` or produced by this
/// plugin's packed result with the same `len`, and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn zee_dealloc(ptr: *mut u8, len: i32) {
    if !ptr.is_null() && len > 0 {
        // length 0: we never read the (possibly uninitialised) contents.
        drop(Vec::from_raw_parts(ptr, 0, len as usize));
    }
}

/// Leaks `bytes` to the host. `into_boxed_slice` guarantees `capacity == len`.
fn pack_result(bytes: Vec<u8>) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let boxed = std::mem::ManuallyDrop::new(bytes.into_boxed_slice());
    let len = boxed.len() as u64;
    let ptr = boxed.as_ptr() as u64;
    (ptr << 32) | (len & 0xFFFF_FFFF)
}

/// # Safety
/// `ptr` must be valid for reads of `len` bytes.
unsafe fn read_str<'a>(ptr: *const u8, len: i32) -> Option<&'a str> {
    if ptr.is_null() || len <= 0 {
        return None;
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len as usize)).ok()
}

/// # Safety
/// `ptr` must be valid for reads of `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn zee_parse_outline(ptr: *const u8, len: i32) -> u64 {
    let Some(text) = read_str(ptr, len) else {
        return 0;
    };
    match serde_json::to_vec(&parse_markdown_headings(text)) {
        Ok(json) => pack_result(json),
        Err(_) => 0,
    }
}

fn parse_markdown_headings(content: &str) -> Vec<OutlineNode> {
    let mut root_nodes: Vec<OutlineNode> = Vec::new();
    let mut stack: Vec<OutlineNode> = Vec::new();
    let mut in_code_block = false;

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        if trimmed.starts_with('#') {
            let count = trimmed.chars().take_while(|&c| c == '#').count();
            if (1..=6).contains(&count) {
                let rest = trimmed[count..].trim();
                if !rest.is_empty() {
                    let node = OutlineNode {
                        title: rest.to_string(),
                        level: count,
                        line: line_idx,
                        children: Vec::new(),
                        is_expanded: true,
                    };

                    while let Some(top) = stack.last() {
                        if top.level >= count {
                            let popped = stack.pop().unwrap();
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(popped);
                            } else {
                                root_nodes.push(popped);
                            }
                        } else {
                            break;
                        }
                    }
                    stack.push(node);
                }
            }
        }
    }

    while let Some(popped) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(popped);
        } else {
            root_nodes.push(popped);
        }
    }

    root_nodes
}

/// # Safety
/// `cmd_ptr`/`text_ptr` must be valid for reads of `cmd_len`/`text_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn zee_transform_text(
    cmd_ptr: *const u8,
    cmd_len: i32,
    text_ptr: *const u8,
    text_len: i32,
) -> u64 {
    match (read_str(cmd_ptr, cmd_len), read_str(text_ptr, text_len)) {
        (Some(cmd), Some(text)) => pack_result(transform(cmd, text).into_bytes()),
        _ => 0,
    }
}

/// # Safety
/// Same contract as [`zee_transform_text`].
#[no_mangle]
pub unsafe extern "C" fn zee_execute_command(
    cmd_ptr: *const u8,
    cmd_len: i32,
    args_ptr: *const u8,
    args_len: i32,
) -> u64 {
    zee_transform_text(cmd_ptr, cmd_len, args_ptr, args_len)
}

/// Pure (FFI-free) implementation of every text command.
fn transform(cmd: &str, text: &str) -> String {
    let reformat_json = |pretty: bool| match serde_json::from_str::<serde_json::Value>(text) {
        Ok(val) if pretty => serde_json::to_string_pretty(&val).unwrap_or_else(|_| text.to_string()),
        Ok(val) => serde_json::to_string(&val).unwrap_or_else(|_| text.to_string()),
        Err(_) => text.to_string(),
    };
    let map_lines = |f: fn(&mut Vec<&str>)| {
        let mut lines: Vec<&str> = text.lines().collect();
        f(&mut lines);
        lines.join("\n")
    };

    match cmd {
        "format_json" => reformat_json(true),
        "minify_json" => reformat_json(false),
        "sort_lines" => map_lines(|l| l.sort()),
        "reverse_lines" => map_lines(|l| l.reverse()),
        "to_uppercase" => text.to_uppercase(),
        "to_lowercase" => text.to_lowercase(),
        "to_snake_case" => to_snake_case(text),
        "to_camel_case" => to_camel_case(text),
        _ => text.to_string(),
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in s.chars() {
        if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else if c == '-' || c == ' ' {
            result.push('_');
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = c.is_alphanumeric();
        }
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else if i == 0 {
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_commands() {
        assert_eq!(transform("sort_lines", "b\na\nc"), "a\nb\nc");
        assert_eq!(transform("reverse_lines", "1\n2\n3"), "3\n2\n1");
        assert_eq!(transform("minify_json", "{ \"a\" : 1 }"), "{\"a\":1}");
        assert_eq!(transform("format_json", "not json"), "not json");
        assert_eq!(transform("to_snake_case", "HelloWorld"), "hello_world");
        assert_eq!(transform("to_camel_case", "hello_world"), "helloWorld");
        assert_eq!(transform("unknown", "x"), "x");
    }

    #[test]
    fn test_outline_nesting_and_code_fences() {
        let md = "# A\n```\n# not heading\n```\n## B\n# C\n";
        let nodes = parse_markdown_headings(md);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].title, "A");
        assert_eq!(nodes[0].children[0].title, "B");
        assert_eq!(nodes[1].title, "C");
    }

    #[test]
    fn test_alloc_pack_dealloc_roundtrip() {
        let p = zee_alloc(16);
        assert!(!p.is_null());
        unsafe { zee_dealloc(p, 16) };
        assert!(zee_alloc(0).is_null());

        // Packed length must equal the payload length (capacity is shrunk to fit).
        let mut v = Vec::with_capacity(64);
        v.extend_from_slice(b"hello");
        assert_eq!(pack_result(v) & 0xFFFF_FFFF, 5);
        assert_eq!(pack_result(Vec::new()), 0);
    }
}
