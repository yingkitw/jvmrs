//! JVM method descriptor parsing.

/// Count the number of parameter slots in a method descriptor.
/// Descriptor format: "(params)return" e.g. "(II)I" = two int params, returns int.
pub fn count_parameters(descriptor: &str) -> usize {
    if let Some(params_end) = descriptor.find(')') {
        let params = &descriptor[1..params_end];
        let mut count = 0;
        let mut chars = params.chars();
        while let Some(c) = chars.next() {
            match c {
                'I' | 'F' | 'B' | 'C' | 'S' | 'Z' => count += 1,
                'J' | 'D' => count += 2, // long and double take 2 slots
                'L' => {
                    count += 1;
                    while let Some(c) = chars.next() {
                        if c == ';' {
                            break;
                        }
                    }
                }
                '[' => {
                    while let Some(c) = chars.next() {
                        if c != '[' {
                            if c == 'L' {
                                count += 1;
                                while let Some(c) = chars.next() {
                                    if c == ';' {
                                        break;
                                    }
                                }
                            } else {
                                count += 1;
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        count
    } else {
        0
    }
}

/// Parse method parameter type names from descriptor.
pub fn parse_method_params(descriptor: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut i = descriptor.find('(').unwrap_or(0) + 1;
    let end = descriptor.find(')').unwrap_or(descriptor.len());

    while i < end {
        match descriptor.chars().nth(i) {
            Some('B') => {
                params.push("byte".to_string());
                i += 1;
            }
            Some('C') => {
                params.push("char".to_string());
                i += 1;
            }
            Some('D') => {
                params.push("double".to_string());
                i += 1;
            }
            Some('F') => {
                params.push("float".to_string());
                i += 1;
            }
            Some('I') => {
                params.push("int".to_string());
                i += 1;
            }
            Some('J') => {
                params.push("long".to_string());
                i += 1;
            }
            Some('S') => {
                params.push("short".to_string());
                i += 1;
            }
            Some('Z') => {
                params.push("boolean".to_string());
                i += 1;
            }
            Some('L') => {
                let mut j = i + 1;
                while j < end && descriptor.chars().nth(j) != Some(';') {
                    j += 1;
                }
                if j < end {
                    params.push("object".to_string());
                    i = j + 1;
                } else {
                    break;
                }
            }
            Some(')') => break,
            _ => i += 1,
        }
    }

    params
}
