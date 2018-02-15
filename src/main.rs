//! Voting Protocol implementation

#[macro_use] extern crate mydht;
#[macro_use] extern crate log;
extern crate mydht_tunnel;
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

 
use mydht::rules::DHTRules; 
use striple::anystriple::{
  Rsa2048Sha512,
};
use mydht::storeprop::{
  KVStoreCommand,
  KVStoreReply,
};
use mydht::{
  MyDHTConf,
  ApiCommand,
  ApiResult,
  QueryPriority,
  PeerPriority,
  QueryConf,
  QueryMode,
  MCReply,
};
use mydht::service::{
  MpscChannel,
  MioChannel,
  SpawnChannel,
};
 
use mydht_tunnel::{
  MyDHTTunnelConf,
  MyDHTTunnelConfType,
  GlobalTunnelReply,
  SSWCache,
  SSRCache,
  GlobalTunnelCommand,
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
use mydht::utils::{
  SerSocketAddr,
  OneResult,
  new_oneresult,
  replace_wait_one_result,
  clone_wait_one_result,
};
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
use votingmachine::anodht::{
  AnoDHTConf as AnoDHTConfC,
  new_ano_conf,
  AnoAddress,
  AnoPeer,
  StoreAnoMsg,
  AnoServiceICommand,
  AnoTunDHTConf,
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
  Envelope,
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

//type RSAPeerInner = RSAPeerC<String,SerSocketAddr,RSA2048SHA512AES256>;
type RSAPeer = StriplePeer<String,SerSocketAddr,RSA2048SHA512AES256,Rsa2048Sha512>;
type RSAPeerMgmt = RSAPeerMgmtC<RSA2048SHA512AES256>;
#[derive(Clone)]
// TODO make generic to striplepeerif ??
pub struct StriplePeerMgmt(RSAPeerMgmt);

type MainDHTConf = MainDHTConfC<RSAPeer,StriplePeerMgmt>;
type AnoDHTConf = AnoDHTConfC<RSAPeer,StriplePeerMgmt,MainDHTConf>;

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
/*
impl<T : Serialize> Serialize for ArcRef<T> {
  fn serialize<S : Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
    let a : &T = self.borrow();
    a.serialize(serializer)
  }
}*/


fn new_vote_conf (stdin : &mut StdinLock, path : &Path) -> IoResult<VoteConf> {
  let mut newname = String::new();
  println!("creating a new user, what is your id/name?");
  stdin.read_line(&mut newname).unwrap();
  newname.pop();
  println!("address initialize to default ipv4 local host \"127.0.0.1:6663\" and ano \"127.0.0.1:7663\"");
  let mut me2 = RSAPeerC::new (SerSocketAddr(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127,0,0,1),6663))),newname).unwrap();
  let secadd = SerSocketAddr(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127,0,0,1),7663)));
  //let me2 = RSAPeer::new (newname, None, IpAddr::new_v4(127,0,0,1), 6663);
  println!("tcp timeout default to 4 seconds");
  {
    let mut tmp_file = File::create(path).unwrap();
  //      tmp_file.write_all(json::encode(&fsconf2).unwrap().into_bytes().as_slice());
    me2.set_write_private(true);
    let m = ArcRef::new(StriplePeer::new(me2,secadd).unwrap());
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
  
  // check fsconf (TODO find a way to autocheck from serde)
  // Done in init serde function (cf fork) , just commented here in case fork is refused.
/*  {
    let m : &RSAPeer = fsconf.me.borrow();
    assert!(m.borrow().check(&STRIPLEREFS.pub_peer).unwrap());
  }*/

  // getting bootstrap peers
  let boot_peers = match File::open(&boot_path) {
    Ok(mut f) => {
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
 
  let ano_tcp_transport = {
    let m = AnoPeer(fsconf.me.clone());
    Tcp::new(
      m.get_address(),
      Some(Duration::from_secs(5)), // timeout
      true,//mult
    ).unwrap()
  };

  // warning MpscChannel is same as init_main_loop_channel_in result
  let (ano_s,ano_r) = MioChannel(MpscChannel).new().unwrap();

  let mut anosendcommand = DHTIn {
    main_loop : ano_s.clone()
  };

  let conf = MainDHTConfC {
    me : fsconf.me.clone(),
    others : boot_peers.clone(),
    msg_enc : Bincode,
    transport : Some(main_tcp_transport),

    peer_mgmt : StriplePeerMgmt(RSAPeerMgmt::new()),
    rules : SimpleRules::new(DHTRULES_MAIN),
    ano_dhtin : Some(DHTIn {
      main_loop : ano_s.clone()
    }),
  };

  let (mut sendcommand,_recv) = conf.start_loop().unwrap();

  let conf2 = AnoTunDHTConf {
    conf : MainDHTConfC {
      me : fsconf.me.clone(),
      others : boot_peers,
      msg_enc : Bincode,
      transport : Some(ano_tcp_transport),

      peer_mgmt : StriplePeerMgmt(RSAPeerMgmt::new()),
      rules : SimpleRules::new(DHTRULES_MAIN),
      ano_dhtin : None,
    },
    main_api : Some(DHTIn {
      main_loop : sendcommand.main_loop.clone(),
    }),
  };


  let anoconf = new_ano_conf(conf2).unwrap();

  let _recv = anoconf.start_loop_with_channel(ano_s, ano_r).unwrap();

  // Interactive mode TODO expand command syntax to not run interactive mode (only if -I), with
  // more common scenarii
  // prompt : vote from file to add a json serialized vote, vote from key to first get the vote in
  // dht.
  loop {
    println!("vote_file, vote_key, quit?");
    let mut line = String::new();
    stdin.read_line(&mut line).unwrap();
    match line.as_str() {
      "quit\n" => {
          break
      },
      "vote_file\n" => {
        println!("file path?");
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

                println!("your vote ?");
                let mut vote_val = String::new();
                stdin.read_line(&mut vote_val).unwrap();
                vote_val.pop();
                do_vote(&mut sendcommand, &mut anosendcommand, vote,vote_val, &fsconf).unwrap();
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

// this logic can be put in global service better for reuse, yet it requires to call api from
// global service to peer and a lot of buff for asynch of itself (we should not block and pending
// buffer needs to be managed), so for now we go simple with blocking api call from outside of
// mydht. 
fn do_vote(main_in : &mut DHTIn<MainDHTConf>, ano_in : &mut DHTIn<AnoDHTConf>, vote : VoteDesc, vote_val : String, conf : &VoteConf) -> MResult<()> {
  // check vote
  let self_check_ok = {
    let s : &RSAPeer = conf.me.borrow();
    if *s.get_id() == *vote.get_from() {
      println!("self vote check...");
      assert!(vote.check(s).unwrap() == true);
      println!("self vote check pass");
      true
    } else {
      false
    }
  };
  if !self_check_ok {
    println!("find vote emitter...");
    let pfrom = find_peer(main_in, &conf.me,vote.get_from().as_ref())?;
    let s : &RSAPeer = pfrom.borrow();
    println!("vote check...");
    assert!(vote.check(s).unwrap() == true);
    println!("vote check pass");
  }

  println!("check vote ok");
  let voteref = ArcRef::new(MainStoreKV::VoteDesc(vote));
  // store vote (to make it accessible from other peers) : also trigger the vote process in global
  // service (TODO switch global service to kvstore overload to simple service)
  let c_store_vote = ApiCommand::call_service(KVStoreCommand::StoreLocally(voteref.clone(),1,None));
  main_in.send(c_store_vote)?;
  Ok(())
}

// TODO need to refer me is odd and unjustified : require redesig to avoid call to 'query_message'
// function which should be internal to mydht
fn find_peer(main_in : &mut DHTIn<MainDHTConf>,me : &ArcRef<RSAPeer>, peer_id : &[u8]) -> MResult<ArcRef<RSAPeer>> {
  let queryconf = QueryConf {
    mode : QueryMode::Asynch, 
    hop_hist : Some((3,true))
  }; // note that we only unloop to 3 hop 

  let nb_res = 1;
  let rules = SimpleRules::new(DHTRULES_MAIN);
  let o_res = new_oneresult((Vec::with_capacity(nb_res),nb_res,nb_res));
  let prio = 1;
  let nb_hop = rules.nbhop(prio);
  let nb_for = rules.nbquery(prio);
  let qm = queryconf.query_message(me.borrow(), nb_res, nb_hop, nb_for, prio);
  let peer_q = ApiCommand::call_peer_reply(KVStoreCommand::Find(qm,peer_id.to_vec(),None),o_res.clone());
  main_in.send(peer_q)?;
  let mut o_res = clone_wait_one_result(&o_res,None).unwrap();
  // fail on peer not found (TODO should retry)
  assert!(o_res.0.len() == 1, "Peer not found {:?}", peer_id);
  let v = o_res.0.pop().unwrap();
  let result : Option<ArcRef<RSAPeer>> = if let ApiResult::ServiceReply(MCReply::PeerStore(KVStoreReply::FoundApi(ores,_))) = v {
    ores
  } else if let ApiResult::ServiceReply(MCReply::PeerStore(KVStoreReply::FoundApiMult(mut vres,_))) = v {
    vres.pop()
  } else {
    None
  };
  Ok(result.unwrap())
}
