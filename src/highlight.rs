use eframe::egui::{self, text::LayoutJob, Color32, FontId};

pub(crate) fn append_with_search(
    job: &mut LayoutJob,
    text: &str,
    font_id: FontId,
    color: Color32,
    query: &str,
    current_idx: usize,
    counter: &mut usize,
) {
    if query.is_empty() {
        job.append(text, 0.0, egui::TextFormat { font_id, color, ..Default::default() });
        return;
    }
    let lc_query = query.to_ascii_lowercase();
    let mut rest = text;
    loop {
        let lc_rest = rest.to_ascii_lowercase();
        if let Some(found_rel) = lc_rest.find(&lc_query) {
            let prefix = &rest[..found_rel];
            if !prefix.is_empty() {
                job.append(prefix, 0.0, egui::TextFormat { font_id: font_id.clone(), color, ..Default::default() });
            }
            let matched = &rest[found_rel..found_rel + lc_query.len()];
            let mut fmt = egui::TextFormat { font_id: font_id.clone(), color, ..Default::default() };
            if *counter == current_idx {
                fmt.background = Color32::from_rgba_premultiplied(224, 108, 117, 96);
            } else {
                fmt.background = Color32::from_rgba_premultiplied(255, 255, 0, 64);
            }
            job.append(matched, 0.0, fmt);
            *counter += 1;
            rest = &rest[found_rel + lc_query.len()..];
            if rest.is_empty() { break; }
        } else {
            if !rest.is_empty() {
                job.append(rest, 0.0, egui::TextFormat { font_id, color, ..Default::default() });
            }
            break;
        }
    }
}

pub(crate) fn token_highlight(
    job: &mut LayoutJob,
    text: &str,
    ext: &str,
    font_id: FontId,
    base_color: Color32,
    query: &str,
    do_syntax: bool,
    depth: &mut i32,
    current_idx: usize,
    counter: &mut usize,
) {
    if !do_syntax {
        append_with_search(job, text, font_id, base_color, query, current_idx, counter);
        return;
    }
    let kw_color = Color32::from_rgb(97, 175, 239);
    let num_color = Color32::from_rgb(209, 154, 102);
    let bool_color = Color32::from_rgb(198, 120, 221);
    let bracket_colors = [
        Color32::from_rgb(152, 195, 121),
        Color32::from_rgb(224, 108, 117),
        Color32::from_rgb(97, 175, 239),
        Color32::from_rgb(229, 192, 123),
        Color32::from_rgb(86, 182, 194),
    ];

    let keywords_rs: &[&str] = &[
        "as","async","await","break","const","continue","crate","dyn","else","enum","extern","false","fn","for","if","impl","in","let","loop","match","mod","move","mut","pub","ref","return","self","Self","static","struct","super","trait","true","type","unsafe","use","where","while",
        "union","box","try","yield","macro","macro_rules"
    ];
    let keywords_py: &[&str] = &[
        "False","None","True","and","as","assert","async","await","break","class","continue","def","del","elif","else","except","finally","for","from","global","if","import","in","is","lambda","nonlocal","not","or","pass","raise","return","try","while","with","yield","match","case"
    ];

    let mut buf = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            buf.push(ch);
        } else {
            if !buf.is_empty() {
                let lc = buf.to_ascii_lowercase();
                let (color, _) = if ext == "rs" && keywords_rs.contains(&buf.as_str()) {
                    (kw_color, true)
                } else if ext == "py" && keywords_py.contains(&buf.as_str()) {
                    (kw_color, true)
                } else if lc == "true" || lc == "false" || lc == "null" || lc == "none" {
                    (bool_color, true)
                } else if buf.chars().all(|c| c.is_ascii_digit()) {
                    (num_color, true)
                } else {
                    (base_color, false)
                };
                append_with_search(job, &buf, font_id.clone(), color, query, current_idx, counter);
                buf.clear();
            }
            let color = match ch {
                '(' | '[' | '{' => {
                    let idx = ((*depth).max(0) as usize) % bracket_colors.len();
                    *depth = depth.saturating_add(1);
                    Some(bracket_colors[idx])
                }
                ')' | ']' | '}' => {
                    *depth = depth.saturating_sub(1);
                    let idx = ((*depth).max(0) as usize) % bracket_colors.len();
                    Some(bracket_colors[idx])
                }
                _ => None,
            };
            let delim = ch.to_string();
            append_with_search(job, &delim, font_id.clone(), color.unwrap_or(base_color), query, current_idx, counter);
        }
    }
    if !buf.is_empty() {
        let lc = buf.to_ascii_lowercase();
        let (color, _) = if ext == "rs" && keywords_rs.contains(&buf.as_str()) {
            (kw_color, true)
        } else if ext == "py" && keywords_py.contains(&buf.as_str()) {
            (kw_color, true)
        } else if lc == "true" || lc == "false" || lc == "null" || lc == "none" {
            (bool_color, true)
        } else if buf.chars().all(|c| c.is_ascii_digit()) {
            (num_color, true)
        } else {
            (base_color, false)
        };
        append_with_search(job, &buf, font_id, color, query, current_idx, counter);
    }
}

