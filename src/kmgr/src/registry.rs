use crate::util::{
    basename, is_dir, is_writable, join, list_dir, path_exists, read_str, scaled, trailing_number,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const FILES: &[&str] = &[
    include_str!("../../../tunables.d/05-sources.json"),
    include_str!("../../../tunables.d/10-cpu.json"),
    include_str!("../../../tunables.d/20-gpu.json"),
    include_str!("../../../tunables.d/30-io.json"),
    include_str!("../../../tunables.d/40-memory.json"),
    include_str!("../../../tunables.d/50-scheduler.json"),
    include_str!("../../../tunables.d/60-misc.json"),
    include_str!("../../../tunables.d/70-display.json"),
    include_str!("../../../tunables.d/80-features.json"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Int,
    Bool,
    Enum,
    EnumBracket,
    Freq,
    Str,
    Dir,
    GlobFiles,
    AutoGlob,
    Rgb,
    Custom,
    EnumInt,
}

impl Kind {
    fn parse(s: &str) -> Kind {
        match s {
            "int" => Kind::Int,
            "bool" => Kind::Bool,
            "enum" => Kind::Enum,
            "enum_bracket" => Kind::EnumBracket,
            "freq" => Kind::Freq,
            "dir" => Kind::Dir,
            "glob_files" => Kind::GlobFiles,
            "auto_glob" => Kind::AutoGlob,
            "rgb" => Kind::Rgb,
            "custom" => Kind::Custom,
            "enum_int" => Kind::EnumInt,
            _ => Kind::Str,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Int => "int",
            Kind::Bool => "bool",
            Kind::Enum => "enum",
            Kind::EnumBracket => "enum_bracket",
            Kind::Freq => "freq",
            Kind::Str => "string",
            Kind::Dir => "dir",
            Kind::GlobFiles => "glob_files",
            Kind::AutoGlob => "auto_glob",
            Kind::Rgb => "rgb",
            Kind::Custom => "custom",
            Kind::EnumInt => "enum_int",
        }
    }
    fn is_writable_node(&self) -> bool {
        !matches!(self, Kind::Custom)
    }
}

#[derive(Clone, Debug)]
pub struct Child {
    pub label: String,
    pub path: String,
    pub value: Option<String>,
    pub kind: Kind,
    pub writable: bool,
    pub persisted: bool,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub scope: String,
    pub key: String,
    pub label: String,
    pub kind: Kind,
    pub path: Option<String>,
    pub value: Option<String>,
    pub children: Vec<Child>,
    pub choices: Vec<(String, String)>,
    pub labels: BTreeMap<String, String>,
    pub enum_values: Vec<(i64, String)>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub step: Option<i64>,
    pub unit: Option<String>,
    pub display_unit: Option<String>,
    pub scale: Option<i64>,
    pub write_order: Option<String>,
    pub handler: Option<String>,
    pub bundle: Option<String>,
    pub zero_label: Option<String>,
    pub help: Option<String>,
    pub risk: Option<String>,
    pub is_dir: bool,
    pub persisted: bool,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub id: String,
    pub scope: String,
    pub title: String,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub sections: Vec<Section>,
}

pub struct Registry {
    pub groups: Vec<Group>,
    pub sources: BTreeMap<String, Vec<String>>,
}

impl Registry {
    pub fn find(&self, key: &str) -> Option<&Entry> {
        for g in &self.groups {
            for s in &g.sections {
                for e in &s.entries {
                    if e.key == key {
                        return Some(e);
                    }
                }
            }
        }
        None
    }

    pub fn find_by_id(&self, id: &str) -> Vec<&Entry> {
        let mut v = Vec::new();
        for g in &self.groups {
            for s in &g.sections {
                for e in &s.entries {
                    if e.id == id || e.key == id {
                        v.push(e);
                    }
                }
            }
        }
        v
    }

    pub fn resolved_sources(&self) -> BTreeMap<String, String> {
        self.sources
            .iter()
            .filter_map(|(k, v)| {
                v.iter()
                    .find(|p| crate::util::path_exists(p))
                    .map(|p| (k.clone(), p.clone()))
            })
            .collect()
    }

    pub fn to_json(&self) -> Value {
        let groups: Vec<Value> = self
            .groups
            .iter()
            .map(|g| {
                let sections: Vec<Value> = g
                    .sections
                    .iter()
                    .filter(|s| !s.entries.is_empty())
                    .map(|s| {
                        json!({
                            "id": s.id,
                            "scope": s.scope,
                            "title": s.title,
                            "entries": s.entries.iter().map(entry_json).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                json!({"id": g.id, "title": g.title, "sections": sections})
            })
            .collect();
        json!({"groups": groups})
    }
}

fn entry_json(e: &Entry) -> Value {
    let display = display_value(e);
    let choices: Vec<Value> = e
        .choices
        .iter()
        .map(|(v, l)| json!({"value": v, "label": l}))
        .collect();
    let children: Vec<Value> = e
        .children
        .iter()
        .map(|c| {
            json!({
                "label": c.label,
                "path": c.path,
                "value": c.value,
                "type": c.kind.as_str(),
                "writable": c.writable,
                "persisted": c.persisted,
            })
        })
        .collect();
    json!({
        "id": e.id,
        "scope": e.scope,
        "key": e.key,
        "label": e.label,
        "type": e.kind.as_str(),
        "path": e.path,
        "value": effective_value(e),
        "display": display,
        "choices": choices,
        "labels": e.labels,
        "min": e.min,
        "max": e.max,
        "step": e.step,
        "unit": e.unit,
        "display_unit": e.display_unit,
        "scale": e.scale,
        "write_order": e.write_order,
        "handler": e.handler,
        "bundle": e.bundle,
        "help": e.help,
        "risk": e.risk,
        "is_dir": e.is_dir,
        "persisted": e.persisted,
        "children": children,
    })
}

pub fn effective_value(e: &Entry) -> Option<String> {
    let v = e.value.as_deref()?;
    match e.kind {
        Kind::EnumBracket => v
            .split_whitespace()
            .find(|t| t.starts_with('['))
            .map(|t| t.trim_matches(|c| c == '[' || c == ']').to_string())
            .or_else(|| Some(v.to_string())),
        _ => Some(v.to_string()),
    }
}

fn display_value(e: &Entry) -> Option<String> {
    let v = e.value.as_deref()?;
    match e.kind {
        Kind::EnumBracket => effective_value(e),
        Kind::Bool => Some(
            e.labels
                .get(v)
                .cloned()
                .unwrap_or_else(|| if v == "0" { "Disabled".into() } else { "Enabled".into() }),
        ),
        Kind::Freq => {
            let n: i64 = v.parse().ok()?;
            Some(scaled(n, e.scale.unwrap_or(1), e.display_unit.as_deref().unwrap_or("")))
        }
        Kind::EnumInt => {
            let n: i64 = v.parse().ok()?;
            let label = e
                .enum_values
                .iter()
                .find(|(val, _)| *val == n)
                .map(|(_, l)| l.clone())
                .unwrap_or_else(|| format!("Unknown ({})", n));
            Some(label)
        }
        Kind::Int => {
            if v == "0" {
                if let Some(z) = &e.zero_label {
                    return Some(z.clone());
                }
            }
            if let Some(du) = &e.display_unit {
                let n: i64 = v.parse().ok()?;
                return Some(scaled(n, e.scale.unwrap_or(1), du));
            }
            Some(v.to_string())
        }
        _ => Some(v.to_string()),
    }
}

// ---------------------------------------------------------------- loading

fn load_raw() -> (Vec<(String, String, Vec<Value>)>, BTreeMap<String, Vec<String>>) {
    let mut groups = Vec::new();
    let mut sources = BTreeMap::new();
    for raw in FILES {
        let v: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(src) = v.get("sources").and_then(|s| s.as_object()) {
            for (k, val) in src {
                if let Some(arr) = val.as_array() {
                    sources.insert(
                        k.clone(),
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect(),
                    );
                }
            }
        }
        if let Some(secs) = v.get("sections").and_then(|s| s.as_array()) {
            let id = v.get("group").and_then(|g| g.as_str()).unwrap_or("").to_string();
            let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
            groups.push((id, title, secs.clone()));
        }
    }
    (groups, sources)
}

pub fn resolve() -> Registry {
    let (raw_groups, sources) = load_raw();
    let mut groups: Vec<Group> = Vec::new();
    let mut declared: BTreeSet<String> = BTreeSet::new();
    let mut deferred: Vec<(usize, usize, Value, String, String)> = Vec::new();

    for (gid, gtitle, sections) in &raw_groups {
        let mut g = Group {
            id: gid.clone(),
            title: gtitle.clone(),
            sections: Vec::new(),
        };
        for sec in sections {
            let sid = sec.get("id").and_then(|s| s.as_str()).unwrap_or("sec").to_string();
            let entries_spec = sec
                .get("entries")
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();

            let expands: Vec<(String, String)> = match sec.get("foreach") {
                Some(f) => expand_foreach(f),
                None => vec![(String::new(), String::new())],
            };

            for (dir, f_title) in expands {
                let name = if dir.is_empty() { String::new() } else { basename(&dir) };
                let scope = if dir.is_empty() {
                    format!("{}.{}", gid, sid)
                } else {
                    format!("{}.{}.{}", gid, sid, name)
                };
                let title = if dir.is_empty() {
                    sec.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string()
                } else if !f_title.is_empty() {
                    f_title
                } else {
                    sec.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string()
                };
                let mut entries: Vec<Entry> = Vec::new();
                let mut cur: BTreeMap<String, String> = BTreeMap::new();
                for spec in &entries_spec {
                    let kind = Kind::parse(spec.get("type").and_then(|t| t.as_str()).unwrap_or("int"));
                    if kind == Kind::AutoGlob {
                        deferred.push((
                            groups.len(),
                            g.sections.len(),
                            spec.clone(),
                            scope.clone(),
                            title.clone(),
                        ));
                        continue;
                    }
                    if let Some(e) = resolve_entry(spec, &dir, &name, &scope, &cur) {
                        if let Some(p) = &e.path {
                            declared.insert(p.clone());
                        }
                        if let Some(v) = &e.value {
                            cur.insert(e.id.clone(), v.clone());
                        }
                        entries.push(e);
                    }
                }
                g.sections.push(Section {
                    id: sid.clone(),
                    scope,
                    title,
                    entries,
                });
            }
        }
        groups.push(g);
    }

    for (gi, si, spec, scope, _title) in deferred {
        let entries = resolve_auto_glob(&spec, &scope, &declared);
        if let Some(g) = groups.get_mut(gi) {
            if let Some(s) = g.sections.get_mut(si) {
                s.entries.extend(entries);
            }
        }
    }

    groups.retain(|g| g.sections.iter().any(|s| !s.entries.is_empty()));

    let applied = crate::store::load_applied();
    for g in &mut groups {
        for s in &mut g.sections {
            for e in &mut s.entries {
                e.persisted = applied.contains_key(&e.key);
                for c in &mut e.children {
                    c.persisted = applied.contains_key(&c.path);
                }
            }
        }
    }

    Registry { groups, sources }
}

// ---------------------------------------------------------------- foreach

fn expand_foreach(f: &Value) -> Vec<(String, String)> {
    let glob = f.get("glob").and_then(|g| g.as_str()).unwrap_or("");
    let order = f.get("order").and_then(|o| o.as_str()).unwrap_or("alpha");
    let titles: Vec<String> = f
        .get("titles")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let fallback = f
        .get("title_fallback")
        .and_then(|t| t.as_str())
        .unwrap_or("Item {i}")
        .to_string();

    let mut matches = glob_paths(glob);
    if order == "numeric" {
        matches.sort_by_key(|p| trailing_number(&basename(p)));
    }
    matches
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let title = titles.get(i).cloned().unwrap_or_else(|| {
                fallback
                    .replace("{i}", &i.to_string())
                    .replace("{name}", &basename(p))
            });
            (p.clone(), title)
        })
        .collect()
}

// ---------------------------------------------------------------- glob

fn has_wildcard(s: &str) -> bool {
    s.contains(['*', '?', '['])
}

fn glob_paths(pattern: &str) -> Vec<String> {
    let parts: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    let mut current: Vec<String> = vec!["/".to_string()];
    for (idx, part) in parts.iter().enumerate() {
        let is_last = idx == parts.len() - 1;
        let mut next: Vec<String> = Vec::new();
        for base in &current {
            if !has_wildcard(part) {
                let cand = join(base, part);
                if (is_last && path_exists(&cand)) || (!is_last && is_dir(&cand)) {
                    next.push(cand);
                }
            } else {
                for name in list_dir(base) {
                    if component_match(part, &name) {
                        let cand = join(base, &name);
                        if (is_last && path_exists(&cand)) || (!is_last && is_dir(&cand)) {
                            next.push(cand);
                        }
                    }
                }
            }
        }
        current = next;
        if current.is_empty() {
            break;
        }
    }
    current
}

fn component_match(pat: &str, name: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let n: Vec<char> = name.chars().collect();
    fn go(p: &[char], n: &[char]) -> bool {
        if p.is_empty() {
            return n.is_empty();
        }
        match p[0] {
            '*' => (0..=n.len()).any(|i| go(&p[1..], &n[i..])),
            '?' => !n.is_empty() && go(&p[1..], &n[1..]),
            '[' => {
                if let Some(end) = p.iter().position(|&c| c == ']') {
                    if n.is_empty() {
                        return false;
                    }
                    let class = &p[1..end];
                    let neg = matches!(class.first(), Some(&'!') | Some(&'^'));
                    let cls = if neg { &class[1..] } else { class };
                    let mut matched = false;
                    let mut i = 0;
                    while i < cls.len() {
                        if i + 2 < cls.len() && cls[i + 1] == '-' {
                            if n[0] >= cls[i] && n[0] <= cls[i + 2] {
                                matched = true;
                            }
                            i += 3;
                        } else {
                            if n[0] == cls[i] {
                                matched = true;
                            }
                            i += 1;
                        }
                    }
                    if matched != neg {
                        go(&p[end + 1..], &n[1..])
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            c => !n.is_empty() && n[0] == c && go(&p[1..], &n[1..]),
        }
    }
    go(&p, &n)
}

// ---------------------------------------------------------------- locate

fn locate(loc: &Value, dir: &str, name: &str, i: usize) -> Option<String> {
    let names: Vec<String> = loc
        .get("names")
        .and_then(|n| n.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let roots: Vec<String> = loc
        .get("roots")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let max_depth = loc.get("max_depth").and_then(|d| d.as_u64()).unwrap_or(3) as usize;

    for root in roots {
        let root = subst(&root, dir, name, i, &BTreeMap::new())?;
        for n in &names {
            if let Some(found) = find_file(&root, n, max_depth) {
                return Some(found);
            }
        }
    }
    None
}

fn find_file(root: &str, target: &str, max_depth: usize) -> Option<String> {
    if !is_dir(root) {
        return None;
    }
    let mut queue: Vec<(String, usize)> = vec![(root.to_string(), 0)];
    while let Some((d, depth)) = queue.pop() {
        for name in list_dir(&d) {
            let cand = join(&d, &name);
            if name == target && path_exists(&cand) {
                return Some(cand);
            }
            if depth + 1 < max_depth && is_dir(&cand) {
                queue.push((cand, depth + 1));
            }
        }
    }
    None
}

// ---------------------------------------------------------------- entry resolution

fn subst(
    s: &str,
    dir: &str,
    name: &str,
    i: usize,
    cur: &BTreeMap<String, String>,
) -> Option<String> {
    let mut out = s
        .replace("{dir}", dir)
        .replace("{name}", name)
        .replace("{i}", &i.to_string());
    while let Some(start) = out.find("{cur:") {
        let rel = out[start..].find('}')?;
        let end = start + rel;
        let key = out[start + 5..end].to_string();
        let val = cur.get(&key)?;
        out.replace_range(start..=end, val);
    }
    Some(out)
}

fn resolve_entry(
    spec: &Value,
    dir: &str,
    name: &str,
    scope: &str,
    cur: &BTreeMap<String, String>,
) -> Option<Entry> {
    let id = spec.get("id")?.as_str()?.to_string();
    let label = spec.get("label").and_then(|l| l.as_str()).unwrap_or(&id).to_string();
    let kind = Kind::parse(spec.get("type").and_then(|t| t.as_str()).unwrap_or("int"));
    let optional = spec.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);
    let verify = spec.get("verify").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut e = Entry {
        id: id.clone(),
        scope: scope.to_string(),
        key: format!("{}.{}", scope, id),
        label,
        kind,
        path: None,
        value: None,
        children: Vec::new(),
        choices: Vec::new(),
        labels: parse_labels(spec.get("labels")),
        enum_values: parse_enum_values(spec.get("values")),
        min: spec.get("min").and_then(|v| v.as_i64()),
        max: spec.get("max").and_then(|v| v.as_i64()),
        step: spec.get("step").and_then(|v| v.as_i64()),
        unit: spec.get("unit").and_then(|v| v.as_str()).map(String::from),
        display_unit: spec.get("display_unit").and_then(|v| v.as_str()).map(String::from),
        scale: spec.get("scale").and_then(|v| v.as_i64()),
        write_order: spec.get("write_order").and_then(|v| v.as_str()).map(String::from),
        handler: spec.get("handler").and_then(|v| v.as_str()).map(String::from),
        bundle: spec.get("bundle").and_then(|v| v.as_str()).map(String::from),
        zero_label: spec.get("zero_label").and_then(|v| v.as_str()).map(String::from),
        help: spec.get("help").and_then(|v| v.as_str()).map(String::from),
        risk: spec.get("risk").and_then(|v| v.as_str()).map(String::from),
        is_dir: false,
        persisted: false,
    };

    // Resolve path
    let path: Option<String> = if let Some(p) = spec.get("path").and_then(|v| v.as_str()) {
        subst(p, dir, name, 0, cur)
    } else if let Some(arr) = spec.get("paths").and_then(|v| v.as_array()) {
        arr.iter().filter_map(|v| v.as_str()).find_map(|p| {
            let pp = subst(p, dir, name, 0, cur)?;
            if path_exists(&pp) {
                Some(pp)
            } else {
                None
            }
        })
    } else if let Some(loc) = spec.get("locate") {
        locate(loc, dir, name, 0)
    } else {
        None
    };

    if kind == Kind::GlobFiles {
        let glob = spec.get("glob").and_then(|g| g.as_str()).unwrap_or("");
        let files: Vec<String> = spec
            .get("files")
            .and_then(|f| f.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        e.children = glob_file_children(glob, &files);
        if e.children.is_empty() {
            if verify {
                log_discover(&e.key, "no matching files");
            }
            return None;
        }
        e.path = None;
        e.is_dir = true;
        return Some(e);
    }

    if kind == Kind::Custom {
        e.path = None;
        e.is_dir = true;
        return Some(e);
    }

    let path = match path {
        Some(p) => p,
        None => {
            if verify {
                log_discover(&e.key, "path not found");
            }
            let _ = optional;
            return None;
        }
    };

    if !path_exists(&path) {
        if verify {
            log_discover(&e.key, &format!("{} missing", path));
        }
        return None;
    }

    e.is_dir = is_dir(&path);
    if kind.is_writable_node() && e.handler.is_none() && !e.is_dir && !is_writable(&path) {
        return None;
    }
    if kind == Kind::Dir && !e.is_dir {
        return None;
    }

    e.value = read_str(&path);
    e.path = Some(path.clone());

    // choices
    if let Some(cf) = spec.get("choices_from").and_then(|v| v.as_str()) {
        if let Some(cf) = subst(cf, dir, name, 0, cur) {
            if let Some(content) = read_str(&cf) {
                e.choices = build_choices(kind, &content, &e);
            }
        }
    } else if let Some(arr) = spec.get("choices").and_then(|v| v.as_array()) {
        e.choices = arr
            .iter()
            .filter_map(|x| x.as_str())
            .map(|s| (s.to_string(), s.to_string()))
            .collect();
    } else if kind == Kind::EnumInt {
        e.choices = e
            .enum_values
            .iter()
            .map(|(v, l)| (v.to_string(), l.clone()))
            .collect();
    }

    // children for container types
    match kind {
        Kind::Dir => {
            e.children = dir_children(&path);
        }
        Kind::GlobFiles => {
            let glob = spec.get("glob").and_then(|g| g.as_str()).unwrap_or("");
            let files: Vec<String> = spec
                .get("files")
                .and_then(|f| f.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            e.children = glob_file_children(glob, &files);
        }
        _ => {}
    }

    Some(e)
}

fn build_choices(kind: Kind, content: &str, e: &Entry) -> Vec<(String, String)> {
    match kind {
        Kind::EnumBracket => {
            let toks: Vec<String> = content
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '[' || c == ']').to_string())
                .collect();
            toks.into_iter().map(|t| (t.clone(), t)).collect()
        }
        Kind::Freq => {
            let mut v: Vec<i64> = content
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            v.sort();
            v.dedup();
            v.into_iter()
                .map(|n| {
                    (
                        n.to_string(),
                        scaled(n, e.scale.unwrap_or(1), e.display_unit.as_deref().unwrap_or("")),
                    )
                })
                .collect()
        }
        _ => content
            .split_whitespace()
            .map(|t| (t.to_string(), t.to_string()))
            .collect(),
    }
}

fn dir_children(path: &str) -> Vec<Child> {
    let mut out = Vec::new();
    for name in list_dir(path) {
        let p = join(path, &name);
        if is_dir(&p) {
            continue;
        }
        let value = read_str(&p);
        let kind = match &value {
            Some(v) if v.parse::<i64>().is_ok() => Kind::Int,
            _ => Kind::Str,
        };
        let writable = is_writable(&p);
        out.push(Child {
            label: name,
            path: p,
            value,
            kind,
            writable,
            persisted: false,
        });
    }
    out
}

fn glob_file_children(glob: &str, files: &[String]) -> Vec<Child> {
    let mut out = Vec::new();
    for dir in glob_paths(glob) {
        for f in files {
            let p = join(&dir, f);
            if path_exists(&p) {
                let value = read_str(&p);
                let kind = match &value {
                    Some(v) if v.parse::<i64>().is_ok() => Kind::Int,
                    _ => Kind::Str,
                };
                out.push(Child {
                    label: format!("{}/{}", basename(&dir), f),
                    path: p,
                    value,
                    kind,
                    writable: is_writable(&join(&dir, f)),
                    persisted: false,
                });
            }
        }
    }
    out
}

fn resolve_auto_glob(spec: &Value, scope: &str, declared: &BTreeSet<String>) -> Vec<Entry> {
    let id = spec.get("id").and_then(|v| v.as_str()).unwrap_or("auto").to_string();
    let label = spec
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Other tunables")
        .to_string();
    let roots: Vec<String> = spec
        .get("roots")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let name_glob = spec.get("name_glob").and_then(|n| n.as_str()).unwrap_or("*").to_string();
    let max_depth = spec.get("max_depth").and_then(|d| d.as_u64()).unwrap_or(2) as usize;
    let excludes: Vec<String> = spec
        .get("exclude")
        .and_then(|e| e.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let ensure: Vec<String> = spec
        .get("ensure_names")
        .and_then(|e| e.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let zero_label = spec.get("zero_label").and_then(|v| v.as_str()).map(String::from);

    let mut children: Vec<Child> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for root in &roots {
        for (p, depth) in walk(root, max_depth) {
            let bn = basename(&p);
            if depth == 0 {
                continue;
            }
            if !component_match(&name_glob, &bn) {
                continue;
            }
            if declared.contains(&p) || seen.contains(&p) {
                continue;
            }
            if excludes.iter().any(|ex| component_match(ex, &bn)) {
                continue;
            }
            if !is_writable(&p) {
                continue;
            }
            seen.insert(p.clone());
            let value = read_str(&p);
            let kind = match &value {
                Some(v) if v.parse::<i64>().is_ok() => Kind::Int,
                _ => Kind::Str,
            };
            children.push(Child {
                label: bn,
                path: p,
                value,
                kind,
                writable: true,
                persisted: false,
            });
        }
    }
    for n in ensure {
        if children.iter().any(|c| basename(&c.path) == n) {
            continue;
        }
        for root in &roots {
            if let Some(found) = find_file(root, &n, max_depth) {
                if declared.contains(&found) || seen.contains(&found) {
                    continue;
                }
                seen.insert(found.clone());
                let value = read_str(&found);
                let kind = match &value {
                    Some(v) if v.parse::<i64>().is_ok() => Kind::Int,
                    _ => Kind::Str,
                };
                let writable = is_writable(&found);
                children.push(Child {
                    label: n.clone(),
                    path: found,
                    value,
                    kind,
                    writable,
                    persisted: false,
                });
                break;
            }
        }
    }
    children.sort_by(|a, b| a.label.cmp(&b.label));

    if children.is_empty() {
        return Vec::new();
    }

    vec![Entry {
        id: id.clone(),
        scope: scope.to_string(),
        key: format!("{}.{}", scope, id),
        label,
        kind: Kind::AutoGlob,
        path: None,
        value: None,
        children,
        choices: Vec::new(),
        labels: BTreeMap::new(),
        enum_values: Vec::new(),
        min: None,
        max: None,
        step: None,
        unit: None,
        display_unit: None,
        scale: None,
        write_order: None,
        handler: None,
        bundle: None,
        zero_label,
        help: spec.get("help").and_then(|v| v.as_str()).map(String::from),
        risk: spec.get("risk").and_then(|v| v.as_str()).map(String::from),
        is_dir: true,
        persisted: false,
    }]
}

fn walk(root: &str, max_depth: usize) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    if !is_dir(root) {
        return out;
    }
    let mut stack: Vec<(String, usize)> = vec![(root.to_string(), 0)];
    while let Some((d, depth)) = stack.pop() {
        for name in list_dir(&d) {
            let cand = join(&d, &name);
            if is_dir(&cand) {
                if depth + 1 <= max_depth {
                    stack.push((cand, depth + 1));
                }
            } else {
                out.push((cand, depth + 1));
            }
        }
    }
    out
}

fn parse_labels(v: Option<&Value>) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    if let Some(o) = v.and_then(|x| x.as_object()) {
        for (k, val) in o {
            if let Some(s) = val.as_str() {
                m.insert(k.clone(), s.to_string());
            }
        }
    }
    m
}

fn parse_enum_values(v: Option<&Value>) -> Vec<(i64, String)> {
    let mut out = Vec::new();
    if let Some(arr) = v.and_then(|x| x.as_array()) {
        for item in arr {
            if let (Some(n), Some(l)) = (
                item.get("value").and_then(|x| x.as_i64()),
                item.get("label").and_then(|x| x.as_str()),
            ) {
                out.push((n, l.to_string()));
            }
        }
    }
    out
}

fn log_discover(key: &str, msg: &str) {
    let dir = crate::store::state_dir().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("discover.log"))
    {
        let _ = writeln!(f, "{}: {}", key, msg);
    }
}
