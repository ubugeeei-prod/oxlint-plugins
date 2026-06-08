use divan::black_box;
use oxlint_plugins_stylistic::scan_source_for_rule;

fn main() {
    divan::main();
}

#[divan::bench]
fn scan_file_without_matches() {
    let source = black_box("const value = input.map((item) => item.id).join(',');\n".repeat(512));
    let matches = scan_source_for_rule(&source, []);
    black_box(matches);
}

#[divan::bench]
fn scan_file_with_matches() {
    let source = black_box("const event = data.error;\n".repeat(512));
    let matches = scan_source_for_rule(&source, []);
    black_box(matches);
}
