#![feature(old_orphan_check)]
extern crate glob;
extern crate "rustc-serialize" as rustc_serialize;

use std::borrow::IntoCow;
use std::collections::{HashMap, HashSet, hash_map};
use std::io::{BufferedReader, File};

#[derive(RustcDecodable)]
#[allow(dead_code)]
struct CrateInfo {
    name: String,
    vers: String,
    deps: Vec<DepInfo>,
    cksum: String,
    features: HashMap<String, Vec<String>>,
    yanked: bool,
}

#[derive(RustcDecodable)]
#[allow(dead_code)]
struct DepInfo {
    name: String,
    req: String,
    features: Vec<String>,
    optional: bool,
    default_features: bool,
    target: Option<String>,
    kind: Option<String>
}

fn main() {
    let mut opts = glob::MatchOptions::new();
    opts.require_literal_leading_dot = true;

    let mut crates = vec![];
    let mut edges = vec![];
    let mut interacts = HashSet::new();
    let mut rev_dep_count = HashMap::new();

    for path in glob::glob_with("crates.io-index/*/*/*", &opts)
                    .chain(glob::glob_with("crates.io-index/[12]/*", &opts)) {
        let file = File::open(&path).unwrap();
        let last_line = BufferedReader::new(file).lines().last().unwrap().unwrap();
        let crate_info: CrateInfo = rustc_serialize::json::decode(&*last_line).unwrap();

        crates.push(crate_info.name.clone());
        let mut deps = crate_info.deps.iter()
            .filter(|d| d.kind.as_ref().map_or(true, |s| &**s != "dev"))
            .map(|d| &d.name)
            .collect::<Vec<_>>();
        if !deps.is_empty() {
            interacts.insert(crate_info.name.clone());
        }

        deps.sort();
        deps.dedup();
        for &dep_name in deps.iter() {
            interacts.insert(dep_name.clone());
            edges.push((crate_info.name.clone(), dep_name.clone()));

            let count = match rev_dep_count.entry(dep_name.clone()) {
                hash_map::Entry::Occupied(o) => o.into_mut(),
                hash_map::Entry::Vacant(v) => v.set(0u)
            };
            *count += 1;
        }
    }

    println!("total crates: {}", crates.len());
    crates.retain(|name| interacts.contains(name));
    println!("filtered crates: {}", crates.len());

    println!("digraph cratesio {{");
    for krate in crates.iter() {
        let count = rev_dep_count.get(krate).map_or(0, |n| *n);
        println!("  {ident}[label=\"{name}\" href=\"https://crates.io/crates/{name}\" fontsize={size}]",
                 ident = krate.replace("-", "_"),
                 name = krate,
                 size = 14.0 + count as f64 / 2.0);
    }
    for &(ref source, ref target) in edges.iter() {
        println!("  {} -> {}", source.replace("-", "_"), target.replace("-", "_"))
    }

    println!("}}");
}
