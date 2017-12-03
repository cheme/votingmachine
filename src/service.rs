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


impl<P : Peer,RP : Ref<P> + Clone> Service for VotingService<P,RP> {
  //KVStoreService<P,RP,MainStoreKV,MainStoreKVRef,MainStoreKVStore,SimpleRules,MainStoreQueryCache<P,RP>> {
  type CommandIn = GlobalCommand<RP,KVStoreCommand<P,RP,MainStoreKV,MainStoreKVRef>>;
  type CommandOut = GlobalReply<P,RP,KVStoreCommand<P,RP,MainStoreKV,MainStoreKVRef>,KVStoreReply<MainStoreKVRef>>;

  fn call<Y : SpawnerYield>(&mut self, req: Self::CommandIn, async_yield : &mut Y) -> Result<Self::CommandOut> {

    // filters :

    // query votedesc : if no participation set for votedesc : reply only if user  in list of voters -> create a command for it that use owith in command TODO evo MyDHT to add owith to local or global commad
    // for a start simply reply?? (key being the secret)
    
    self.store_service.call(req,async_yield)
  }
}
 
