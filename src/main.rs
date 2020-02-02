#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

mod fetcher;

use regex::Regex;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, Write};
use std::path::Path;

/// main() helps to generate the submission template .rs
fn main() {
    println!("Welcome to leetcode-rust system.");
    let mut solved_ids = get_solved_ids();
    loop {
        println!("Please enter a frontend problem id, or \"random\" to generate a random one.");
        let mut is_random = false;
        let mut is_solving = false;
        let mut id: u32 = 0;
        let mut id_arg = String::new();
        io::stdin()
            .read_line(&mut id_arg)
            .expect("Failed to read line");
        let id_arg = id_arg.trim();

        let random_pattern = Regex::new(r"^random$").unwrap();
        let solving_pattern = Regex::new(r"^solve (\d+)$").unwrap();

        if random_pattern.is_match(id_arg) {
            println!("You select random mode.");
            id = generate_random_id(&solved_ids);
            is_random = true;
            println!("Generate random problem: {}", &id);
        } else if solving_pattern.is_match(id_arg) {
            // solve a problem
            // move it from problem/ to solution/
            is_solving = true;
            id = solving_pattern
                .captures(id_arg)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
        } else {
            id = id_arg
                .parse::<u32>()
                .unwrap_or_else(|_| panic!("not a number: {}", id_arg));
            if solved_ids.contains(&id) {
                println!(
                    "The problem you chose is invalid (the problem may have been initialized \
                     or may have no rust version)."
                );
                continue;
            }
        }

        let problem = fetcher::get_problem(id).unwrap_or_else(|| {
            panic!(
                "Error: failed to get problem #{} \
                 (The problem may be paid-only or may not be exist).",
                id
            )
        });
        let code = problem.code_definition.iter().find(|&d| d.value == "rust");
        if code.is_none() {
            println!("Problem {} has no rust version.", &id);
            solved_ids.push(problem.question_id);
            continue;
        }
        let code = code.unwrap();

        let file_name = format!(
            "p{:04}_{}",
            problem.question_id,
            problem.title_slug.replace("-", "_")
        );
        let file_path = Path::new("./src/problem").join(format!("{}.rs", file_name));
        if is_solving {
            // check problem/ existence
            if !file_path.exists() {
                panic!("problem does not exist");
            }
            // check solution/ no existence
            let solution_name = format!(
                "s{:04}_{}",
                problem.question_id,
                problem.title_slug.replace("-", "_")
            );
            let solution_path = Path::new("./src/solution").join(format!("{}.rs", solution_name));
            if solution_path.exists() {
                panic!("solution exists");
            }
            // rename/move file
            fs::rename(file_path, solution_path).unwrap();
            // remove from problem/mod.rs
            let mod_file = "./src/problem/mod.rs";
            let target_line = format!("mod {};", file_name);
            let lines: Vec<String> = io::BufReader::new(File::open(mod_file).unwrap())
                .lines()
                .map(|x| x.unwrap())
                .filter(|x| *x != target_line)
                .collect();
            fs::write(mod_file, lines.join("\n"));
            // insert into solution/mod.rs
            let mut lib_file = fs::OpenOptions::new()
                .append(true)
                .open("./src/solution/mod.rs")
                .unwrap();
            writeln!(lib_file, "mod {};", solution_name);
            break;
        }
        if file_path.exists() {
            panic!("problem already initialized");
        }

        let template = fs::read_to_string("./template.rs").unwrap();
        let source = template
            .replace("__PROBLEM_TITLE__", &problem.title)
            .replace("__PROBLEM_DESC__", &build_desc(&problem.content))
            .replace("__PROBLEM_DEFAULT_CODE__", &code.default_code)
            .replace("__PROBLEM_ID__", &format!("{}", problem.question_id))
            .replace("__EXTRA_USE__", &parse_extra_use(&code.default_code));

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)
            .unwrap();

        file.write_all(source.as_bytes()).unwrap();
        drop(file);

        let mut lib_file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open("./src/problem/mod.rs")
            .unwrap();
        writeln!(lib_file, "mod {};", file_name);
        break;
    }
}

fn generate_random_id(except_ids: &[u32]) -> u32 {
    use rand::Rng;
    use std::fs;
    let mut rng = rand::thread_rng();
    loop {
        let res: u32 = rng.gen_range(1, 1106);
        if !except_ids.contains(&res) {
            return res;
        }
        println!(
            "Generate a random num ({}), but it is invalid (the problem may have been solved \
             or may have no rust version). Regenerate..",
            res
        );
    }
}

fn get_solved_ids() -> Vec<u32> {
    let paths = fs::read_dir("./src/problem").unwrap();
    let mut solved_ids = Vec::new();

    for path in paths {
        let path = path.unwrap().path();
        let s = path.to_str().unwrap();
        if !s.starts_with('n') {
            continue;
        }
        let id = &s[7..11];
        let id = id.parse::<u32>().unwrap();
        solved_ids.push(id);
    }
    solved_ids
}

fn parse_extra_use(code: &str) -> String {
    let mut extra_use_line = String::new();
    // a linked-list problem
    if code.contains("pub struct ListNode") {
        extra_use_line.push_str("\nuse crate::util::linked_list::{ListNode, to_list};")
    }
    if code.contains("pub struct TreeNode") {
        extra_use_line.push_str("\nuse crate::util::tree::{TreeNode, to_tree};")
    }
    if code.contains("pub struct Point") {
        extra_use_line.push_str("\nuse crate::util::point::Point;")
    }
    extra_use_line
}

fn build_desc(content: &str) -> String {
    // TODO: fix this shit
    content
        .replace("<strong>", "")
        .replace("</strong>", "")
        .replace("<em>", "")
        .replace("</em>", "")
        .replace("</p>", "")
        .replace("<p>", "")
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("<pre>", "")
        .replace("</pre>", "")
        .replace("<ul>", "")
        .replace("</ul>", "")
        .replace("<li>", "")
        .replace("</li>", "")
        .replace("<code>", "")
        .replace("</code>", "")
        .replace("<i>", "")
        .replace("</i>", "")
        .replace("<sub>", "")
        .replace("</sub>", "")
        .replace("</sup>", "")
        .replace("<sup>", "^")
        .replace("&nbsp;", " ")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&quot;", "\"")
        .replace("&minus;", "-")
        .replace("&#39;", "'")
        .replace("\n\n", "\n")
        .replace("\n", "\n * ")
}
