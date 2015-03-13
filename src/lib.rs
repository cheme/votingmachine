#![feature(int_uint)] // for log
#![feature(core)] // for core
#[macro_use] extern crate log;
extern crate "rustc-serialize" as rustc_serialize;
extern crate time;
extern crate mydht;
use std::old_io::File;

pub mod vote;
