use proc_macro::TokenStream;

use crate::parser::{
    parser::{Node, NodeType, Parser},
    scanner::Scanner,
    token::Token,
};

pub fn expand_template(path: String) -> TokenStream {
    let template_path_opt = if path.is_empty() {
        None
    } else {
        Some(path.replace('"', ""))
    };
    let template_path = template_path_opt.unwrap_or_else(|| "src/App.vue".to_string());
    let template = std::fs::read_to_string(template_path.clone())
        .unwrap_or_else(|_| panic!("Could not read template file: {template_path}"));

    let scanner = Scanner::new(template);
    let tokens: Vec<Token> = scanner.try_into().unwrap();
    let parser = Parser::new(tokens);
    let mut code: String = "".into();

    let root: Node = parser.try_into().unwrap();

    fn convert_children(code: &mut String, node: &Node) {
        match &node.node_type {
            NodeType::Tag(tag) => {
                if tag != "template" {
                    code.push_str(
                        format!(
                            "
                    let e = document.create_element(\"{tag}\").unwrap();
                    parents.last().unwrap().append_child(&e).unwrap();
                    parents.push(e);
                    "
                        )
                        .as_str(),
                    );
                }

                for child in &node.children {
                    convert_children(code, child);
                }

                code.push_str("parents.pop();");
            }
            NodeType::Attribute(name, value, _) => {
                if name == "v-model" {
                    let sig = value.as_ref().unwrap().value.as_ref().unwrap();

                    code.push_str(
                        format!(
                            r#"
    let cloned_{sig} = msg.clone();

    parents
        .last()
        .unwrap()
        .add_event_listener_with_callback(
            "keypress",
            &Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {{
                let input = event
                    .current_target()
                    .unwrap()
                    .dyn_into::<web_sys::HtmlInputElement>()
                    .unwrap();

                cloned_{sig}.set(input.value().parse::<i32>().unwrap());
            }})
            .into_js_value()
            .as_ref()
            .unchecked_ref(),
        )
        .unwrap();
                        "#,
                        )
                        .as_str(),
                    );
                }

                code.push_str(
                    format!(
                        "
                     parents.last().unwrap().set_attribute(\"{}\", \"{}\").unwrap();",
                        name,
                        if let Some(token) = value {
                            token.value.as_ref().unwrap()
                        } else {
                            ""
                        }
                    )
                    .as_str(),
                );
            }
            NodeType::Text(text) => {
                code.push_str(
                    format!(
                        "
                    let e = document.create_text_node(\"{}\");
                    let p = parents.last().unwrap().clone();
                    p.append_child(&e).unwrap();

                    let document_clone = document.clone();
                    let future = msg.signal().for_each(move |value| {{
                        // This code is run for the current value of my_state,
                        // and also every time my_state changes


                        let n = document_clone.create_text_node(&format!(\"{{}}\", value));
                        p.append_child(&n).unwrap();
                        // p.remove_child(&e).unwrap();

                        async {{}}
                    }});
                    spawn_local(future);
                    ",
                        &text.escape_default()
                    )
                    .as_str(),
                );
            }
            _ => {}
        }
    }

    for child in &root.children {
        convert_children(&mut code, child);
    }

    format!(
        "fn template(document: web_sys::Document, root: web_sys::Element) {{
            // Stack of parents since nodes as nested and we basically emulate recursion
            use futures_signals::signal::Mutable;
            use futures_signals::signal::SignalExt;
            use wasm_bindgen_futures::spawn_local;

            let msg = Mutable::new(1);
            let mut parents = vec![root];
            {code}
        }}"
    )
    .parse()
    .unwrap()
}
