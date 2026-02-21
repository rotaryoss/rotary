use std::path::Path;

use rotaryoss_core::{RotaryConfig, RotaryError};

pub fn run(dir: &Path) -> Result<(), RotaryError> {
    let target = dir.join("rotary.toml");
    if target.exists() {
        return Err(RotaryError::Config(format!(
            "rotary.toml already exists at {}",
            target.display()
        )));
    }

    std::fs::write(&target, RotaryConfig::starter())?;
    println!("Created {}", target.display());
    println!("Edit rotary.toml to add your secret sources, then run `rotary scan`.");
    Ok(())
}
