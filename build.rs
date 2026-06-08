fn main() {
    // Only compile resources when target is Windows
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target == "windows" {
        embed_resource::compile("resources.rc", embed_resource::NONE);
    }
}