pub(crate) fn append_highlighted(
    job: &mut LayoutJob,
    line: &str,
    ext: &str,
    query: &str,
    font_id: FontId,
    base_color: Color32,
    do_syntax: bool,
    depth: &mut i32,
    current_idx: usize,
    counter: &mut usize,
    in_block_comment: &mut bool,
) {
    if do_syntax {
        if ext == "rs" {
            let mut i = 0usize;
            if *in_block_comment {
                if let Some(end) = line[i..].find("*/") {
                    let end_abs = i + end + 2;
                    let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                    job.append(&line[i..end_abs], 0.0, fmt);
                    *in_block_comment = false;
                    i = end_abs;
                } else {
                    let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                    job.append(&line[i..], 0.0, fmt);
                    return;
                }
            }
            while i < line.len() {
                let rest = &line[i..];
                let pos_sl = rest.find("//");
                let pos_blk = rest.find("/*");
                match (pos_sl, pos_blk) {
                    (Some(psl), Some(pblk)) if psl < pblk => {
                        if psl > 0 {
                            token_highlight(job, &rest[..psl], ext, font_id.clone(), base_color, query, do_syntax, depth, current_idx, counter);
                        }
                        let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                        job.append(&rest[psl..], 0.0, fmt);
                        return;
                    }
                    (Some(psl), None) => {
                        if psl > 0 {
                            token_highlight(job, &rest[..psl], ext, font_id.clone(), base_color, query, do_syntax, depth, current_idx, counter);
                        }
                        let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                        job.append(&rest[psl..], 0.0, fmt);
                        return;
                    }
                    (None, Some(pblk)) => {
                        if pblk > 0 {
                            token_highlight(job, &rest[..pblk], ext, font_id.clone(), base_color, query, do_syntax, depth, current_idx, counter);
                        }
                        let after = pblk + 2;
                        let tail = &rest[after..];
                        if let Some(end) = tail.find("*/") {
                            let end_abs = i + after + end + 2;
                            let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                            job.append(&line[i + pblk..end_abs], 0.0, fmt);
                            i = end_abs;
                            continue;
                        } else {
                            let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                            job.append(&rest[pblk..], 0.0, fmt);
                            *in_block_comment = true;
                            return;
                        }
                    }
                    (None, None) => {
                        token_highlight(job, rest, ext, font_id.clone(), base_color, query, do_syntax, depth, current_idx, counter);
                        return;
                    }
                    (Some(_psl), Some(pblk)) => {
                        if pblk > 0 {
                            token_highlight(job, &rest[..pblk], ext, font_id.clone(), base_color, query, do_syntax, depth, current_idx, counter);
                        }
                        let after = pblk + 2;
                        let tail = &rest[after..];
                        if let Some(end) = tail.find("*/") {
                            let end_abs = i + after + end + 2;
                            let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                            job.append(&line[i + pblk..end_abs], 0.0, fmt);
                            i = end_abs;
                            continue;
                        } else {
                            let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                            job.append(&rest[pblk..], 0.0, fmt);
                            *in_block_comment = true;
                            return;
                        }
                    }
                }
            }
            return;
        }
        let comment_prefix = if ext == "rs" || ext == "js" { "//" } else if ext == "toml" { "#" } else { "" };
        let comment_prefix = if ext == "py" { "#" } else { comment_prefix };
        if !comment_prefix.is_empty() {
            if let Some(pos) = line.find(comment_prefix) {
                append_highlighted(job, &line[..pos], "", query, font_id.clone(), base_color, do_syntax, depth, current_idx, counter, in_block_comment);
                let fmt = egui::TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
                job.append(&line[pos..], 0.0, fmt);
                return;
            }
        }
    }

    let mut buf = String::new();

    if do_syntax {
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '"' {
                if !buf.is_empty() { token_highlight(job, &buf, ext, font_id.clone(), base_color, query, do_syntax, depth, current_idx, counter); buf.clear(); }
                buf.clear();
                let mut s = String::from('"');
                while let Some(c2) = chars.next() {
                    s.push(c2);
                    if c2 == '"' { break; }
                }
                append_with_search(job, &s, font_id.clone(), Color32::from_rgb(152, 195, 121), query, current_idx, counter);
            } else {
                buf.push(ch);
            }
        }
    } else {
        buf.push_str(line);
    }

    if !buf.is_empty() {
        token_highlight(job, &buf, ext, font_id, base_color, query, do_syntax, depth, current_idx, counter);
    }
}
