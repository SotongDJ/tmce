//! tmce — Tab-separate Markdown Consolidation Engine.
//!
//! CLI subcommands:
//!   tmce input   <target.tmc> <source.md> <tags> [<base62-Gu-id>]
//!   tmce compact <target.tmc>
//!   tmce relate  <target.tmc>

mod base62;
mod error;
mod escape;
mod markdown;
mod order;
mod section;
mod tmc;

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::error::{Result, TmceError};
use crate::markdown::{decompose, recompose, ContentDraft};
use crate::order::Order;
use crate::tmc::{normalise_tags, Container, Row};

fn usage() -> ExitCode {
    eprintln!("Usage:");
    eprintln!("  tmce input   <target.tmc> <source.md> <tags> [<base62-Gu-id>]");
    eprintln!("  tmce list    <target.tmc>");
    eprintln!("  tmce remove  <target.tmc> <uuid>");
    eprintln!("  tmce view    <target.tmc> <uuid>");
    eprintln!("  tmce compact <target.tmc>");
    eprintln!("  tmce relate  <target.tmc>");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [flag] if flag == "--version" || flag == "-V" || flag == "-v" => {
            println!("tmce {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        [cmd, target, source, tags] if cmd == "input" => {
            cmd_input(Path::new(target), Path::new(source), tags, None)
        }
        [cmd, target, source, tags, id] if cmd == "input" => {
            cmd_input(Path::new(target), Path::new(source), tags, Some(id))
        }
        [cmd, target] if cmd == "list" => cmd_list(Path::new(target)),
        [cmd, target, uuid] if cmd == "remove" => cmd_remove(Path::new(target), uuid),
        [cmd, target, uuid] if cmd == "view" => cmd_view(Path::new(target), uuid),
        [cmd, target] if cmd == "compact" => cmd_compact(Path::new(target)),
        [cmd, target] if cmd == "relate" => cmd_relate(Path::new(target)),
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(1)
        }
    }
}

/// `tmce input`: decompose `source` and append as a new member file.
fn cmd_input(target: &Path, source: &Path, tags: &str, id: Option<&str>) -> Result<()> {
    let md = fs::read_to_string(source)?;

    let mut container = if target.exists() {
        Container::load(target)?
    } else {
        Container::default()
    };

    let uuid = match id {
        Some(s) => {
            // Caller supplied a base62-Gu id; require it to validate.
            base62::validate_uuid(s)?;
            s.to_string()
        }
        None => source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| TmceError::Other(format!("cannot derive filename from {:?}", source)))?
            .to_string(),
    };

    if container.find_header(&uuid).is_some() {
        return Err(TmceError::DuplicateMember(uuid));
    }

    let tags = normalise_tags(tags);
    let title = derive_title(&md).unwrap_or_else(|| uuid.clone());

    let new_member_order = next_member_order(&mut container)?;

    // Header row for the new member. Use the same width as the member_order
    // so that the file_order pseudo-value ("a…a") is width-consistent with
    // any auto-widening that may have just occurred.
    let header_file_order = Order::header(new_member_order.width());
    container.rows.push(Row::Header {
        file_order: header_file_order,
        member_order: new_member_order.clone(),
        uuid: uuid.clone(),
        tags,
        title,
    });

    // Content rows from markdown decomposition.
    for (i, draft) in decompose(&md).into_iter().enumerate() {
        let section_order = Order::from_value(i as u64, 1)
            .or_else(|| Order::from_value(i as u64, 2))
            .or_else(|| Order::from_value(i as u64, 3))
            .ok_or_else(|| TmceError::Other(format!("too many sections: {}", i + 1)))?;
        container.rows.push(Row::Content {
            file_order: new_member_order.clone(),
            section_order,
            section_type: draft.section_type,
            indent: draft.indent,
            content: draft.content,
        });
    }

    container.validate()?;
    container.save(target)?;
    eprintln!(
        "Added {} ({} content rows) to {}",
        uuid,
        container
            .rows
            .iter()
            .filter(|r| matches!(r, Row::Content { file_order, .. } if file_order == &new_member_order))
            .count(),
        target.display()
    );
    Ok(())
}

/// `tmce list`: print member uuids in header-declaration order.
fn cmd_list(target: &Path) -> Result<()> {
    let container = Container::load(target)?;
    for r in container.header_rows() {
        if let Row::Header { uuid, .. } = r {
            println!("{}", uuid);
        }
    }
    Ok(())
}

/// `tmce remove`: drop a member's header row and all its content rows.
fn cmd_remove(target: &Path, uuid: &str) -> Result<()> {
    let mut container = Container::load(target)?;
    let member_order = container
        .rows
        .iter()
        .find_map(|r| match r {
            Row::Header { uuid: u, member_order, .. } if u == uuid => {
                Some(member_order.clone())
            }
            _ => None,
        })
        .ok_or_else(|| TmceError::MissingMember(uuid.to_string()))?;

    container.rows.retain(|r| match r {
        Row::Header { uuid: u, .. } => u != uuid,
        Row::Content { file_order, .. } => file_order != &member_order,
    });

    container.save(target)?;
    eprintln!("Removed {} from {}", uuid, target.display());
    Ok(())
}

