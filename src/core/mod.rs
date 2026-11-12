//! The Vulture engine — pure system logic, no UI types. Everything here is
//! runnable headlessly (which is how `--auto-clean` works).

pub mod auto_clean;
pub mod browsers;
pub mod disk;
pub mod duplicates;
pub mod memory;
pub mod scanner;
pub mod schedule;
pub mod startup;
pub mod uninstall;
