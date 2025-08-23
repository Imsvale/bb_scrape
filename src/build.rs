// build.rs
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/round_green_pixelated.ico");    // multi-size .ico
        res.compile().unwrap();
    }
}