/// `tmce view`: reconstruct a member's markdown and print it.
fn cmd_view(target: &Path, uuid: &str) -> Result<()> {
    let container = Container::load(target)?;
    let member_order = container
        .rows
        .iter()
        .find_map(|r| match r {
            Row::Header { uuid: u, member_order, .. } if u == uuid => {
                Some(member_order.clone())
            }
            _ => None,
        })
        .ok_or_else(|| TmceError::MissingMember(uuid.to_string()))?;

    let mut drafts: Vec<(Order, ContentDraft)> = container
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Content {
                file_order,
                section_order,
                section_type,
                indent,
                content,
            } if file_order == &member_order => Some((
                section_order.clone(),
                ContentDraft {
                    section_type: section_type.clone(),
                    indent: *indent,
                    content: content.clone(),
                },
            )),
            _ => None,
        })
        .collect();

    drafts.sort_by(|a, b| a.0.cmp(&b.0));
    let drafts: Vec<ContentDraft> = drafts.into_iter().map(|(_, d)| d).collect();

    let rendered = recompose(&drafts);
    print!("{}", rendered);
    Ok(())
}

/// `tmce compact`: validate, then sort member files by uuid and reassign
/// `file_order`. Validation runs both before and after rewriting.
fn cmd_compact(target: &Path) -> Result<()> {
    let container = Container::load(target)?;
    container.validate()?;

    // Collect (uuid, old_member_order) pairs in sorted order.
    let mut entries: Vec<(String, Order)> = container
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Header { uuid, member_order, .. } => {
                Some((uuid.clone(), member_order.clone()))
            }
            _ => None,
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Compute new member_order for each uuid: `b`, `c`, … (value 1, 2, …).
    let width = Order::min_width_for(1 + entries.len() as u64);
    let mut remap: std::collections::HashMap<String, Order> = std::collections::HashMap::new();
    for (i, (uuid, _old)) in entries.iter().enumerate() {
        let new_order = Order::from_value(1 + i as u64, width)
            .ok_or_else(|| TmceError::Other("too many members for chosen width".into()))?;
        remap.insert(uuid.clone(), new_order);
    }

    // Rewrite rows.
    let old_to_new: std::collections::HashMap<String, Order> = container
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Header { uuid, member_order, .. } => {
                remap.get(uuid).map(|new| (member_order.render(), new.clone()))
            }
            _ => None,
        })
        .collect();

    let mut new_rows: Vec<Row> = Vec::with_capacity(container.rows.len());
    for r in &container.rows {
        match r {
            Row::Header {
                member_order,
                uuid,
                tags,
                title,
                ..
            } => {
                let new_member = old_to_new
                    .get(&member_order.render())
                    .cloned()
                    .ok_or_else(|| TmceError::Other("inconsistent header remap".into()))?;
                new_rows.push(Row::Header {
                    file_order: Order::header(width),
                    member_order: new_member,
                    uuid: uuid.clone(),
                    tags: tags.clone(),
                    title: title.clone(),
                });
            }
            Row::Content {
                file_order,
                section_order,
                section_type,
                indent,
                content,
            } => {
                let new_fo = old_to_new
                    .get(&file_order.render())
                    .cloned()
                    .ok_or_else(|| {
                        TmceError::Other(format!(
                            "content row refers to unknown file_order {:?}",
                            file_order.render()
                        ))
                    })?;
                new_rows.push(Row::Content {
                    file_order: new_fo,
                    section_order: section_order.clone(),
                    section_type: section_type.clone(),
                    indent: *indent,
                    content: content.clone(),
                });
            }
        }
    }

    // Sort rows: headers first (by member_order), then content (by file_order
    // then section_order).
    new_rows.sort_by(|a, b| {
        let a_key = sort_key(a);
        let b_key = sort_key(b);
        a_key.cmp(&b_key)
    });

    let out = Container { rows: new_rows };
    out.validate()?;
    out.save(target)?;
    eprintln!("Compacted {}: {} members", target.display(), entries.len());
    Ok(())
}

/// Sort key: (0, member_order) for header rows; (1, file_order, section_order)
/// for content. We encode with a tuple that sorts naturally.
fn sort_key(r: &Row) -> (u8, String, String) {
    match r {
        Row::Header { member_order, .. } => (0, member_order.render(), String::new()),
        Row::Content {
            file_order,
            section_order,
            ..
        } => (1, file_order.render(), section_order.render()),
    }
}

