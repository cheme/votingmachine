//! Main MyDHT instance to share content while authenticated
use mydht_tcp_loop::{
  Tcp,
};
use service::VotingService; 
use serde::{Serializer,Deserializer};
use std::borrow::Borrow;
use std::mem::replace;
use mydht_slab::slab::Slab;
use mydht_inefficientmap::inefficientmap::InefficientmapBase2;
use mydht_bincode::Bincode;
use mydht::kvstoreif::{
  KVStore,
  KVCache,
};
use mydht::transportif::{
  Transport,
};
use mydht::dhtimpl::{
  DhtRules,
  SimpleRules,
  SimpleCache,
  SimpleCacheQuery,
  HashMapQuery,
};
use mydht::peer::{
  Peer,
  PeerMgmtMeths,
};
use mydht::keyval::{
  KeyVal,
};
use mydht::utils::{
  Ref,
  ArcRef,
  OneResult,
  SerSocketAddr,
  Proto,
};
use mydht::{
  MyDHTConf,
  RWSlabEntry,
  PeerCacheEntry,
  AddressCacheEntry,
  ChallengeEntry,
  PeerCacheRouteBase,
  LocalReply,
  Api,
  ApiResult,
  ApiQueryId,
  ClientMode,
};
use mydht::storeprop::{
  KVStoreProtoMsgWithPeer,
  KVStoreCommand,
  KVStoreReply,
  KVStoreService,
};
use mydht::noservice::{
  NoCommandReply,
};
use mydht::service::{
  ThreadPark,
  MpscChannel,
  NoSend,
  NoService,
  NoSpawn,
  NoChannel,
};
use std::hash::Hash;
use std::marker::PhantomData;
use std::collections::HashMap;
use mydht::mydhtresult::{
  Result,
};
use std::time::Instant;
use std::time::Duration;
use vote::{
  MainStoreKV,
  MainStoreKVRef,
};

pub type MainStoreKVStore = SimpleCache<MainStoreKV,HashMap<<MainStoreKV as KeyVal>::Key,MainStoreKV>>;
pub type MainStoreQueryCache<P,PR> = SimpleCacheQuery<P,MainStoreKVRef,PR,HashMapQuery<P,MainStoreKVRef,PR>>;

pub struct MainDHTConf<P,PM> {
  pub me : ArcRef<P>,
  pub others : Option<Vec<ArcRef<P>>>,
  // transport in conf is bad, but flexible (otherwhise we could not be generic as we would need
  // transport initialisation parameter in struct : not only address for transport test).
  // Furthermore it makes the conf usable only once.
  pub transport : Option<Tcp>,
  pub msg_enc : Bincode,
  pub peer_mgmt : PM,
  pub rules : SimpleRules,
}

// TODO configure and refactor it
pub const DHTRULES_MAIN : DhtRules = DhtRules {
  randqueryid : true,
  // nbhop = prio * fact
  nbhopfact : 1,
  // nbquery is 1 + query * fact
  nbqueryfact : 1.0, 
  //query lifetime second
  lifetime : 15,
  // increment of lifetime per priority inc
  lifetimeinc : 2,
  cleaninterval : None, // in seconds if needed
  cacheduration : None, // cache in seconds
  cacheproxied : false, // do you cache proxied result
  storelocal : true, // is result stored locally
  storeproxied : None, // store only if less than nbhop
  heavyaccept : false,
  clientmode : ClientMode::ThreadedOne,
  // TODO client mode param + testing for local tcp and mult tcp in max 2 thread and in pool 2
  // thread
  tunnellength : 3,
  not_found_reply : true,
};



