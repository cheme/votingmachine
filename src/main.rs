//! Voting Protocol implementation

#[macro_use] extern crate mydht;
#[macro_use] extern crate log;
extern crate votingmachine;
//extern crate env_logger;

extern crate mydht_openssl;
//extern crate mydht_wot;

#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use serde::de::DeserializeOwned;
use serde_json as json;
use serde_json::error::Error as JSonError;

use serde::{Serializer,Serialize,Deserializer,Deserialize};
use std::os;
use std::env;
use std::fs::File;
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
use mydht::utils::SerSocketAddr;
use std::path::{Path,PathBuf};
use mydht_openssl::rsa_openssl::{
  RSAPeer as RSAPeerC,
  RSA2048SHA512AES256,
};
use votingmachine::vote;

type RSAPeer = RSAPeerC<String,SerSocketAddr,RSA2048SHA512AES256>;

#[derive(Debug,Deserialize,Serialize)]
/// Config of the storage
pub struct VoteConf {
  /// your own peer infos.
  pub me : RSAPeer,
  /// Transport to use
  pub tcptimeout : i64,
}

fn new_vote_conf (stdin : &mut StdinLock) -> VoteConf {
  let mut newname = String::new();
  println!("creating a new user, what is your id/name?");
  stdin.read_line(&mut newname).unwrap();
  newname.pop();
  println!("address initialize to default ipv4 local host \"127.0.0.1:6663\"");
  let me2 = RSAPeerC::new (SerSocketAddr(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127,0,0,1),6663))),newname).unwrap();
  //let me2 = RSAPeer::new (newname, None, IpAddr::new_v4(127,0,0,1), 6663);
  println!("tcp timeout default to 4 seconds");
  VoteConf {
    me : me2,
    tcptimeout : 4,
  }
}

fn main() {
  //env_logger::init().unwrap();
  let show_help = || println!("-C <file> to select config file, -B <file> to choose node bootstrap file");
  let mut conf_path = PathBuf::from("voteconf.json");
  let mut boot_path = PathBuf::from("votebootstrap.json");
  enum ParamState {Normal, Conf, Boot}
  let mut state = ParamState::Normal;
  for arg in env::args() {
    match arg.as_str() {
      "-h" => show_help(),
      "--help" => show_help(),
      "-C" => state = ParamState::Conf,
      "-B" => state = ParamState::Boot,
        a  => match state {
           ParamState::Normal => debug!("{:?}", arg),
           ParamState::Conf => { conf_path = PathBuf::from(a); state = ParamState::Normal },
           ParamState::Boot => { boot_path = PathBuf::from(a); state = ParamState::Normal },
         }
    }
  }

  let mut tstdin = stdin();
  let mut stdin = tstdin.lock();
 
  info!("using conf file : {:?}" , conf_path);
  info!("using boot file : {:?}" , boot_path);

  // TODO on no file ask for new config creation (with new key).
  let fsconf : VoteConf = match File::open(&conf_path) {
    Ok(mut f) => {
/*      let mut jcont = String::new();
      f.read_to_string(&mut jcont).unwrap();
      */
      json::from_reader(&mut f).unwrap_or_else(|e|panic!("Invalid config {:?}\n quiting",e))
    },
    Err(_) => {
      println!("No conf found");
      let fsconf2 = new_vote_conf(&mut stdin);
      let mut tmp_file = File::create(&Path::new("./voteconf.json")).unwrap();
//      tmp_file.write_all(json::encode(&fsconf2).unwrap().into_bytes().as_slice());
      json::to_writer(&mut tmp_file,&fsconf2).unwrap();
      println!("New fsconf written to \".\\voteconf.json\"");
      fsconf2
    },
  };

  let tcptimeout = &fsconf.tcptimeout;
  info!("my conf is : {:?}" , fsconf);
  let mynode = fsconf.me;

  // getting bootstrap peers
  let boot_peers : Vec<RSAPeer> = match File::open(&boot_path) {
    Ok(mut f) => {
      let mut jcont = String::new();
      /*f.read_to_string(&mut jcont).unwrap();
      json::decode(jcont.as_str()).unwrap_or_else(|e|panic!("Invalid config {:?}\n quiting",e))*/
      json::from_reader(&mut f).unwrap_or_else(|e|panic!("Invalid config {:?}\n quiting",e))
    },
    Err(_) => {
      println!("No bootstrap peer found.");
      Vec::new()
    },
  };




  // Bootstrap dht with rights types TODO
  
  // Interactive mode TODO expand command syntax to not run interactive mode (only if -I), with
  // more common scenarii
  // prompt : vote from file to add a json serialized vote, vote from key to first get the vote in
  // dht.
  println!("vote_file, vote_key, quit?");
  loop {
    let mut line = String::new();
    stdin.read_line(&mut line);
    match line.as_str() {
      "quit\n" => {
          break
      },
      "vote_file\n" => {
         let mut path = String::new();
         stdin.read_line(&mut path).unwrap();
         path.pop();
         let p = Path::new (path.as_str());
         let f = File::open(&p); 
//        let kv = <FileKV as FileKeyVal>::from_file(&mut f.unwrap());
  //        match kv {
   //        None => println!("Invald file content"),
          //  Some(kv) => {
           // println!("Loading");
  //        },
 // };
      },
      c => { 
        println!("unrecognize command : {:?}", c);
      },
    };
  };

  println!("exiting...");
  // clean shut
//  serv.shutdown();
  // wait
//  serv.block();
 
  println!("exiting.ok");
}
