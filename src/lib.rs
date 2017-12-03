#![feature(int_uint)] // for log
#[macro_use] extern crate log;
#[macro_use] extern crate mydht;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;
extern crate serde;
extern crate time;
extern crate mydht_openssl;
extern crate mydht_tcp_loop;
extern crate mydht_bincode;
extern crate mydht_slab;
extern crate mydht_inefficientmap;
extern crate striple;
// currently use for content encoding, something like protobuf could be better (self describing)
extern crate bincode;

// TODO unpub when stable
pub mod vote;
pub mod maindht;
pub mod service;

