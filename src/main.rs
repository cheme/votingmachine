//! Voting Protocol implementation

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
  RSAPeerMgmt as RSAPeerMgmtC,
  RSA2048SHA512AES256,
};
use votingmachine::vote;
use votingmachine::striples;
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
use striples::{
  StriplePeerIf,
  StriplePeer,
};
use striple::striple::{
  StripleKind,
};
type RSAPeerInner = RSAPeerC<String,SerSocketAddr,RSA2048SHA512AES256>;
type RSAPeer = StriplePeer<RSAPeerInner>;
type RSAPeerMgmt = RSAPeerMgmtC<RSA2048SHA512AES256>;
#[derive(Clone)]
// TODO make generic to striplepeerif ??
pub struct StriplePeerMgmt(RSAPeerMgmt);

type MainDHTConf = MainDHTConfC<RSAPeer,StriplePeerMgmt>;

impl PeerMgmtMeths<RSAPeer> for StriplePeerMgmt {
  fn challenge (&self, p: &RSAPeer) -> Vec<u8> {
    self.0.challenge(&p.inner)
  }
  fn signmsg (&self, p : &RSAPeer, m : &[u8]) -> Vec<u8> {
    self.0.signmsg(&p.inner,m)
  }
  fn checkmsg (&self, p : &RSAPeer, a : &[u8], b : &[u8]) -> bool {
    self.0.checkmsg(&p.inner,a,b)
  }
  fn accept (&self, p : &RSAPeer) -> Option<PeerPriority> {
    // TODO run a striple check!!!
    self.0.accept(&p.inner)
  }
}


#[derive(Debug,Deserialize,Serialize)]
/// Config of the storage
pub struct VoteConf {
  /// your own peer infos.
  pub me : ArcRef<RSAPeer>,
  /// Transport to use
  pub tcptimeout : i64,
}

fn new_vote_conf (stdin : &mut StdinLock, path : &Path) -> IoResult<VoteConf> {
  let mut newname = String::new();
  println!("creating a new user, what is your id/name?");
  stdin.read_line(&mut newname).unwrap();
  newname.pop();
  println!("address initialize to default ipv4 local host \"127.0.0.1:6663\"");
  let mut me2 = RSAPeerC::new (SerSocketAddr(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127,0,0,1),6663))),newname).unwrap();
  //let me2 = RSAPeer::new (newname, None, IpAddr::new_v4(127,0,0,1), 6663);
  println!("tcp timeout default to 4 seconds");
  {
    let mut tmp_file = File::create(path).unwrap();
  //      tmp_file.write_all(json::encode(&fsconf2).unwrap().into_bytes().as_slice());
    me2.set_write_private(true);
    let m = ArcRef::new(StriplePeer::new(me2).unwrap());
    let fsconf2 = VoteConf {
      me : m,
      tcptimeout : 4,
    };

    json::to_writer(&mut tmp_file,&fsconf2).unwrap();
    println!("New fsconf written to \"{:?}\"",path);
  }
//      fsconf2.me.set_write_private(false);

  let mut tmp_file = File::open(&path)?;
  let fsconf : VoteConf = json::from_reader(&mut tmp_file)?;
  {
    let m : &RSAPeer = fsconf.me.borrow();
    assert!(m.inner.is_write_private() == false);
  }
  Ok(fsconf)
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

  let tstdin = stdin();
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
      new_vote_conf(&mut stdin,&Path::new("./voteconf.json")).unwrap()
    },
  };

  let tcptimeout = &fsconf.tcptimeout;
  info!("my conf is : {:?}" , fsconf);

  // getting bootstrap peers
  let boot_peers = match File::open(&boot_path) {
    Ok(mut f) => {
      let mut jcont = String::new();
      let peers : Vec<ArcRef<RSAPeer>> = json::from_reader(&mut f).unwrap_or_else(|e|panic!("Invalid config {:?}\n quiting",e));
      if peers.len() > 0 {
        Some(peers)
      } else {
        None
      }
    },
    Err(_) => {
      println!("No bootstrap peer found.");
      None
    },
  };


  // Bootstrap dht with rights types TODO

  let main_tcp_transport = {
    let m : &RSAPeer = fsconf.me.borrow();
    Tcp::new(
      m.inner.get_address(),
      Some(Duration::from_secs(5)), // timeout
      true,//mult
    ).unwrap()
  };
 

  let conf = MainDHTConfC {
    me : fsconf.me.clone(),
    others : boot_peers,
    msg_enc : Bincode,
    transport : Some(main_tcp_transport),

    peer_mgmt : StriplePeerMgmt(RSAPeerMgmt::new()),
    rules : SimpleRules::new(DHTRULES_MAIN),
  };

  let (mut sendcommand,_recv) = conf.start_loop().unwrap();


  // Interactive mode TODO expand command syntax to not run interactive mode (only if -I), with
  // more common scenarii
  // prompt : vote from file to add a json serialized vote, vote from key to first get the vote in
  // dht.
  loop {
    println!("vote_file, vote_key, quit?");
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
        let p = Path::new(path.as_str());
        match File::open(&p) {
          Ok(mut f) => {
            let r_vote : Result<VoteDesc,_> = json::from_reader(&mut f);
            match r_vote {
              Ok(vote) => {

                println!("subject : {}", vote.subject);
                println!("replies : {:?}", vote.replies);
                let vote = ArcRef::new(MainStoreKV::VoteDesc(vote));

                println!("your vote ?");
                let mut vote_val = String::new();
                stdin.read_line(&mut vote_val).unwrap();
                vote_val.pop();
                do_vote(&mut sendcommand,vote,vote_val).unwrap();
              },
              Err(e) => {
                println!("Invalid vote config {:?}",e);
                continue;
              }
            }
          },
          Err(_) => {
            println!("No such file.");
            continue;
          },
        };
      },
      "vote_key\n" => {
        let mut key = String::new();
        key.pop();
        stdin.read_line(&mut key).unwrap();
        // TODO base58 or 64 decode and get from other peers, with big depth
        unimplemented!();
        
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

fn do_vote(main_in : &mut DHTIn<MainDHTConf>, vote : MainStoreKVRef, vote_val : String) -> MResult<()> {

  // store vote (to make it accessible from other peers)
  let c_store_vote = ApiCommand::call_service(KVStoreCommand::StoreLocally(vote.clone(),1,None));
  main_in.send(c_store_vote)?;

  // make our enveloppe
  

  // store enveloppe


  // make enveloppe vote peer relation

  // store relation : TODO include as private field of vote (unserialized) and store vote later
  Ok(())
}
