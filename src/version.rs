const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn version() -> &'static str {
    let version = VERSION;
    version.into()
}