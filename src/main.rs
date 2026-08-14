use clap::{ArgGroup, Parser};
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Parser, Debug)]
#[command(
    name = "domtree",
    version,
    author = "MTX",
    about = "Fast HTML DOM tree viewer and analyzer",
    long_about = "Render HTML as a readable DOM tree and inspect forms, links, scripts and statistics."
)]
#[command(group(
    ArgGroup::new("analysis")
        .args(["forms", "links", "scripts", "stats", "ctf"])
        .multiple(true)
))]
struct Args {
    /// HTML file. If omitted, read from stdin.
    file: Option<String>,

    /// Enable syntax-like colors.
    #[arg(long)]
    color: bool,

    /// Show text nodes.
    #[arg(long)]
    text: bool,

    /// Show HTML comments.
    #[arg(long)]
    comments: bool,

    /// Maximum DOM depth.
    #[arg(long)]
    depth: Option<usize>,

    /// Find elements by tag, #id or .class.
    #[arg(long)]
    find: Option<String>,

    /// Show ancestor path for matching elements.
    #[arg(long)]
    path: bool,

    /// Show form/action/method/input information.
    #[arg(long)]
    forms: bool,

    /// Show links.
    #[arg(long)]
    links: bool,

    /// Show external and inline JavaScript.
    #[arg(long)]
    scripts: bool,

    /// Show DOM statistics.
    #[arg(long)]
    stats: bool,

    /// CTF-oriented summary: stats + forms + links + scripts.
    #[arg(long)]
    ctf: bool,
}

#[derive(Clone, Debug)]
struct ElementInfo {
    tag: String,
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<(String, String)>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input = read_input(args.file.as_deref())?;
    if input.trim().is_empty() {
        eprintln!("domtree: input is empty");
        std::process::exit(1);
    }

    let dom = parse_document(RcDom::default(), Default::default()).one(input);
    let color = args.color;

    let do_ctf = args.ctf;
    let show_tree = !do_ctf
        && args.find.is_none()
        && !args.forms
        && !args.links
        && !args.scripts
        && !args.stats;

    if show_tree {
        let mut out = StandardStream::stdout(if color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        });

        let nodes: Vec<_> = dom
            .document
            .children
            .borrow()
            .iter()
            .filter(|n| visible(n, args.text, args.comments))
            .cloned()
            .collect();

        for (i, node) in nodes.iter().enumerate() {
            render(
                &mut out,
                node,
                "",
                i + 1 == nodes.len(),
                0,
                &args,
            )?;
        }
    }

    if let Some(query) = args.find.as_deref() {
        find_elements(&dom.document, query, args.path, color)?;
    }

    if args.stats || do_ctf {
        print_stats(&dom.document);
    }

    if args.forms || do_ctf {
        print_forms(&dom.document, color);
    }

    if args.links || do_ctf {
        print_links(&dom.document, color);
    }

    if args.scripts || do_ctf {
        print_scripts(&dom.document, color);
    }

    Ok(())
}

fn read_input(file: Option<&str>) -> io::Result<String> {
    match file {
        Some(path) => fs::read_to_string(path),
        None => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            Ok(s)
        }
    }
}

fn visible(node: &Handle, text: bool, comments: bool) -> bool {
    match &node.data {
        NodeData::Element { .. } => true,
        NodeData::Text { contents } => text && !contents.borrow().trim().is_empty(),
        NodeData::Comment { .. } => comments,
        _ => false,
    }
}

fn element_info(node: &Handle) -> Option<ElementInfo> {
    if let NodeData::Element { name, attrs, .. } = &node.data {
        let attrs = attrs.borrow();
        let mut id = None;
        let mut classes = Vec::new();
        let mut all = Vec::new();

        for a in attrs.iter() {
            let k = a.name.local.to_string();
            let v = a.value.to_string();
            if k == "id" {
                id = Some(v.clone());
            } else if k == "class" {
                classes.extend(v.split_whitespace().map(ToOwned::to_owned));
            }
            all.push((k, v));
        }

        Some(ElementInfo {
            tag: name.local.to_string(),
            id,
            classes,
            attrs: all,
        })
    } else {
        None
    }
}

fn label(info: &ElementInfo) -> String {
    let mut s = info.tag.clone();
    if let Some(id) = &info.id {
        s.push('#');
        s.push_str(id);
    }
    for c in &info.classes {
        s.push('.');
        s.push_str(c);
    }
    s
}

