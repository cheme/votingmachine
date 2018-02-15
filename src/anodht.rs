//! Anonymous MyDHT instance to share enveloppe and votes through tunnel

extern crate sized_windows_lim;
use maindht::{
  MainDHTConf,
};
use mydht::dhtimpl::{
  NoShadow,
  Cache,
}; 
use mydht_openssl::rsa_openssl::{
  OSSLSym,
  OSSLSymW,
  OSSLSymR,
  OpenSSLSymConf,
  AES256CBC,
};
use mydht_tcp_loop::{
  Tcp,
};
use rand::{
  Rng,
  OsRng,
};
use mydht::{
  PeerStatusListener,
  PeerStatusCommand,
  MCCommand,
  MCReply,
  Route,
  IndexableWriteCache,
  PeerPriority,
  MainLoopCommand,
};

use mydht::api::{
  Api,
  ApiResult,
  ApiResultSend,
  ApiCommand,
  ApiQueriable,
  ApiQueryId,
  ApiRepliable,
};
use vote::{
  Envelope,
};


use service::VotingService; 
use mydht::service::{
  Service,
  SpawnerYield,
  SpawnChannel,
  SpawnSend,
  MioSend,
};
use mydht::{
  GlobalCommand,
  GlobalReply,
};

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
  OptFrom,
  ArcRef,
  CloneRef,
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
use mydht_tunnel::{
  MyDHTTunnelConf,
  MyDHTTunnelConfType,
  GlobalTunnelReply,
  SSWCache,
  SSRCache,
};
use mydht_tunnel::reexp::{
  ErrorWriter,
  MultipleErrorInfo,
  MultipleReplyMode,
  MultipleErrorMode,
  SymProvider,
};
use self::sized_windows_lim::{
  SizedWindowsParams,
  SizedWindows,
};
use mydht::keyval::{
  SettableAttachment,
  SettableAttachments,
  GettableAttachments,
  Attachment,
};
use mydht::api::{
  DHTIn,
};

// local type alias
type MLSend<MC : MyDHTConf> = <MC::MainLoopChannelIn as SpawnChannel<MainLoopCommand<MC>>>::Send;

pub trait AnoAddress {
  type Address;
  fn get_sec_address (&self) -> &Self::Address;
}

pub type AnoDHTConf<P,SP,SI> = MyDHTTunnelConfType<AnoTunDHTConf<P,SP,SI>>;

pub type AnoTunDHTConf2<P,PM> = AnoTunDHTConf<P,PM,MainDHTConf<P,PM>>;

pub fn new_ano_conf<P : Peer<Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>, PM : PeerMgmtMeths<P>, 
  >(tc : AnoTunDHTConf2<P,PM>)
 -> Result<AnoDHTConf<P,PM,MainDHTConf<P,PM>>> 
where <P as KeyVal>::Key : Hash,
      //MainLoopSendIn<SI> : Send,
      //MLSend<SI> : Send,
{
  MyDHTTunnelConfType::new(
    tc,
    // no reply
    MultipleReplyMode::NoHandling,
    // no error 
    MultipleErrorMode::NoHandling,
    // default value for nb hops
    None,
    None)
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(bound(deserialize = ""))]
pub struct AnoPeer<P : Peer + AnoAddress<Address = SerSocketAddr>> (pub ArcRef<P>);

impl<P : Peer + AnoAddress<Address = SerSocketAddr>> KeyVal for AnoPeer<P> {
  type Key = <P as KeyVal>::Key;
  fn attachment_expected_size(&self) -> usize {
    let inner : &P = self.0.borrow();
    inner.attachment_expected_size()
  }
  fn get_key_ref(&self) -> &Self::Key {
    let inner : &P = self.0.borrow();
    inner.get_key_ref()
  }
  fn get_key(&self) -> Self::Key {
    let inner : &P = self.0.borrow();
    inner.get_key()
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    let inner : &P = self.0.borrow();
    inner.get_attachment()
  }
}

impl<P : Peer + AnoAddress<Address = SerSocketAddr>> SettableAttachment for AnoPeer<P> { }
 
