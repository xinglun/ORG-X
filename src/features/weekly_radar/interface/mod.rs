//! Interface boundary for Weekly Radar renderers and future entrypoints.

pub mod markdown_renderer;
pub mod semantic_message_splitter;
pub mod telegram_renderer;

#[cfg(test)]
mod mod_test;
