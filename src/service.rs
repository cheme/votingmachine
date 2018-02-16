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
use striple::striple::{
  StripleIf,
};
use vote::striples::{
  StripleMydhtErr,
};
use std::collections::BTreeMap;

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
  Error,
  ErrorKind,
};
use mydht::peer::{
  Peer,
  PeerMgmtMeths,
};
use mydht::utils::{
  ArcRef,
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
  FWConf,
};

pub struct VotingService<P : Peer<Key = Vec<u8>, Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP,PM : PeerMgmtMeths<P>> 
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
  pub ano_dhtin : DHTIn<MyDHTTunnelConfType<AnoTunDHTConf2<P,PM>>>,
  pub votes : BTreeMap<Vec<u8>,VoteContext>,
}
// TODO change mydht error to contain static &[u8] and list of objects to format!!!
//const no_vote_context : Error = Error("no vote context".to_string(),ErrorKind::ExpectedError,None);
#[inline]
fn no_vote_context() -> Error {
  Error("no vote context".to_string(),ErrorKind::ExpectedError,None)
}

pub struct VoteContext {
  pub vote_desc : VoteDesc,
  pub my_envelope : Envelope,
  pub envelopes : Vec<Envelope>,
}

use std::borrow::Borrow;

impl<P : Peer<Key = Vec<u8>, Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP : Ref<P> + Clone,PM : PeerMgmtMeths<P>> VotingService<P,RP,PM>
  {


  /// filter for validation
  fn vote_impl(&mut self, kv: &MainStoreKVRef, is_local : bool) -> Result<Option<<Self as Service>::CommandOut>> {
    if is_local {
    match *kv.borrow() {
      MainStoreKV::VoteDesc(ref votedesc) => {
       
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
         let context = VoteContext {
           vote_desc : votedesc.clone(),
           my_envelope : envelope.clone(),
           envelopes : Vec::with_capacity(votedesc.nb_invit()),
         };
         self.votes.insert(votedesc.get_key(), context);
         println!("initialized envolope");

         //  assert!(envelope.check(votedesc).unwrap()==true); useless check except for debuging purpose)

         // store enveloppe with pk : not in POC (use this object for next steps no persistence)
 
        // share enveloppe anonymously (store + query all)
        //TODO run with reply ?? or do another store after a triggered timer (required to create
        //mainloop timer (cf already needed to maintain pool)
        let c_store_env = GlobalTunnelCommand::Inner(AnoServiceICommand(StoreAnoMsg::STORE_ENVELOPE(envelope)));
        let command = ApiCommand::call_service(c_store_env);
        self.ano_dhtin.send(command)?;
 
      },
      MainStoreKV::Envelope(ref envelope) => {
        // TODO manage envelope list and probably store it
        println!("--------------------> Env store reach");
        let mut context = self.votes.get_mut(&envelope.votekey).ok_or_else(||no_vote_context())?;
        let valid_env = envelope.check(&context.vote_desc).map_err(|e|StripleMydhtErr(e))?;
        if valid_env {
          println!("an anonymous valid envelop");
        } else {
          println!("an anonymous invalid envelop");
          return Ok(Some(GlobalReply::NoRep));
        }
        context.envelopes.push(envelope.clone());
        // query all enveloppe of anonymous dht : no query currently (add timer to do it). But
        // plain and simple broadcast (TODO allow kvstore to broadcast/query search).

        let mut dests = Vec::with_capacity(context.vote_desc.nb_invit());
        let me_key = self.store_service.me.borrow().get_key_ref();
        for destk in context.vote_desc.invitations.iter().filter(|k|&k[..] != &me_key[..])  {
          // TODO would be way better with peer ref
          dests.push((Some(destk.clone()),None,))
        }
        return Ok(Some(GlobalReply::Forward(
              None,
              Some(dests),
              FWConf {
                nb_for : 0,
                discover : true,
              },
              KVStoreCommand::Store(0,[
                ArcRef::new(MainStoreKV::Envelope(envelope.clone())) 
              ].to_vec())
              )));
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
    }
    } else {
    // distant
    match *kv.borrow() {
      MainStoreKV::VoteDesc(ref votedesc) => {
        unimplemented!()
      },
      MainStoreKV::Envelope(ref envelope) => {
        unimplemented!()
      },
    }
    }
    Ok(None)
  }

}

impl<P : Peer<Key = Vec<u8>, Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP : Ref<P> + Clone,PM : PeerMgmtMeths<P>> Service for VotingService<P,RP,PM>
  {
 
  //KVStoreService<P,RP,MainStoreKV,MainStoreKVRef,MainStoreKVStore,SimpleRules,MainStoreQueryCache<P,RP>> {
  type CommandIn = GlobalCommand<RP,KVStoreCommand<P,RP,MainStoreKV,MainStoreKVRef>>;
  type CommandOut = GlobalReply<P,RP,KVStoreCommand<P,RP,MainStoreKV,MainStoreKVRef>,KVStoreReply<MainStoreKVRef>>;

  fn call<Y : SpawnerYield>(&mut self, req: Self::CommandIn, async_yield : &mut Y) -> Result<Self::CommandOut> {

    let is_local = req.is_local(); 
    // filters :
    match req.get_inner_command() {
      &KVStoreCommand::Store(_,ref vals) => for v in vals.iter() {
        if let Some(r) = self.vote_impl(v,is_local)? {
          return Ok(r);
        }
      },
      &KVStoreCommand::StoreLocally(ref v,..) => if let Some(r) = self.vote_impl(v,is_local)? {
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
