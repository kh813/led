use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineNode {
    pub title: String,
    pub level: usize,
    pub line: usize,
    pub children: Vec<OutlineNode>,
    pub is_expanded: bool,
}

#[no_mangle]
pub extern "C" fn zee_alloc(len: i32) -> *mut u8 {
    let mut buf = Vec::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn zee_dealloc(ptr: *mut u8, len: i32) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, len as usize, len as usize);
        }
    }
}

fn pack_result(bytes: Vec<u8>) -> u64 {
    let len = bytes.len() as u64;
    let ptr = bytes.as_ptr() as u64;
    std::mem::forget(bytes);
    (ptr << 32) | (len & 0xFFFF_FFFF)
}

#[no_mangle]
pub extern "C" fn zee_parse_outline(ptr: *const u8, len: i32) -> u64 {
    if ptr.is_null() || len <= 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let text = match std::str::from_utf8(slice) {
        Ok(t) => t,
        Err(_) => return 0,
    };

    let nodes = parse_markdown_headings(text);
    let json_bytes = match serde_json::to_vec(&nodes) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    pack_result(json_bytes)
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

#[no_mangle]
pub extern "C" fn zee_transform_text(
    cmd_ptr: *const u8,
    cmd_len: i32,
    text_ptr: *const u8,
    text_len: i32,
) -> u64 {
    if cmd_ptr.is_null() || cmd_len <= 0 || text_ptr.is_null() || text_len <= 0 {
        return 0;
    }

    let cmd_slice = unsafe { std::slice::from_raw_parts(cmd_ptr, cmd_len as usize) };
    let text_slice = unsafe { std::slice::from_raw_parts(text_ptr, text_len as usize) };

    let cmd = match std::str::from_utf8(cmd_slice) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let text = match std::str::from_utf8(text_slice) {
        Ok(t) => t,
        Err(_) => return 0,
    };

    let result = match cmd {
        "format_json" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                serde_json::to_string_pretty(&val).unwrap_or_else(|_| text.to_string())
            } else {
                text.to_string()
            }
        }
        "minify_json" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                serde_json::to_string(&val).unwrap_or_else(|_| text.to_string())
            } else {
                text.to_string()
            }
        }
        "sort_lines" => {
            let mut lines: Vec<&str> = text.lines().collect();
            lines.sort();
            lines.join("\n")
        }
        "reverse_lines" => {
            let mut lines: Vec<&str> = text.lines().collect();
            lines.reverse();
            lines.join("\n")
        }
        "to_uppercase" => text.to_uppercase(),
        "to_lowercase" => text.to_lowercase(),
        "to_snake_case" => to_snake_case(text),
        "to_camel_case" => to_camel_case(text),
        _ => text.to_string(),
    };

    pack_result(result.into_bytes())
}

#[no_mangle]
pub extern "C" fn zee_execute_command(
    cmd_ptr: *const u8,
    cmd_len: i32,
    args_ptr: *const u8,
    args_len: i32,
) -> u64 {
    zee_transform_text(cmd_ptr, cmd_len, args_ptr, args_len)
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
