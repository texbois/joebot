use std::{io, env, fs::File, path::Path};
use std::collections::HashMap;

use html5ever::{ParseOpts, parse_document, local_name};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::tendril::TendrilSink;

fn main() {
    let mut messages_html = File::open("messages.html").unwrap();

    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut messages_html)
        .unwrap();

    let mut messages_by_name: HashMap<String, Vec<String>> = HashMap::new();
    let mut full_names: HashMap<String, String> = HashMap::new();

    retrieve_messages(dom.document, &mut messages_by_name, &mut full_names);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("messages.rs");
    let mut f = File::create(&dest_path).unwrap();

    generate_code(&mut f, &messages_by_name, &full_names).unwrap();
}

fn trunc_full_name(full_name: &str) -> String {
    let full_name_sep = full_name.find(' ').unwrap_or(full_name.len() - 1);

    full_name.to_lowercase().chars().take(full_name_sep + 2).collect()
}

fn generate_code<W: io::Write>(out: &mut W, messages: &HashMap<String, Vec<String>>, full_names: &HashMap<String, String>) -> io::Result<()> {
    let names: Vec<&String> = messages.keys().map(|k| k).collect();

    writeln!(out, "mod messages {{")?;

    writeln!(out, "pub const SCREEN_NAMES: [&'static str; {}] = {:#?};",
        names.len(), names)?;
    writeln!(out, "pub const FULL_NAMES: [&'static str; {}] = {:#?};",
        names.len(), names.iter().map(|&n| &full_names[n]).collect::<Vec<_>>())?;
    writeln!(out, "pub const FULL_NAMES_TRUNC: [&'static str; {}] = {:#?};",
        names.len(), names.iter().map(|&n| trunc_full_name(&full_names[n])).collect::<Vec<_>>())?;

    for (i, name) in names.iter().enumerate() {
        let messages_by_name = &messages[*name];
        writeln!(out, "const MESSAGES_{}: [&'static str; {}] = {:#?};", i, messages_by_name.len(), messages_by_name)?;
    }

    writeln!(out, "pub fn get_full_name_full_name_trunc_messages(name: &str) -> Option<(&'static str, &'static str, &'static [&str])> {{")?;
    writeln!(out, "    match name {{")?;
    for (i, name) in names.iter().enumerate() {
        writeln!(out, "        {:?} => Some((FULL_NAMES[{}], FULL_NAMES_TRUNC[{}], &MESSAGES_{})),", name, i, i, i)?;
    }
    writeln!(out, "        _ => None")?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;

    writeln!(out, "}}")?;

    Ok(())
}

fn retrieve_messages(node: Handle, messages: &mut HashMap<String, Vec<String>>, full_names: &mut HashMap<String, String>) {
    if let NodeData::Element { ref name, ref attrs, ..  } = node.data {
        if name.local == local_name!("div") && class_attr_eq(&attrs.borrow(), "msg_item") {
            insert_message(node, messages, full_names);
            return;
        }
        if name.local == local_name!("head") {
            return;
        }
    }

    for child in node.children.borrow().iter() {
        retrieve_messages(child.clone(), messages, full_names);
    }
}

fn insert_message(node: Handle, messages: &mut HashMap<String, Vec<String>>, full_names: &mut HashMap<String, String>) {
    let mut full_name = String::new();
    let mut screen_name = String::new();
    let mut msg_body = String::new();

    for child in node.children.borrow().iter() {
        if let NodeData::Element { ref name, ref attrs, .. } = child.data {
            if name.local != local_name!("div") {
                continue;
            }
            if class_attr_eq(&attrs.borrow(), "from") {
                let inner = child.children.borrow();
                assert!(inner.len() == 6, "expected .from to have 6 child nodes, got {}", inner.len());

                full_name = 
                    if let NodeData::Text { ref contents } = inner[1].children.borrow()[0].data {
                        contents.borrow().to_string()
                    }
                    else {
                        panic!("Expected the 2nd .from child to contain a text node");
                    };

                screen_name =
                    if let NodeData::Text { ref contents } = inner[3].children.borrow()[0].data {
                        contents.borrow()[1..].to_string()
                    }
                    else {
                        panic!("Expected the 4th .from child to contain a text node")
                    };

                continue;
            }
            if class_attr_eq(&attrs.borrow(), "msg_body") {
                for body_child in child.children.borrow().iter() {
                    match body_child.data {
                        NodeData::Text { ref contents } => {
                            msg_body = [msg_body, contents.borrow().to_string()].concat();
                        },
                        NodeData::Element { ref name, ref attrs, .. } => {
                            if name.local == local_name!("div") && class_attr_eq(&attrs.borrow(), "emoji") {
                                let alt = attr_value(&attrs.borrow(), local_name!("alt")).unwrap();
                                msg_body = [msg_body, alt].concat();
                            }
                            else if name.local == local_name!("br") {
                                msg_body = [msg_body, "\n".to_string()].concat();
                            }
                        },
                        _ => ()
                    }
                }
            }
        }
    }

    if msg_body != "" {
        if !full_names.contains_key(&screen_name) {
            full_names.insert(screen_name.clone(), full_name);
        }

        if let Some(by_user) = messages.get_mut(&screen_name) {
            by_user.push(msg_body);
        }
        else {
            messages.insert(screen_name, vec![msg_body]);
        }
    }
}

fn class_attr_eq(attrs: &Vec<html5ever::Attribute>, value: &str) -> bool {
    attrs.iter().any(|a| a.name.local == local_name!("class") && *a.value == *value)
}

fn attr_value(attrs: &Vec<html5ever::Attribute>, name: html5ever::LocalName) -> Option<String> {
    attrs.iter().find(|a| a.name.local == name).map(|a| a.value.to_string())
}
