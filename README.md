# domtree

**MTX domtree** is a fast Rust CLI for turning HTML into a readable DOM tree and quickly inspecting forms, links, scripts, IDs, classes, and basic statistics.

## Features

- Fast HTML5 parsing with Rust
- Compact `tag#id.class` tree output
- Optional colors
- Search by tag, `#id`, or `.class`
- Ancestor paths for matches
- Form/action/method/input inspection
- Link extraction
- JavaScript source inspection
- DOM statistics
- CTF analysis mode
- Reads from stdin or files
- No Python runtime required

## Install

```bash
sudo apt update
sudo apt install cargo
./install.sh
```

## Usage

```bash
curl -s http://TARGET/ | domtree
```

Colored:

```bash
curl -s http://TARGET/ | domtree --color
```

From a file:

```bash
domtree page.html
```

Find an element:

```bash
domtree page.html --find '#resetBtn'
domtree page.html --find '.panel'
domtree page.html --find button
```

Show its ancestor path:

```bash
domtree page.html --find '#resetBtn' --path
```

Statistics:

```bash
domtree page.html --stats
```

Forms:

```bash
domtree page.html --forms
```

Links:

```bash
domtree page.html --links
```

Scripts:

```bash
domtree page.html --scripts
```

CTF summary:

```bash
curl -s http://TARGET/ | domtree --ctf
```

Limit depth:

```bash
domtree page.html --depth 3
```

Show text nodes:

```bash
domtree page.html --text
```

Show comments:

```bash
domtree page.html --comments
```

## Example

```text
└── html
    ├── head
    │   ├── meta
    │   ├── title
    │   └── link
    └── body
        ├── div.app
        │   ├── header.topbar
        │   │   └── div.brand
        │   └── main.layout
        │       ├── section.board-wrap
        │       └── aside.panel
        └── script
```

## Author

**MTX**

## License

MIT
