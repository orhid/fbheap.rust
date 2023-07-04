#![allow(dead_code)]
#![warn(
    clippy::all,
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::suspicious,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![feature(let_chains)]

pub mod error;
pub mod heap;
pub mod node;