/// `tmce relate`: create two sibling index files.
///
/// `<target>.ft.rtv` — file → tags. One line per member:
///   <uuid>\t<tag1,tag2,…>
///
/// `<target>.tf.rtv` — tag → files. One line per tag:
///   <tag>\t<uuid1,uuid2,…>
///
/// Both sorted. Uuids and tag values go through the TMC escape rules.
fn cmd_relate(target: &Path) -> Result<()> {
    let container = Container::load(target)?;
    container.validate()?;

    let mut ft: Vec<(String, Vec<String>)> = Vec::new();
    let mut tag_to_uuids: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for r in container.header_rows() {
        if let Row::Header { uuid, tags, .. } = r {
            let tag_list: Vec<String> = tags
                .split(',')
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect();
            ft.push((uuid.clone(), tag_list.clone()));
            for tag in tag_list {
                tag_to_uuids
                    .entry(tag)
                    .or_default()
                    .push(uuid.clone());
            }
        }
    }

    ft.sort_by(|a, b| a.0.cmp(&b.0));

    let ft_path = sibling_with_suffix(target, "ft.rtv");
    let tf_path = sibling_with_suffix(target, "tf.rtv");

    let mut ft_text = String::new();
    for (uuid, tags) in &ft {
        let joined = tags
            .iter()
            .map(|t| escape::escape(t))
            .collect::<Vec<_>>()
            .join(",");
        ft_text.push_str(&format!("{}\t{}\n", escape::escape(uuid), joined));
    }

    let mut tf_text = String::new();
    for (tag, uuids) in &tag_to_uuids {
        let joined = uuids
            .iter()
            .map(|u| escape::escape(u))
            .collect::<Vec<_>>()
            .join(",");
        tf_text.push_str(&format!("{}\t{}\n", escape::escape(tag), joined));
    }

    atomic_write(&ft_path, &ft_text)?;
    atomic_write(&tf_path, &tf_text)?;

    eprintln!(
        "Wrote {} ({} rows) and {} ({} rows)",
        ft_path.display(),
        ft.len(),
        tf_path.display(),
        tag_to_uuids.len()
    );
    Ok(())
}

fn sibling_with_suffix(target: &Path, suffix: &str) -> std::path::PathBuf {
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("tmce");
    let mut name = String::from(stem);
    name.push('.');
    name.push_str(suffix);
    target.with_file_name(name)
}

fn atomic_write(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("out")
    ));
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

// -- helpers ---------------------------------------------------------------

fn derive_title(md: &str) -> Option<String> {
    for line in md.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("# ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn next_member_order(container: &mut Container) -> Result<Order> {
    // Value 0 is reserved for the header pseudo-file. Pick the next integer
    // greater than any existing member_order.
    let max_used: u64 = container
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Header { member_order, .. } => Some(member_order.value()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let next = max_used + 1;

    // Collect existing widths to detect mixed-width or overflow conditions.
    let existing_widths: std::collections::HashSet<usize> = container
        .rows
        .iter()
        .map(|r| match r {
            Row::Header { member_order, .. } => member_order.width(),
            Row::Content { file_order, .. } => file_order.width(),
        })
        .collect();

    if existing_widths.len() > 1 {
        return Err(TmceError::Other(
            "container has mixed order widths; run `tmce compact`".into(),
        ));
    }

    let current_width = existing_widths.into_iter().next().unwrap_or(1);

    // If `next` fits within the current width, use it as-is.
    if Order::from_value(next, current_width).is_some() {
        return Order::from_value(next, current_width)
            .ok_or_else(|| TmceError::Other(format!("cannot allocate order for value {}", next)));
    }

    // `next` does not fit: auto-widen the container. Compute the minimum
    // width that accommodates `next` member slots (value 0 = header, so we
    // need capacity for values 0..=next).
    let new_width = Order::min_width_for(next + 1);
    eprintln!(
        "Auto-widening file_order from width {} to {} (member slot {} exceeds capacity)",
        current_width, new_width, next
    );

    // Build old_render -> new_order mapping for every existing member_order
    // (content file_orders share the same values).
    // We keep the same numeric values, just re-encoded at the new width.
    let value_map: std::collections::HashMap<String, Order> = container
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Header { member_order, .. } => {
                let old_render = member_order.render();
                let val = member_order.value();
                let new_order = Order::from_value(val, new_width)?;
                Some((old_render, new_order))
            }
            _ => None,
        })
        .collect();

    // Rewrite all rows in-place with the widened orders.
    for row in &mut container.rows {
        match row {
            Row::Header {
                file_order,
                member_order,
                ..
            } => {
                // file_order for header rows is the all-zero pseudo-order.
                *file_order = Order::header(new_width);
                let new_mo = value_map
                    .get(&member_order.render())
                    .cloned()
                    .ok_or_else(|| {
                        TmceError::Other(format!(
                            "widen: unknown member_order {:?}",
                            member_order.render()
                        ))
                    })?;
                *member_order = new_mo;
            }
            Row::Content { file_order, .. } => {
                let new_fo = value_map
                    .get(&file_order.render())
                    .cloned()
                    .ok_or_else(|| {
                        TmceError::Other(format!(
                            "widen: unknown file_order {:?}",
                            file_order.render()
                        ))
                    })?;
                *file_order = new_fo;
            }
        }
    }

    Order::from_value(next, new_width)
        .ok_or_else(|| TmceError::Other(format!("cannot allocate order for value {}", next)))
}