impl<P : Peer + AnoAddress<Address = SerSocketAddr>> Peer for AnoPeer<P> {
  type Address = <P as AnoAddress>::Address;
  type ShadowWAuth = <P as Peer>::ShadowWMsg;
  type ShadowRAuth = <P as Peer>::ShadowRMsg;
  type ShadowWMsg = NoShadow;
  type ShadowRMsg = NoShadow;
  fn get_address (&self) -> &Self::Address {
    let inner : &P = self.0.borrow();
    inner.get_sec_address()
  }
  fn to_address (&self) -> Self::Address {
    let inner : &P = self.0.borrow();
    inner.get_sec_address().clone()
  }
  // shadower msg could be better TODO test
  fn get_shadower_r_auth (&self) -> Self::ShadowRAuth {
    let inner : &P = self.0.borrow();
    inner.get_shadower_r_msg()
  }
  // tunnel dht is currently running on NoAuth, so the msg shadower is used
  // rsa peer w shadower is not compatible with this mode : using NoShadow
  fn get_shadower_r_msg (&self) -> Self::ShadowRMsg {
    NoShadow
  }
  fn get_shadower_w_auth (&self) -> Self::ShadowWAuth {
    let inner : &P = self.0.borrow();
    inner.get_shadower_w_msg()
  }
  fn get_shadower_w_msg (&self) -> Self::ShadowWMsg {
    NoShadow
  }

}

pub struct AnoTunDHTConf<P,PM,SI : MyDHTConf> {
  pub conf : MainDHTConf<P,PM>,
  // sibling dht api input : only a send as we address directly the mio service (otherwhise a
  // handle would be needed (added optional for possible smarter addressing through weak handle)!!!
  // In fact handle could be include in a spawn send composition if neede
  pub main_api : Option<DHTIn<SI>>,
}


/*pub struct AnoTunDHTConf<P,PM> {
  pub me : ArcRef<P>,
  pub others : Option<Vec<ArcRef<P>>>,
  // transport in conf is bad, but flexible (otherwhise we could not be generic as we would need
  // transport initialisation parameter in struct : not only address for transport test).
  // Furthermore it makes the conf usable only once.
  pub transport : Option<Tcp>,
  pub msg_enc : Bincode,
  pub peer_mgmt : PM,
  pub rules : SimpleRules,
}*/

impl<
  P : Peer<Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>, 
  PM : PeerMgmtMeths<P>, 
//  SI : MyDHTConf
  > MyDHTTunnelConf for AnoTunDHTConf2<P,PM> 
where <P as KeyVal>::Key : Hash,
      <P as AnoAddress>::Address : Hash,
