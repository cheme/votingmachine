#![feature(int_uint)] // for log
#[macro_use] extern crate log;
#[macro_use] extern crate mydht;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate mydht_openssl;
extern crate mydht_tcp_loop;
extern crate mydht_bincode;
extern crate mydht_slab;
extern crate mydht_inefficientmap;

pub mod vote;
pub mod maindht;