fn print_label(out: &mut StandardStream, info: &ElementInfo, color: bool) -> io::Result<()> {
    if !color {
        write!(out, "{}", info.tag)?;
    } else {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Cyan));
        out.set_color(&spec)?;
        write!(out, "{}", info.tag)?;
    }

    if let Some(id) = &info.id {
        if color {
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Yellow));
            out.set_color(&spec)?;
        }
        write!(out, "#{id}")?;
    }

    for class in &info.classes {
        if color {
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Green));
            out.set_color(&spec)?;
        }
        write!(out, ".{class}")?;
    }

    out.reset()
}

fn render(
    out: &mut StandardStream,
    node: &Handle,
    prefix: &str,
    last: bool,
    depth: usize,
    args: &Args,
) -> io::Result<()> {
    if let Some(max) = args.depth {
        if depth > max {
            return Ok(());
        }
    }

    let connector = if last { "└── " } else { "├── " };
    write!(out, "{prefix}{connector}")?;

    match &node.data {
        NodeData::Element { .. } => {
            if let Some(info) = element_info(node) {
                print_label(out, &info, args.color)?;
            }
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().trim().replace('\n', " ");
            if args.color {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::White));
                out.set_color(&spec)?;
            }
            write!(out, "\"{text}\"")?;
            out.reset()?;
        }
        NodeData::Comment { contents } => {
            if args.color {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Magenta));
                out.set_color(&spec)?;
            }
            write!(out, "<!-- {} -->", contents)?;
            out.reset()?;
        }
        _ => {}
    }

    writeln!(out)?;

    let child_prefix = if last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };

    let children: Vec<_> = node
        .children
        .borrow()
        .iter()
        .filter(|n| visible(n, args.text, args.comments))
        .cloned()
        .collect();

    for (i, child) in children.iter().enumerate() {
        render(
            out,
            child,
            &child_prefix,
            i + 1 == children.len(),
            depth + 1,
            args,
        )?;
    }

    Ok(())
}

fn walk<F: FnMut(&Handle)>(node: &Handle, f: &mut F) {
    f(node);
    let children: Vec<_> = node.children.borrow().iter().cloned().collect();
    for child in children {
        walk(&child, f);
    }
}

fn print_stats(root: &Handle) {
    let mut elements = 0usize;
    let mut ids = 0usize;
    let mut classes = 0usize;
    let mut scripts = 0usize;
    let mut forms = 0usize;
    let mut links = 0usize;
    let mut inputs = 0usize;
    let mut max_depth = 0usize;
    let mut tags = BTreeMap::<String, usize>::new();

    fn depth(node: &Handle, d: usize, max: &mut usize) {
        *max = (*max).max(d);
        let children: Vec<_> = node.children.borrow().iter().cloned().collect();
        for c in children {
            depth(&c, d + 1, max);
        }
    }

    walk(root, &mut |n| {
        if let Some(info) = element_info(n) {
            elements += 1;
            if info.id.is_some() { ids += 1; }
            classes += info.classes.len();
            *tags.entry(info.tag.clone()).or_default() += 1;
            match info.tag.as_str() {
                "script" => scripts += 1,
                "form" => forms += 1,
                "a" => links += 1,
                "input" | "button" | "textarea" | "select" => inputs += 1,
                _ => {}
            }
        }
    });
    depth(root, 0, &mut max_depth);

    println!("DOM Statistics");
    println!("──────────────");
    println!("Elements : {elements}");
    println!("IDs      : {ids}");
    println!("Classes  : {classes}");
    println!("Scripts  : {scripts}");
    println!("Forms    : {forms}");
    println!("Links    : {links}");
    println!("Inputs   : {inputs}");
    println!("Depth    : {max_depth}");
    println!();
    println!("Tags:");

    for (tag, count) in tags {
        println!("  {tag:<16} {count}");
    }
    println!();
}

