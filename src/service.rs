//! main service (in the main dht) for internal logic (synch of content ...),
//! with inner standerad kvstore service
use mydht_tunnel::{
  MyDHTTunnelConf,
  MyDHTTunnelConfType,
  GlobalTunnelReply,
  SSWCache,
  SSRCache,
  GlobalTunnelCommand,
};


use mydht::keyval::{
  KeyVal,
};
use anodht::{
  AnoServiceICommand,
  StoreAnoMsg,
  AnoAddress,
};
use std::hash::Hash;
use anodht::{
  AnoDHTConf,
  AnoTunDHTConf2,
};
 
use mydht::api::{
  DHTIn,
};
use maindht::{
  MainDHTConf,
};



use mydht::dhtimpl::{
  SimpleRules,
};
use mydht::mydhtresult::{
  Result,
};
use mydht::peer::{
  Peer,
  PeerMgmtMeths,
};
use mydht::utils::{
  Ref,
  SerSocketAddr,
};
use mydht::service::{
  Service,
  SpawnerYield,
  SpawnSend,
};
use mydht::storeprop::{
  KVStoreProtoMsgWithPeer,
  KVStoreCommand,
  KVStoreReply,
  KVStoreService,
};
use vote::{
  VoteDesc,
  Envelope,
  MainStoreKV,
  MainStoreKVRef,
};

use maindht::{
  MainStoreKVStore,
  MainStoreQueryCache,
};

use mydht::{
  GlobalCommand,
  GlobalReply,
  ApiCommand,
};

pub struct VotingService<P : Peer<Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP,PM : PeerMgmtMeths<P>> 
where <P as KeyVal>::Key : Hash,
      <P as Peer>::Address : Hash,
  {
  pub store_service : KVStoreService<
    P,
    RP,
    MainStoreKV,
    MainStoreKVRef,
    MainStoreKVStore,
    SimpleRules,
    MainStoreQueryCache<P,RP>
  >,
  pub ano_dhtin : 
    DHTIn<MyDHTTunnelConfType<AnoTunDHTConf2<P,PM>>>,
 
}
use std::borrow::Borrow;

impl<P : Peer<Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP : Ref<P> + Clone,PM : PeerMgmtMeths<P>> VotingService<P,RP,PM>
where <P as KeyVal>::Key : Hash,
      <P as Peer>::Address : Hash,
  {
  /// filter for validation
  fn vote_impl(&mut self, kv: &MainStoreKVRef) -> Result<Option<<Self as Service>::CommandOut>> {
    match *kv.borrow() {
      MainStoreKV::VoteDesc(ref votedesc) => {
        // TODO let global service listen peer update and query check this vd
        //
        //
       
  // make our enveloppe (public sign by votedesc striple)
  /* explicitely safe version
  let (envelope, envpk) = {
    let mut envelope = Envelope::new(votedesc)?;
    let envpk = envelope.privatekey();
    envelope.privatekey = Vec::new();
    (envelope, envpk)
  };*/

  // keep localy envelope with pk (pk not serialized through serde so vec null is send).
  let envelope = Envelope::new(votedesc)?;

  //assert!(true == false);
//  assert!(envelope.check(votedesc).unwrap()==true); useless check except for debuging purpose)
  println!("initialized envolope");

  // store enveloppe with pk : not in POC (use this object for next steps no persistence)
 
  // share enveloppe anonymously (store + query all)
  //TODO add apiid to kvstore or create push for kvstore (similar to store locally
  //TODO run with reply ??
  let c_store_env = GlobalTunnelCommand::Inner(AnoServiceICommand(StoreAnoMsg::STORE_ENVELOPE(envelope.clone())));
  let command = ApiCommand::call_service(c_store_env);
  self.ano_dhtin.send(command)?;

  // query all enveloppe of anonymous dht

  // create participation (sign by our peer striple)

  // share participation (store + query all)
 
  // todo (not in poc) public synchro of everyone validating participation (in POC panic peer if
  // invalid)
 
  // make vote (sign by enveloppe, about votedesc)
  
  // share votes (store + query all) in anonymous dht

  // make result (valid my vote and nb vote) : sign by user
  
  // share results (store + query all)

  // print global vote result


      },
      MainStoreKV::Envelope(ref envelope) => {
        // TODO manage envelope list and probably store it
        println!("--------------------> Env store reach");
      },
    }
    Ok(None)
  }
}

impl<P : Peer<Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP : Ref<P> + Clone,PM : PeerMgmtMeths<P>> Service for VotingService<P,RP,PM>
where <P as KeyVal>::Key : Hash,
      <P as Peer>::Address : Hash,
  {
 
  //KVStoreService<P,RP,MainStoreKV,MainStoreKVRef,MainStoreKVStore,SimpleRules,MainStoreQueryCache<P,RP>> {
  type CommandIn = GlobalCommand<RP,KVStoreCommand<P,RP,MainStoreKV,MainStoreKVRef>>;
  type CommandOut = GlobalReply<P,RP,KVStoreCommand<P,RP,MainStoreKV,MainStoreKVRef>,KVStoreReply<MainStoreKVRef>>;

  fn call<Y : SpawnerYield>(&mut self, req: Self::CommandIn, async_yield : &mut Y) -> Result<Self::CommandOut> {

    // filters :
    match req.get_inner_command() {
      &KVStoreCommand::Store(_,ref vals) => for v in vals.iter() {
        if let Some(r) = self.vote_impl(v)? {
          return Ok(r);
        }
      },
      &KVStoreCommand::StoreLocally(ref v,..) => if let Some(r) = self.vote_impl(v)? {
        return Ok(r);
      },
      _ => (),
    }

    // query votedesc : if no participation set for votedesc : reply only if user in list of voters -> create a command for it that use owith in command TODO evo MyDHT to add owith to local or global commad
    // for a start simply reply?? (key being the secret)
    let command_out = self.store_service.call(req,async_yield)?;
    match command_out {
      GlobalReply::Api(KVStoreReply::FoundApi(Some(ref val),_)) => (),
      GlobalReply::Api(KVStoreReply::FoundApiMult(ref vals,_)) => (),
      _ => (),
    }
    Ok(command_out)
  }
}
