fn main() {
    println!("cargo:rerun-if-changed=data/prompts.yaml");
    println!("cargo:rerun-if-changed=data/builtin_ignores.yaml");
    println!("cargo:rerun-if-changed=data/ai_readme.yaml");
    println!("cargo:rerun-if-changed=data/rules/");
}
