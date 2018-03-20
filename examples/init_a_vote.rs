//! hard coded vote init to build vote for poc
//!


#[macro_use] extern crate mydht;
#[macro_use] extern crate log;
extern crate votingmachine;
extern crate striple;
//extern crate env_logger;

extern crate mydht_openssl;
extern crate mydht_tcp_loop;
//extern crate mydht_wot;

#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate mydht_bincode;
 
use std::ffi::OsString;
use striple::anystriple::{
  Rsa2048Sha512,
};
use mydht::storeprop::{
  KVStoreCommand,
};
use mydht::{
  MyDHTConf,
  ApiCommand,
  QueryPriority,
  PeerPriority,
};
use mydht::dhtif::{
  Result as MResult,
};
use mydht::service::{
  SpawnSend,
};
use mydht::dhtimpl::{
  SimpleRules,
};
use mydht::peer::{
  Peer,
  PeerMgmtMeths,
};
use mydht::api::{
  DHTIn,
};
use std::time::Duration;
use mydht_bincode::Bincode;
use std::borrow::Borrow;
use serde::de::DeserializeOwned;
use serde_json as json;
use serde_json::error::Error as JSonError;
use std::io::Result as IoResult;
use serde::{Serializer,Serialize,Deserializer,Deserialize};
use std::os;
use std::env;
use std::fs::{
  self,
  File,
};
use std::io::Read;
use std::io::Write;
use std::io::BufRead;
use std::io::stdin;
use std::io::Stdin;
use std::io::StdinLock;
use std::net::SocketAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use mydht::keyval::KeyVal;
use mydht::transportif::SerSocketAddr;
use std::path::{Path,PathBuf};
use mydht_openssl::rsa_openssl::{
  RSAPeer as RSAPeerC,
  RSAPeerMgmt as RSAPeerMgmtC,
  RSA2048SHA512AES256,
};
use votingmachine::vote;
use votingmachine::vote::striples;
use votingmachine::maindht::{
  MainDHTConf as MainDHTConfC,
  DHTRULES_MAIN,
};
use mydht::utils::{
  Ref,
  ArcRef,
};
use mydht_tcp_loop::{
  Tcp,
};
use vote::{
  VoteDesc,
  MainStoreKV,
  MainStoreKVRef,
};
use vote::striples::{
  StriplePeer,
  STRIPLEREFS,
};
use striple::striple::{
  StripleIf,
  StripleFieldsIf,
  StripleImpl,
  StripleKind,
};

// copied from main (without useless arcref)
#[derive(Debug,Deserialize,Serialize)]
/// Config of the storage
pub struct VoteConf {
  /// your own peer infos.
  pub me : RSAPeer,
  /// Transport to use
  pub tcptimeout : i64,
}

type RSAPeer = StriplePeer<String,SerSocketAddr,RSA2048SHA512AES256,Rsa2048Sha512>;

const voteconfspath : &str = "./test_peers/peers/";
const bootstrapspath : &str = "./test_peers/peers_bootstrap/";

const votesubject : &str = "Are you?";
const vote_replies : [&str;3] = ["yes","no","maybe"];
const vote_dest : &str = "vote.json";
fn main() {
  let peers : Vec<(RSAPeer,OsString)> = fs::read_dir(voteconfspath).unwrap()
    .map(|e|(peer_from_conf(&e.as_ref().unwrap().path()),
    e.unwrap().file_name())).collect();
  let peers_id : Vec<Vec<u8>> = peers.iter().map(|&(ref p,_)|p.get_id().to_vec()).collect();
  for id in peers_id.iter() {
    write_bootstrap(id,&peers)
  }
  let pk = (peers[0].0).inner.get_pri_key();
  let vote_desc = VoteDesc::new(
     &peers[0].0,
     &pk[..],
     votesubject.to_string(),
     vote_replies.iter().map(|r|r.to_string()).collect(),
     peers_id,
    ).unwrap();
 
  let mut tmp_file = File::create(vote_dest).unwrap();
  json::to_writer(&mut tmp_file,&vote_desc).unwrap();

}
fn peer_from_conf(conf_path : &PathBuf) -> RSAPeer {

  let mut f = File::open(conf_path).unwrap();
  let fsconf : VoteConf = json::from_reader(&mut f).unwrap();
  fsconf.me

}

fn write_bootstrap(peer_id : &Vec<u8>, peers : &Vec<(RSAPeer,OsString)>) {
  let fname = peers.iter().find(|pn|pn.0.get_id() == &peer_id[..]).unwrap().1.clone();

  let mut dest = PathBuf::from(bootstrapspath);
  dest.push(fname);
  println!("{:?}",dest);
  let mut tmp_file = File::create(dest).unwrap();
  let tw : Vec<&RSAPeer> = peers.iter().filter(|pn|pn.0.get_id() != &peer_id[..]).map(|pn|&pn.0).collect();
  json::to_writer(&mut tmp_file,&tw).unwrap();

}
