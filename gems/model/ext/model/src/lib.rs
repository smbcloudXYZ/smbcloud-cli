use magnus::{function, prelude::*, Error, Ruby};
use smbcloud_model::error_codes::ErrorCode;
use strum::IntoEnumIterator;

fn t(e: i32, l: Option<String>) -> String {
    ErrorCode::from_i32(e).message(l).to_string()
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SmbCloud")?;
    let error_code_mod = module.define_module("ErrorCode")?;
    error_code_mod.define_singleton_method("t", function!(t, 2))?;
    // Define all error codes as constants
    for e in ErrorCode::iter() {
        error_code_mod.const_set(e.rb_constant_name(), e as i32)?;
    }
    Ok(())
}
