use std::{io, env, fs::File, path::Path};
use std::collections::HashMap;

use html5ever::{ParseOpts, parse_document, local_name};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::tendril::TendrilSink;

struct MessageData<'a> {
    ignored_names: Vec<&'a str>,
    full_names: HashMap<String, String>,
    messages: HashMap<String, Vec<String>>
}

fn main() {
    let ignored_names_csv = env::var("TAKI_IGNORE_NAMES").unwrap_or(String::new());
    let ignored_names: Vec<&str> = ignored_names_csv.split(',').filter(|s| !s.is_empty()).collect();

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

    let mut data = MessageData { messages: HashMap::new(), full_names: HashMap::new(), ignored_names };

    retrieve_messages(dom.document, &mut data);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("messages.rs");
    let mut f = File::create(&dest_path).unwrap();

    generate_code(&mut f, &data.messages, &data.full_names).unwrap();
}

fn trunc_full_name(full_name: &str) -> String {
    let full_name_sep = full_name.chars().position(|c| c == ' ').unwrap_or(full_name.len() - 1);

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

fn retrieve_messages(node: Handle, data: &mut MessageData) {
    if let NodeData::Element { ref name, ref attrs, ..  } = node.data {
        if name.local == local_name!("div") && class_attr_eq(&attrs.borrow(), "msg_item") {
            insert_message(node, data);
            return;
        }
        if name.local == local_name!("head") {
            return;
        }
    }

    for child in node.children.borrow().iter() {
        retrieve_messages(child.clone(), data);
    }
}

fn insert_message(node: Handle, data: &mut MessageData) {
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
                            msg_body += &contents.borrow();
                        },
                        NodeData::Element { ref name, ref attrs, .. } => {
                            if name.local == local_name!("div") && class_attr_eq(&attrs.borrow(), "emoji") {
                                msg_body += &attr_value(&attrs.borrow(), local_name!("alt")).unwrap();
                            }
                            else if name.local == local_name!("br") {
                                msg_body += "\n";
                            }
                        },
                        _ => ()
                    }
                }
            }
        }
    }

    if msg_body != "" && !data.ignored_names.iter().any(|&ignored| ignored == screen_name) {
        if !data.full_names.contains_key(&screen_name) {
            data.full_names.insert(screen_name.clone(), full_name);
        }

        if let Some(by_user) = data.messages.get_mut(&screen_name) {
            by_user.push(msg_body);
        }
        else {
            data.messages.insert(screen_name, vec![msg_body]);
        }
    }
}

fn class_attr_eq(attrs: &Vec<html5ever::Attribute>, value: &str) -> bool {
    attrs.iter().any(|a| a.name.local == local_name!("class") && *a.value == *value)
}

fn attr_value(attrs: &Vec<html5ever::Attribute>, name: html5ever::LocalName) -> Option<&tendril::StrTendril> {
    attrs.iter().find(|a| a.name.local == name).map(|a| &a.value)
}