impl<P : Peer<Address = SerSocketAddr>, PM : PeerMgmtMeths<P>> MyDHTConf for MainDHTConf<P,PM> 
where <P as KeyVal>::Key : Hash,
      <P as Peer>::Address : Hash,
{
  const SEND_NB_ITER : usize = 10;

  type MainloopSpawn = ThreadPark;
  type MainLoopChannelIn = MpscChannel;
  type MainLoopChannelOut = MpscChannel;

  type Transport = Tcp;
  type MsgEnc = Bincode;
  type Peer = P;
  type PeerRef = ArcRef<P>;
  type PeerMgmtMeths = PM;
  type DHTRules = SimpleRules;
  type Slab = Slab<RWSlabEntry<Self>>;

  type PeerCache = InefficientmapBase2<Self::Peer, Self::PeerRef, PeerCacheEntry<Self::PeerRef>,
    HashMap<<Self::Peer as KeyVal>::Key,PeerCacheEntry<Self::PeerRef>>>;
  type AddressCache = HashMap<<Self::Transport as Transport>::Address,AddressCacheEntry>;
  type ChallengeCache = HashMap<Vec<u8>,ChallengeEntry<Self>>;
  type PeerMgmtChannelIn = MpscChannel;
  type ReadChannelIn = MpscChannel;
  type ReadSpawn = ThreadPark;
  // Placeholder
  type WriteDest = NoSend;
  type WriteChannelIn = MpscChannel;
  type WriteSpawn = ThreadPark;
  type Route = PeerCacheRouteBase;

  // keep val of global service to peer
  type ProtoMsg = KVStoreProtoMsgWithPeer<Self::Peer,Self::PeerRef,MainStoreKV,MainStoreKVRef>;


  nolocal!();

  type GlobalServiceCommand = KVStoreCommand<Self::Peer,Self::PeerRef,MainStoreKV,MainStoreKVRef>;
  type GlobalServiceReply = KVStoreReply<MainStoreKVRef>;
  type GlobalService = VotingService<Self::Peer,Self::PeerRef>;
  type GlobalServiceSpawn = ThreadPark;
  type GlobalServiceChannelIn = MpscChannel;

  // TODO replace by future here is good future use case
  type ApiReturn = OneResult<(Vec<ApiResult<Self>>,usize,usize)>;
  type ApiService = Api<Self,HashMap<ApiQueryId,(OneResult<(Vec<ApiResult<Self>>,usize,usize)>,Instant)>>;
  type ApiServiceSpawn = ThreadPark;
  type ApiServiceChannelIn = MpscChannel;

  type PeerStoreQueryCache = SimpleCacheQuery<Self::Peer,Self::PeerRef,Self::PeerRef,HashMapQuery<Self::Peer,Self::PeerRef,Self::PeerRef>>;
  type PeerKVStore = SimpleCache<Self::Peer,HashMap<<Self::Peer as KeyVal>::Key,Self::Peer>>;
  type PeerStoreServiceSpawn = ThreadPark;
  type PeerStoreServiceChannelIn = MpscChannel;
 
  type SynchListenerSpawn = ThreadPark;

  const NB_SYNCH_CONNECT : usize = 3;
  type SynchConnectChannelIn = MpscChannel;
  type SynchConnectSpawn = ThreadPark;


  fn init_peer_kvstore(&mut self) -> Result<Box<Fn() -> Result<Self::PeerKVStore> + Send>> {
    let others = self.others.clone();
    Ok(Box::new(
      move ||{
        let others = others.clone();
        let mut sc = SimpleCache::new(None);
        if let Some(others) = others {
          debug!("init kvstore with nb val {}",others.len());
          for o in others.into_iter() {
            let p : &P = o.borrow();
            sc.add_val(p.clone(),None);
          }
        }

        Ok(sc)
      }
    ))
  }
  fn do_peer_query_forward_with_discover(&self) -> bool {
    // allow discovering of peer
    true
  }
  fn init_peer_kvstore_query_cache(&mut self) -> Result<Box<Fn() -> Result<Self::PeerStoreQueryCache> + Send>> {
    Ok(Box::new(
      ||{
        // non random id
        Ok(SimpleCacheQuery::new(false))
      }
    ))
  }
  fn init_peerstore_channel_in(&mut self) -> Result<Self::PeerStoreServiceChannelIn> {
    Ok(MpscChannel)
  }
  fn init_peerstore_spawner(&mut self) -> Result<Self::PeerStoreServiceSpawn> {
    Ok(ThreadPark)
  }
//impl<P : Peer, V : KeyVal, RP : Ref<P>> SimpleCacheQuery<P,V,RP,HashMapQuery<P,V,RP>> {
// QueryCache<Self::Peer,Self::PeerRef,Self::PeerRef>;
  fn init_ref_peer(&mut self) -> Result<Self::PeerRef> {
    Ok(self.me.clone())
  }
  fn get_main_spawner(&mut self) -> Result<Self::MainloopSpawn> {
    //Ok(Blocker)
    Ok(ThreadPark)
//      Ok(ThreadParkRef)
  }

  fn init_main_loop_slab_cache(&mut self) -> Result<Self::Slab> {
    Ok(Slab::new())
  }
  fn init_main_loop_peer_cache(&mut self) -> Result<Self::PeerCache> {
    Ok(InefficientmapBase2::new(HashMap::new()))
  }
  fn init_main_loop_address_cache(&mut self) -> Result<Self::AddressCache> {
    Ok(HashMap::new())
  }
 
  fn init_main_loop_challenge_cache(&mut self) -> Result<Self::ChallengeCache> {
    Ok(HashMap::new())
  }


  fn init_main_loop_channel_in(&mut self) -> Result<Self::MainLoopChannelIn> {
    Ok(MpscChannel)
    //Ok(MpscChannelRef)
  }
  fn init_main_loop_channel_out(&mut self) -> Result<Self::MainLoopChannelOut> {
    Ok(MpscChannel)
  }


  fn init_read_spawner(&mut self) -> Result<Self::ReadSpawn> {
    Ok(ThreadPark)
    //Ok(Blocker)
  }

  fn init_write_spawner(&mut self) -> Result<Self::WriteSpawn> {
    Ok(ThreadPark)
    //Ok(Blocker)
  }

  fn init_global_spawner(&mut self) -> Result<Self::GlobalServiceSpawn> {
    Ok(ThreadPark)
    //Ok(Blocker)
  }


  fn init_write_spawner_out() -> Result<Self::WriteDest> {
    Ok(NoSend)
  }
  fn init_read_channel_in(&mut self) -> Result<Self::ReadChannelIn> {
    Ok(MpscChannel)
  }
  fn init_write_channel_in(&mut self) -> Result<Self::WriteChannelIn> {
//      Ok(LocalRcChannel)
    Ok(MpscChannel)
  }
  fn init_peermgmt_channel_in(&mut self) -> Result<Self::PeerMgmtChannelIn> {
    Ok(MpscChannel)
  }


  fn init_enc_proto(&mut self) -> Result<Self::MsgEnc> {
    Ok(self.msg_enc.get_new())
  }

  fn init_transport(&mut self) -> Result<Self::Transport> {
    Ok(replace(&mut self.transport,None).unwrap())
  }
  fn init_peermgmt_proto(&mut self) -> Result<Self::PeerMgmtMeths> {
    Ok(self.peer_mgmt.clone())
  }
  fn init_dhtrules_proto(&mut self) -> Result<Self::DHTRules> {
    Ok(self.rules.clone())
  }

  fn init_global_service(&mut self) -> Result<Self::GlobalService> {
    let i_store = Box::new(
      ||{
        Ok(SimpleCache::new(None))
      });
    let i_cache = Box::new(
      ||{
        Ok(SimpleCacheQuery::new(false))
      }
    );
    Ok(VotingService {
      store_service : KVStoreService {
        me : self.init_ref_peer()?,
        init_store : i_store,
        init_cache : i_cache,
        store : None,
        dht_rules : self.init_dhtrules_proto()?,
        query_cache : None,
        discover : true,
        _ph : PhantomData,
      }
    })
  }

  fn init_global_channel_in(&mut self) -> Result<Self::GlobalServiceChannelIn> {
    Ok(MpscChannel)
  }

  fn init_route(&mut self) -> Result<Self::Route> {
    Ok(PeerCacheRouteBase)
  }

  fn init_api_service(&mut self) -> Result<Self::ApiService> {
    Ok(Api(HashMap::new(),Duration::from_millis(3000),0,PhantomData))
  }

  fn init_api_channel_in(&mut self) -> Result<Self::ApiServiceChannelIn> {
    Ok(MpscChannel)
  }
  fn init_api_spawner(&mut self) -> Result<Self::ApiServiceSpawn> {
    Ok(ThreadPark)
    //Ok(Blocker)
  }
  fn init_synch_listener_spawn(&mut self) -> Result<Self::SynchListenerSpawn> {
    Ok(ThreadPark)
  }

  fn init_synch_connect_spawn(&mut self) -> Result<Self::SynchConnectSpawn> {
    Ok(ThreadPark)
  }
  fn init_synch_connect_channel_in(&mut self) -> Result<Self::SynchConnectChannelIn> {
    Ok(MpscChannel)
  }


}