fn attr(info: &ElementInfo, name: &str) -> Option<String> {
    info.attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn print_forms(root: &Handle, color: bool) {
    println!("Forms");
    println!("─────");

    let mut count = 0usize;

    walk(root, &mut |n| {
        if let Some(info) = element_info(n) {
            if info.tag == "form" {
                count += 1;
                let method = attr(&info, "method").unwrap_or_else(|| "GET".to_string());
                let action = attr(&info, "action").unwrap_or_else(|| "(current URL)".to_string());
                println!("FORM #{}", count);
                println!("  method : {}", method.to_uppercase());
                println!("  action : {action}");

                let mut inputs = Vec::new();
                collect_descendants(n, &mut |x| {
                    if let Some(i) = element_info(x) {
                        if matches!(i.tag.as_str(), "input" | "textarea" | "select" | "button") {
                            inputs.push(i);
                        }
                    }
                });

                for input in inputs {
                    let name = attr(&input, "name").unwrap_or_else(|| "(unnamed)".to_string());
                    let typ = attr(&input, "type").unwrap_or_else(|| input.tag.clone());
                    println!("    ├── {name} [{typ}]");
                }
                println!();
            }
        }
    });

    if count == 0 {
        println!("No forms found.\n");
    }
    let _ = color;
}

fn print_links(root: &Handle, _color: bool) {
    println!("Links");
    println!("─────");

    let mut found = false;
    walk(root, &mut |n| {
        if let Some(info) = element_info(n) {
            if info.tag == "a" {
                if let Some(href) = attr(&info, "href") {
                    println!("  {href}");
                    found = true;
                }
            }
        }
    });

    if !found {
        println!("  No links found");
    }
    println!();
}

fn print_scripts(root: &Handle, _color: bool) {
    println!("JavaScript");
    println!("──────────");

    let mut found = false;

    walk(root, &mut |n| {
        if let Some(info) = element_info(n) {
            if info.tag == "script" {
                found = true;
                if let Some(src) = attr(&info, "src") {
                    println!("  external : {src}");
                } else {
                    let inline = text_content(n).trim().replace('\n', " ");
                    let preview: String = inline.chars().take(100).collect();
                    println!("  inline   : {}", if preview.is_empty() { "(empty)" } else { &preview });
                }
            }
        }
    });

    if !found {
        println!("  No scripts found");
    }
    println!();
}

fn collect_descendants<F: FnMut(&Handle)>(root: &Handle, f: &mut F) {
    let children: Vec<_> = root.children.borrow().iter().cloned().collect();
    for child in children {
        f(&child);
        collect_descendants(&child, f);
    }
}

fn text_content(node: &Handle) -> String {
    let mut out = String::new();
    walk(node, &mut |n| {
        if let NodeData::Text { contents } = &n.data {
            out.push_str(&contents.borrow());
            out.push(' ');
        }
    });
    out
}

fn find_elements(root: &Handle, query: &str, show_path: bool, color: bool) -> io::Result<()> {
    let mut matches = Vec::<Handle>::new();

    walk(root, &mut |n| {
        if let Some(info) = element_info(n) {
            if matches_query(&info, query) {
                matches.push(n.clone());
            }
        }
    });

    println!("Matches: {}", matches.len());
    println!("────────");

    if matches.is_empty() {
        return Ok(());
    }

    let mut out = StandardStream::stdout(if color {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    });

    for node in matches {
        if show_path {
            let path = ancestor_path(&node);
            for (i, info) in path.iter().enumerate() {
                if i > 0 {
                    print!(" → ");
                }
                print!("{}", label(info));
            }
            println!();
        } else if let Some(info) = element_info(&node) {
            print_label(&mut out, &info, color)?;
            writeln!(out)?;
        }
    }

    Ok(())
}

fn matches_query(info: &ElementInfo, q: &str) -> bool {
    if let Some(id) = q.strip_prefix('#') {
        return info.id.as_deref() == Some(id);
    }

    if let Some(class) = q.strip_prefix('.') {
        return info.classes.iter().any(|c| c == class);
    }

    let mut tag = q;
    let mut wanted_id = None;
    let mut wanted_class = None;

    if let Some(pos) = tag.find('#') {
        wanted_id = Some(&tag[pos + 1..]);
        tag = &tag[..pos];
    }
    if let Some(pos) = tag.find('.') {
        wanted_class = Some(&tag[pos + 1..]);
        tag = &tag[..pos];
    }

    if !tag.is_empty() && !info.tag.eq_ignore_ascii_case(tag) {
        return false;
    }

    if let Some(id) = wanted_id {
        if info.id.as_deref() != Some(id) {
            return false;
        }
    }

    if let Some(class) = wanted_class {
        if !info.classes.iter().any(|c| c == class) {
            return false;
        }
    }

    true
}

fn ancestor_path(node: &Handle) -> Vec<ElementInfo> {
    let mut result = Vec::new();
    let mut current = Some(node.clone());

    while let Some(n) = current {
        if let Some(info) = element_info(&n) {
            result.push(info);
        }
        current = n.parent.get().and_then(|p| p.upgrade());
    }

    result.reverse();
    result
}