//      MLSend<SI> : Send,
{
  const INIT_ROUTE_LENGTH : usize = 4;
  const INIT_ROUTE_BIAS : usize = 0;
  type PeerKey = <Self::Peer as KeyVal>::Key;
  type Peer = AnoPeer<P>;
  type PeerRef = CloneRef<AnoPeer<P>>;
  type InnerCommand = AnoServiceICommand;
  type InnerReply = AnoServiceIReply;
  type InnerService = AnoService<Self,MainDHTConf<P,PM>>;
  type InnerServiceProto = MioSend<MLSend<MainDHTConf<P,PM>>>;
  type Transport = Tcp;
  type TransportAddress = SerSocketAddr;
  type MsgEnc = Bincode;
  type PeerMgmtMeths = AnoPeerMgmt<PM>;
  type DHTRules = SimpleRules;
  type ProtoMsg = StoreAnoMsg;
//  type PeerCache = HashMap<<Self::Peer as KeyVal>::Key,PeerCacheEntry<Self::PeerRef>>;
  type PeerCache = InefficientmapBase2<Self::Peer, Self::PeerRef, PeerCacheEntry<Self::PeerRef>,
    HashMap<<Self::Peer as KeyVal>::Key,PeerCacheEntry<Self::PeerRef>>>;
 
  type AddressCache = HashMap<<Self::Peer as Peer>::Address,AddressCacheEntry>;
  type ChallengeCache = HashMap<Vec<u8>,ChallengeEntry<MyDHTTunnelConfType<Self>>>;
  /// must be random as it decide which peer will store (not use to build tunnel but to choose
  /// dest) -> actually not
  type Route = RandomRoute;
  type PeerKVStore = SimpleCache<Self::Peer,HashMap<<Self::Peer as KeyVal>::Key,Self::Peer>>;

  type LimiterW = SizedWindows<AnoSizedWindows>;
  type LimiterR = SizedWindows<AnoSizedWindows>;

  type SSW = OSSLSymW<AES256CBC>;
  type SSR = OSSLSymR<AES256CBC>;
  type SP = OpenSSLSymProvider<AES256CBC>;

  type CacheSSW = HashMap<Vec<u8>,SSWCache<Self>>;
  type CacheSSR = HashMap<Vec<u8>,SSRCache<Self>>;
  type CacheErW = HashMap<Vec<u8>,(ErrorWriter,<Self::Transport as Transport>::Address)>;
  type CacheErR = HashMap<Vec<u8>,Vec<MultipleErrorInfo>>;
 
  fn init_ref_peer(&mut self) -> Result<Self::PeerRef> {
    Ok(CloneRef::new(AnoPeer(self.conf.me.clone())))
  }

  fn init_inner_service_proto(&mut self) -> Result<Self::InnerServiceProto> {
    let maindhtin = replace(&mut self.main_api,None).unwrap();
    Ok(maindhtin.main_loop)
  }

  fn init_inner_service(maindhtin : Self::InnerServiceProto, me : Self::PeerRef) -> Result<Self::InnerService> {
    Ok(AnoService(me,DHTIn {
      main_loop : maindhtin,
    }))
  }

  fn init_peer_kvstore(&mut self) -> Result<Box<Fn() -> Result<Self::PeerKVStore> + Send>> {
    let others = self.conf.others.clone();
    Ok(Box::new(
      move ||{
        let others = others.clone();
        let mut sc = SimpleCache::new(None);
        if let Some(others) = others {
          debug!("init kvstore with nb val {}",others.len());
          for o in others.into_iter() {
//            let p : &P = o.borrow();
            sc.add_val(AnoPeer(o),None);
          }
        }

        Ok(sc)
      }
    ))
  }

  fn init_transport(&mut self) -> Result<Self::Transport> {
    Ok(replace(&mut self.conf.transport,None).unwrap())
  }

  fn init_peermgmt_proto(&mut self) -> Result<Self::PeerMgmtMeths> {
    Ok(AnoPeerMgmt(self.conf.peer_mgmt.clone()))
  }

  fn init_dhtrules_proto(&mut self) -> Result<Self::DHTRules> {
    Ok(self.conf.rules.clone())
  }

  fn init_enc_proto(&mut self) -> Result<Self::MsgEnc> {
    Ok(self.conf.msg_enc.get_new())
  }

  fn init_route(&mut self) -> Result<Self::Route> {
    Ok(RandomRoute(OsRng::new()?))
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
  fn init_cache_ssw(&mut self) -> Result<Self::CacheSSW> {
    Ok(HashMap::new())
  }
  fn init_cache_ssr(&mut self) -> Result<Self::CacheSSR> {
    Ok(HashMap::new())
  }
  fn init_cache_err(&mut self) -> Result<Self::CacheErR> {
    Ok(HashMap::new())
  }
  fn init_cache_erw(&mut self) -> Result<Self::CacheErW> {
    Ok(HashMap::new())
  }
  fn init_shadow_provider(&mut self) -> Result<Self::SP> {
    Ok(OpenSSLSymProvider{ _conf : PhantomData })
  }
  fn init_limiter_w(&mut self) -> Result<Self::LimiterW> {
    Ok(SizedWindows::new(AnoSizedWindows))
  }
  fn init_limiter_r(&mut self) -> Result<Self::LimiterR> {
    Ok(SizedWindows::new(AnoSizedWindows))
  }
}

#[derive(Clone)]
// TODO put it in tunnel openssl crate ?
pub struct OpenSSLSymProvider<C : OpenSSLSymConf> {
  /// using salt is not always needed : currently we use a new key everytime
  pub _conf : PhantomData<C>,
}

unsafe impl<C : OpenSSLSymConf> Send for OpenSSLSymProvider<C> {}

impl<C : OpenSSLSymConf> SymProvider<OSSLSymW<C>,OSSLSymR<C>> for OpenSSLSymProvider<C> {
  fn new_sym_key (&mut self) -> Vec<u8> {
    <OSSLSym<C>>::new_key().unwrap()
  }
  fn new_sym_writer (&mut self, key : Vec<u8>) -> OSSLSymW<C> {
    OSSLSymW(
      <OSSLSym<C>>::new(key,true).unwrap()
    )
  }
  fn new_sym_reader (&mut self, key : Vec<u8>) -> OSSLSymR<C> {
    let sym = <OSSLSym<C>>::new(key,false).unwrap();
    OSSLSymR::from_read_sym(sym)
  }
}

#[derive(Clone)]
pub struct AnoSizedWindows;

impl SizedWindowsParams for AnoSizedWindows {
//    const INIT_SIZE : usize = 45;
    const INIT_SIZE : usize = 150;
    const MAX_SIZE : usize = 2048;
    const GROWTH_RATIO : Option<(usize,usize)> = Some((3,2));
    const WRITE_SIZE : bool = true;
    const SECURE_PAD : bool = false;
}


/// TODO move to mydht ??
pub struct RandomRoute(OsRng);

impl<MC : MyDHTConf> Route<MC> for RandomRoute 
  where MC::PeerCache : IndexableWriteCache 
//where MC : MyDHTConf<PeerCache = InefficientmapBase2<MC::Peer, MC::PeerRef, PeerCacheEntry<MC::PeerRef>,
//    HashMap<<MC::Peer as KeyVal>::Key,PeerCacheEntry<MC::PeerRef>>>>
{

  /// for testing we build tunnel with this route : simply get from cache plus could contain the
  /// dest (not an issue I think (self hop should be fine)).
  fn route(&mut self, 
           targetted_nb : usize, 
           c : MCCommand<MC>,
           slab : &mut <MC as MyDHTConf>::Slab, 
           cache : &mut <MC as MyDHTConf>::PeerCache) 
    -> Result<(MCCommand<MC>,Vec<usize>)> {
    let totl = cache.len_c();
    let mut res : Vec<usize> = Vec::new();
    // TODO define rule  to get some margin
    if targetted_nb > cache.len_c() {
      return Ok((c,res))
    }
    let mut rem = totl;
    while rem > 0 { 
      let r_ix = (self.0.next_u64() as usize) % totl;
      if let Some(a) = cache.get_at(r_ix) {
        if !res.contains(&a) {
          res.push(a);
          rem -= 1;
        } else {
          // TODO reduce search space somehow (next after some tries...)
        }
      }
    }
    Ok((c,res))
  }
}


#[derive(Clone,Serialize,Deserialize,Debug)]
#[serde(bound(deserialize = ""))]
pub enum StoreAnoMsg {
  STORE_ENVELOPE(Envelope),
}

impl SettableAttachments for StoreAnoMsg {
  fn attachment_expected_sizes(&self) -> Vec<usize> { Vec::new() }
  fn set_attachments(& mut self, _ : &[Attachment]) -> bool { false }
}

impl GettableAttachments for StoreAnoMsg {
  fn get_attachments(&self) -> Vec<&Attachment> { Vec::new() }
  fn get_nb_attachments(&self) -> usize { 0 }
}

/// TODO when use case finalize : consider replacing dhtin with kvstore spawsend + handle
pub struct AnoService<MC : MyDHTTunnelConf, SI : MyDHTConf>(<MC as MyDHTTunnelConf>::PeerRef, DHTIn<SI>);

#[derive(Clone)]
pub struct AnoServiceICommand(pub StoreAnoMsg);

impl OptFrom<AnoServiceICommand> for StoreAnoMsg {
  fn can_from(_ : &AnoServiceICommand) -> bool { true }
  fn opt_from(c : AnoServiceICommand) -> Option<Self> {
    Some(c.0)
  }
}

impl From<StoreAnoMsg> for AnoServiceICommand {
  fn from(c: StoreAnoMsg) -> Self {
    AnoServiceICommand(c)
  }
}

#[derive(Clone)]
pub struct AnoServiceIReply;

// no api query on tunnel
impl ApiQueriable for AnoServiceIReply {
  fn is_api_reply(&self) -> bool { false }
  fn set_api_reply(&mut self, _ : ApiQueryId) { }
  fn get_api_reply(&self) -> Option<ApiQueryId> { None }
}

impl ApiQueriable for AnoServiceICommand {
  fn is_api_reply(&self) -> bool { false }
  fn set_api_reply(&mut self, _ : ApiQueryId) { }
  fn get_api_reply(&self) -> Option<ApiQueryId> { None }
}


impl<P> PeerStatusListener<P> for AnoServiceICommand {
  const DO_LISTEN : bool = false;
  fn build_command(c : PeerStatusCommand<P>) -> Self {
    unreachable!()
  }
}
impl<
  P : Peer<Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,
  PM : PeerMgmtMeths<P>, 
  > Service for AnoService<AnoTunDHTConf2<P,PM>,MainDHTConf<P,PM>>
where <P as KeyVal>::Key : Hash,
      <P as AnoAddress>::Address : Hash,
{
  type CommandIn = GlobalCommand<<AnoTunDHTConf2<P,PM> as MyDHTTunnelConf>::PeerRef,<AnoTunDHTConf2<P,PM> as MyDHTTunnelConf>::InnerCommand>;
  //type CommandOut = GlobalTunnelReply<C>;
  type CommandOut = GlobalTunnelReply<AnoTunDHTConf2<P,PM>>;
 
//impl<C : MyDHTTunnelConf<InnerCommand = AnoServiceICommand>> Service for AnoService<C> {
  //type CommandOut = GlobalTunnelReply<C>;
  fn call<S : SpawnerYield>(&mut self, req: Self::CommandIn, _async_yield : &mut S) -> Result<Self::CommandOut> {
    match req {

      GlobalCommand::Distant(_opr,AnoServiceICommand(StoreAnoMsg::STORE_ENVELOPE(envelope))) => {

        let enveloperef = ArcRef::new(MainStoreKV::Envelope(envelope));
        let c_store_env = ApiCommand::call_service(KVStoreCommand::StoreLocally(enveloperef,1,None));
        self.1.send(c_store_env)?;
        Ok(GlobalTunnelReply::NoRep)
      },
      GlobalCommand::Local(AnoServiceICommand(StoreAnoMsg::STORE_ENVELOPE(envelope))) => {
        // proxy message
        Ok(GlobalTunnelReply::SendCommandToRand(AnoServiceICommand(StoreAnoMsg::STORE_ENVELOPE(envelope))))
      },
    }
  }
}

#[derive(Clone)]
pub struct AnoPeerMgmt<PM>(PM);
impl<P : Peer + AnoAddress<Address = SerSocketAddr>, PM : PeerMgmtMeths<P>>
  PeerMgmtMeths<AnoPeer<P>> for AnoPeerMgmt<PM> {
  fn challenge (&self, p : &AnoPeer<P>) -> Vec<u8> {
    let inner : &P = p.0.borrow();
    self.0.challenge(inner)
  }
  fn signmsg (&self, p : &AnoPeer<P>, ch : &[u8]) -> Vec<u8> {
    let inner : &P = p.0.borrow();
    self.0.signmsg(inner,ch)
  }
  fn checkmsg (&self, p : &AnoPeer<P>, ch : &[u8], sig : &[u8]) -> bool {
    let inner : &P = p.0.borrow();
    self.0.checkmsg(inner,ch,sig)
  }
  fn accept (&self, p : &AnoPeer<P>) -> Option<PeerPriority> {
    let inner : &P = p.0.borrow();
    self.0.accept(inner)
  }

}
