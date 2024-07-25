use std::error::Error;
use vergen_gix::{CargoBuilder, Emitter, GixBuilder as GitBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    Emitter::default()
        .add_instructions(&CargoBuilder::all_cargo()?)?
        .add_instructions(&GitBuilder::all_git()?)?
        .emit()?;
    Ok(())
}
