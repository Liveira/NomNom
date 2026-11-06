fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 {
        eprintln!("usage: sparse_manifest <template> <publisher> <app version>");
        std::process::exit(2);
    }
    let template = std::fs::read_to_string(&a[1]).expect("template");
    print!(
        "{}",
        nomnom_win::sparse_manifest::render(
            &template,
            &a[2],
            &nomnom_win::sparse_manifest::package_version(&a[3])
        )
    );
}
