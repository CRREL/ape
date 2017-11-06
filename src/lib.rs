extern crate bincode;
extern crate chrono;
#[macro_use]
extern crate quick_error;
#[cfg(target_os = "linux")]
extern crate scanlib;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod incl;
pub mod utils;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Bincode(err: bincode::Error) {
            from()
            cause(err)
        }
        ChronoParse(err: chrono::ParseError) {
            from()
            cause(err)
        }
        Io(err: std::io::Error) {
            from()
            cause(err)
        }
        MissingFileStem {}
        Scanlib(msg: String) {}
    }
}

pub type Result<T> = std::result::Result<T, Error>;
