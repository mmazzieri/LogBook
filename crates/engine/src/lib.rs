//! DiaryCraft pure logic: file names of the entries, front matter, Markdown highlighting, search,
//! calendar, the speed list.
//!
//! No I/O and no clock: everything takes its inputs as parameters, so it builds for wasm32 too.
#![forbid(unsafe_code)]

pub mod calendar;
pub mod layout;
pub mod markdown;
pub mod search;
pub mod speed_list;
pub mod text;

#[cfg(test)]
mod tests;
