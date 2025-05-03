const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn version() -> Box<str> {
    let version = VERSION;
    version.into()
}