//! main service (in the main dht) for internal logic (synch of content ...),
//! with inner standerad kvstore service

use mydht::dhtimpl::{
  SimpleRules,
};
use mydht::mydhtresult::{
  Result,
};
use mydht::peer::{
  Peer,
};
use mydht::utils::{
  Ref,
};
use mydht::service::{
  Service,
  SpawnerYield,
};
use mydht::storeprop::{
  KVStoreProtoMsgWithPeer,
  KVStoreCommand,
  KVStoreReply,
  KVStoreService,
};
use vote::{
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
};
pub struct VotingService<P : Peer,RP> {
  pub store_service : KVStoreService<
    P,
    RP,
    MainStoreKV,
    MainStoreKVRef,
    MainStoreKVStore,
    SimpleRules,
    MainStoreQueryCache<P,RP>
  >,
}
use std::borrow::Borrow;

impl<P : Peer,RP : Ref<P> + Clone> VotingService<P,RP> {
  fn vote_impl(&mut self, kv: &MainStoreKVRef) -> Result<Option<<Self as Service>::CommandOut>> {
    match *kv.borrow() {
      MainStoreKV::VoteDesc(ref vd) => {
        // TODO let global service listen peer update and query check this vd
      },
    }
    Ok(None)
  }
}

impl<P : Peer,RP : Ref<P> + Clone> Service for VotingService<P,RP> {
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
