fn main() {
    println!("cargo:rerun-if-changed=../installer/assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../installer/assets/icon.ico");
        res.compile().unwrap();
    }
}
