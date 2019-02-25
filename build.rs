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

    retrieve_messages(dom.document, &mut messages_by_name);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("messages.rs");
    let mut f = File::create(&dest_path).unwrap();

    generate_code(&mut f, &messages_by_name).unwrap();
}

fn generate_code<W: io::Write>(out: &mut W, messages_by_name: &HashMap<String, Vec<String>>) -> io::Result<()> {
    let names: Vec<&String> = messages_by_name.keys().map(|k| k).collect();

    writeln!(out, "mod messages {{")?;

    writeln!(out, "pub const SCREEN_NAMES: [&'static str; {}] = {:#?};", names.len(), names)?;
    for (i, name) in names.iter().enumerate() {
        let messages = &messages_by_name[*name];
        writeln!(out, "const MESSAGES_{}: [&'static str; {}] = {:#?};", i, messages.len(), messages)?;
    }

    writeln!(out, "pub fn get_by_name(name: &str) -> Option<&'static [&str]> {{")?;
    writeln!(out, "    match name {{")?;
    for (i, name) in names.iter().enumerate() {
        writeln!(out, "        {:?} => Some(&MESSAGES_{}),", name, i)?;
    }
    writeln!(out, "        _ => None")?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;

    writeln!(out, "}}")?;

    Ok(())
}

fn retrieve_messages(node: Handle, messages: &mut HashMap<String, Vec<String>>) {
    if let NodeData::Element { ref name, ref attrs, ..  } = node.data {
        if name.local == local_name!("div") && class_attr_eq(&attrs.borrow(), "msg_item") {
            insert_message(node, messages);
            return;
        }
        if name.local == local_name!("head") {
            return;
        }
    }

    for child in node.children.borrow().iter() {
        retrieve_messages(child.clone(), messages);
    }
}

fn insert_message(node: Handle, messages: &mut HashMap<String, Vec<String>>) {
    let mut from_name: String = "".to_string();
    let mut msg_body: String = "".to_string();

    for child in node.children.borrow().iter() {
        if let NodeData::Element { ref name, ref attrs, .. } = child.data {
            if name.local != local_name!("div") {
                continue;
            }
            if class_attr_eq(&attrs.borrow(), "from") {
                let screen_name: Option<String> = child.children.borrow().iter()
                    .find(|n| match n.data {
                        NodeData::Element { ref name, .. } => name.local == local_name!("a"),
                        _ => false
                    })
                    .map(|n| match n.children.borrow()[0].data {
                        NodeData::Text { ref contents } => contents.borrow()[1..].to_string(),
                        _ => unimplemented!()
                    });

                from_name = screen_name.unwrap();
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
        if let Some(by_user) = messages.get_mut(&from_name) {
            by_user.push(msg_body);
        }
        else {
            messages.insert(from_name, vec![msg_body]);
        }
    }
}

fn class_attr_eq(attrs: &Vec<html5ever::Attribute>, value: &str) -> bool {
    attrs.iter().any(|a| a.name.local == local_name!("class") && *a.value == *value)
}

fn attr_value(attrs: &Vec<html5ever::Attribute>, name: html5ever::LocalName) -> Option<String> {
    attrs.iter().find(|a| a.name.local == name).map(|a| a.value.to_string())
}
