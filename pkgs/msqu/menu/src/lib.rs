//! msqu's Rust side: the menu (msqu-ui, src/main.rs), the bar blocks
//! (msqu-bar, src/bin/msqu-bar.rs) and the tools (msqu-tool,
//! src/bin/msqu-tool.rs; tool), sharing the tools' state files and times
//! (model) and, for the menu, the terminal (term).

pub mod model;
pub mod term;
pub mod tool;
