//! striple implementation of various vote objects

use mydht::utils::TimeSpecExt;
use time::{
  self,
  Timespec,
  Duration,
};
use bincode;
use bincode::Error as BinError;
use std::result::Result as StdResult;
use std::io::Cursor;
use std::fs::File;
use striple::striple::{
  NOKEY,
  ByteSlice,
  InstantiableStripleImpl,
  StripleIf,
  StripleFieldsIf,
  StripleImpl,
  StripleKind,
  BCont,
  from_error,
  from_option,
  Striple,
  ref_builder_id_copy,
  Result as StripleResult,
  Error as StripleError,
};

use striple::storage::{
  FileStripleIterator,
  init_noread_key,
  init_any_cipher_stdin,
};
use serde::{Serializer,Deserializer};
use vote::{
  VoteDesc,
  VoteDescStripleContent,
  
};
use mydht::mydhtresult::Result as MResult;
use mydht::mydhtresult::Error as MError;
use mydht::mydhtresult::ErrorKind as MErrorKind;
use std::marker::PhantomData;
use striple::striple::{
  StripleRef,
  AsStriple,
};
use striple::anystriple::{
  Rsa2048Sha512,
  PubSha512,
  BASE_STRIPLES,
};

use mydht_openssl::rsa_openssl::{
  RSAPeer,
  RSA2048SHA512AES256,
  OpenSSLConf,
};

use mydht::transportif::Address;

use mydht::dhtif::{
  Peer,
  Key,
  Key as KVContent,
  KeyVal,
};
use mydht::keyval::{
  SettableAttachment,
  Attachment,
};

pub struct StripleRefs {
  // peers are signed from a public striple kind
  // this kind is build with striple command (from base)
  // striple 
  // we use libcat for about and root for from
  // peerkind=$( echo "Peer " | base64 -w 0 )
  // striple create --kindfile ./base.data -x 8 --fromfile ./base.data -x 0 --aboutfile ./base.data -x 3 --content ${peerkind} -o ./testp -c PBKDF2 --outpass "pass"
  pub pub_peer : Striple<PubSha512>,
}

lazy_static!{
pub static ref STRIPLEREFS : StripleRefs = init_striple_refs().unwrap();
}


fn init_striple_refs() -> StripleResult<StripleRefs> {
  // TODO param it 
  let datafile = from_error(File::open("./refs.data"))?;
  // get striple without key and without Kind (as we define it)
  let rit : StdResult<FileStripleIterator<PubSha512,Striple<PubSha512>,_,_,_>,_> = FileStripleIterator::init(datafile, ref_builder_id_copy, &init_noread_key, ()); 
  let mut it = rit?;
  let pub_peer = (from_option(it.next())?).0;
  // check it
  assert!(pub_peer.check(&BASE_STRIPLES.root.0)?);
  Ok(StripleRefs {
    pub_peer,
  })
 
}
/*
#[derive(Debug,Clone)]
/// associate peer rsa conf with striple kind conf
/// Warning no deps between crates, implementation must be check for all new pairs
/// TODO test case on it
pub struct StriplePeerConf<K : StripleKind,C : OpenSSLConf>(pub K,pub C);
impl<K : StripleKind,C : OpenSSLConf> StripleKind for StriplePeerConf<K,C> {
  type D = <K as StripleKind>::D;
  type S = <K as StripleKind>::S;

  fn get_algo_key() -> &'static [u8] {
    <K as StripleKind>::get_algo_key()
  }
}

impl<K : StripleKind,C : OpenSSLConf> OpenSSLConf for StriplePeerConf<K,C> {
  fn HASH_SIGN() -> MessageDigest {
    <C as OpenSSLConf>::HASH_SIGN()
  }
  fn HASH_KEY() -> MessageDigest {
    <C as OpenSSLConf>::HASH_KEY()
  }
  const RSA_SIZE : u32 = <C as OpenSSLConf>::RSA_SIZE;
  fn SHADOW_TYPE() -> SymmType {
    <C as OpenSSLConf>::SHADOW_TYPE()
  }
  const CRYPTER_KEY_ENC_SIZE : usize = <C as OpenSSLConf>::CRYPTER_KEY_ENC_SIZE;
  const CRYPTER_KEY_DEC_SIZE : usize = <C as OpenSSLConf>::CRYPTER_KEY_DEC_SIZE;

  const CRYPTER_ASYM_BUFF_SIZE_ENC : usize = <C as OpenSSLConf>::CRYPTER_ASYM_BUFF_SIZE_ENC;
  const CRYPTER_ASYM_BUFF_SIZE_DEC : usize = <C as OpenSSLConf>::CRYPTER_ASYM_BUFF_SIZE_DEC;
  fn  CRYPTER_BUFF_SIZE() -> usize {
    <C as OpenSSLConf>::CRYPTER_BUFF_SIZE()
  }

}
*/

// TODO move this impl??
impl<A : KVContent,B : Address> StripleImpl for StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
  type Kind = Rsa2048Sha512;
}

impl<A : KVContent,B : Address> InstantiableStripleImpl for StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
  // TODO use a variant of instantiable to use from as an Arc or RC, not for poc
  // other idea is changing init to use &[u8] for from and not adding it to striple peer (get from
  // using lazy one (lifetime issue)
  fn init(&mut self,
    from : Vec<u8>,
    sig : Vec<u8>,
    id : Vec<u8>) {
    // assume same derivation fro rsapeer and striple
    self.id = id;
    self.sig = sig;
    self.from = from;
  }
}


pub struct StripleMydhtErr(StripleError);
impl From<StripleMydhtErr> for MError {
  #[inline]
  fn from(e : StripleMydhtErr) -> MError {
    MError((e.0).0, MErrorKind::ExternalLib, (e.0).2)
  }
}

impl<A : KVContent,B : Address> StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
  pub fn new(p : RSAPeer<A,B,RSA2048SHA512AES256>) -> MResult<Self> {
    let mut peer = StriplePeer {
      inner : p,
      id : Vec::new(),
      sig : Vec::new(),
      from : Vec::new(),
      content : None,
    };

    peer.init_content();
    // very wrong
    peer.calc_init(&STRIPLEREFS.pub_peer).map_err(|e|StripleMydhtErr(e))?;
    Ok(peer)
  }
  // TODO this is call manually , check how to integrate it to serde deserialization
  // (deserialize_with ?? or call back after struct deser?)
  pub fn init_content(&mut self) {
    let mut dest = Cursor::new(Vec::new());
    bincode::serialize_into(&mut dest, &self.inner.peerinfo, bincode::Infinite).unwrap();
    self.content = Some(BCont::OwnedBytes(dest.into_inner()));
  }
}
/*  pub fn init_content<A : KVContent,B : Address>(p : &mut StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>>) {
    let mut dest = Cursor::new(Vec::new());
    bincode::serialize_into(&mut dest, &p.inner.peerinfo, bincode::Infinite).unwrap();
    p.content = Some(BCont::OwnedBytes(dest.into_inner()));
  }*/


//impl<A : KVContent,B : Address, C : OpenSSLConf> StripleFieldsIf for StriplePeer<RSAPeer<A,B,C>> {
impl<A : KVContent,B : Address> StripleFieldsIf for StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
 
  fn get_algo_key(&self) -> ByteSlice {
    ByteSlice::Static(<<Self as StripleImpl>::Kind as StripleKind>::get_algo_key())
  }

  fn get_enc(&self) -> ByteSlice {
    // TODO add a striple
    ByteSlice::Static(NOKEY)
  }

  fn get_id(&self) -> &[u8] {
    &self.id[..]
  }

  fn get_from(&self) -> ByteSlice {
    ByteSlice::Owned(&self.from[..])
  }

  fn get_about(&self) -> ByteSlice {
    // this is null about TODO create a striple to fill it ? (from is already defining user : about
    // could be 'is an instance of'
    ByteSlice::Owned(&self.from[..])
  }

  /// Warning, this implies that rsa peer peerinfo is immutable (otherwhise peer will not check)
  /// This is a shortcut for poc, next this BCont ref will be seen as wrong and a read should be
  /// return (for get_tosig).
  fn get_content<'a>(&'a self) -> Option<&'a BCont<'a>> {
    self.content.as_ref()
  }

  fn get_content_ids(&self) -> Vec<&[u8]> {
    Vec::new()
  }

  fn get_key(&self) -> &[u8] {
    self.inner.get_pub_key_ref()
  }

  fn get_sig(&self) -> &[u8] {
    &self.sig[..]
  }

 
}

/*
// for poc we do not load a full striple hierarchy and use dummy trusted striple (in real world the
// root only should be use) : here we cannot check : those striple must be seen as public one
const PEER_STRIPLE_ID : Vec<u8> = 1;
const VOTEDESC_STRIPLE_ID = 2;
const ENVELOPPE_STRIPLE_ID = 3;
const REL_PEER_ENV_VOTE_STRIPLE_ID = 4;
*/

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(bound(deserialize = ""))]
pub struct StriplePeer<P : Peer> {
  pub inner : P,


  id : Vec<u8>,
  from : Vec<u8>,
  sig : Vec<u8>,

  //#[serde(skip_serializing,deserialize_with="init_content")]
  #[serde(skip_serializing,skip_deserializing)]
  /// obviously wrong, striple lib need some refacto to avoid such a buffer
  /// (or allow bcont as bytes producer (meaning Read) from self)
  content : Option<BCont<'static>>,
}
impl<P : Peer> PartialEq for StriplePeer<P> {

  /// fast comp (fine if striple are checked)
  fn eq(&self, other: &StriplePeer<P>) -> bool {
    self.id == other.id
  }
}

impl<P : Peer> Eq for StriplePeer<P> {
}

impl<P : Peer> Peer for StriplePeer<P> {
  type Address = <P as Peer>::Address;
  type ShadowWAuth = <P as Peer>::ShadowWAuth;
  type ShadowRAuth = <P as Peer>::ShadowRAuth;
  type ShadowWMsg = <P as Peer>::ShadowWMsg;
  type ShadowRMsg = <P as Peer>::ShadowRMsg;
  fn get_address (&self) -> &Self::Address {
    self.inner.get_address()
  }
  fn to_address (&self) -> Self::Address {
    self.inner.to_address()
  }
  fn get_shadower_r_auth (&self) -> Self::ShadowRAuth {
    self.inner.get_shadower_r_auth()
  }
  fn get_shadower_r_msg (&self) -> Self::ShadowRMsg {
    self.inner.get_shadower_r_msg()
  }
  fn get_shadower_w_auth (&self) -> Self::ShadowWAuth {
    self.inner.get_shadower_w_auth()
  }
  fn get_shadower_w_msg (&self) -> Self::ShadowWMsg {
    self.inner.get_shadower_w_msg()
  }
}
/// replace rsa id by striple id (more info in striple id)
impl<P : Peer> KeyVal for StriplePeer<P> {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize {
    self.inner.attachment_expected_size()
  }
  fn get_key_ref(&self) -> &Self::Key {
    &self.id
  }
  fn get_key(&self) -> Self::Key {
    self.id.clone()
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    self.inner.get_attachment()
  }
  fn encode_kv<S:Serializer> (&self, s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("TODO rem from trait")
  }
  /// First boolean indicates if the encoding is locally used (not send to other peers).
  /// Second boolean indicates if attachment must be added in the encoding (or just a reference
  /// kept).
  /// Default implementation decode through encode trait.
  fn decode_kv<'de,D:Deserializer<'de>> (d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("TODO rem from trait")
  }

}
impl<P : Peer> SettableAttachment for StriplePeer<P> { }
/*
pub struct RSAPeer<I : KVContent,A : Address,C : OpenSSLConf> {
  /// key to use to identify peer, derived from publickey it is shorter
  key : Vec<u8>,
  /// is used as id/key TODO maybe two publickey use of a master(in case of compromition)
  publickey : PKeyExt<C>,

  pub address : A,

  /// local info
  pub peerinfo : I,
  
}*/


/*
impl<'a, P : StriplePeerIf> AsStriple<'a, <P as StriplePeerIf>::Kind> for StriplePeer<P> {
  type Target = StripleRef<'a,<P as StriplePeerIf>::Kind>;
  fn as_striple(&'a self) -> Self::Target {
    unimplemented!()
/*    StripleRef {
      // no enc (or bincode peerinfo??)
      contentenc : &'a[u8],
      // striple_key
      id         : &'a[u8],
      // static public voting peer : sha512 sign scheme : TODO this striple
      from       : &'a[u8],
      // peer sig 
      sig        : &'a[u8],
      // static peer striple id 
      about      : &'a[u8],
      // rsa peer publickey as bytes
      key        : &'a[u8],
      // none
      contentids : Vec<&'a[u8]>,
      // rsa peerinfo?? or rsa peer key??
      content : Option<BCont<'a>>,

      phtype : PhantomData,
    }*/
  }
}
*/
// TODO Striple Peer mgmt over rsa peer mgmt : rsa peer mgmt should switch to RSAPeerMgmtBase
// over StriplePeerIf
//


//----------
//vote desc is a public striiple (no private key exchanged for enveloppe creation)
impl StripleImpl for VoteDesc {
  type Kind = PubSha512;
}
impl InstantiableStripleImpl for VoteDesc {
  fn init(&mut self,
    from : Vec<u8>,
    sig : Vec<u8>,
    id : Vec<u8>) {
    self.id = id;
    self.emit_by = from;
    self.sign = sig;
  }
}

impl StripleFieldsIf for VoteDesc {
  #[inline]
  fn get_algo_key(&self) -> ByteSlice {
    ByteSlice::Static(<<Self as StripleImpl>::Kind as StripleKind>::get_algo_key())
  }
  fn get_enc(&self) -> ByteSlice {
    // TODO get static value from loaded ref!!
    ByteSlice::Static(NOKEY)
  }
  fn get_id(&self) -> &[u8] {
    &self.id[..]
  }
  fn get_from(&self) -> ByteSlice {
    ByteSlice::Owned(&self.emit_by[..])
  }
  fn get_about(&self) -> ByteSlice {
    // this is null about TODO create a striple to fill it ? (from is already defining user : about
    // could be 'is an instance of' TODO change striple to allow returning &'static or & (use enum) 
    ByteSlice::Owned(&self.emit_by[..])
  }
  // TODO change striple interface to allow calculate each time
  fn get_content<'a>(&'a self) -> Option<&'a BCont<'a>> {
    self.content.as_ref()
  }

  fn get_content_ids(&self) -> Vec<&[u8]> {
    self.invitations.iter().map(|i|&i[..]).collect()
  }

  fn get_key(&self) -> &[u8] {
    // public striple signing with its own id
    &self.id[..]
  }

  fn get_sig(&self) -> &[u8] {
    &self.sign[..]
  }

}
// TODO param this later
const envelope_duration_s : i64 = 2;
// no participation impl (only synch of getting the voteconf)
const participation_duration_s : i64 = 1;
const vote_duration_s : i64 = 2;
pub fn get_new_vote_times () -> (TimeSpecExt,TimeSpecExt,TimeSpecExt) {
  let now = time::get_time();
  let e = Duration::seconds(envelope_duration_s);
  let p = Duration::seconds(participation_duration_s);
  let v = Duration::seconds(vote_duration_s);
  (
    TimeSpecExt(now + p),
    TimeSpecExt(now + p + e),
    TimeSpecExt(now + p + e + v),
  )
}
impl VoteDesc {
  pub fn new<A : KVContent,B : Address> (
    user : &StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>>,
    user_private : &[u8],
    subject : String,
    replies : Vec<String>,
    invitations : Vec<Vec<u8>>,
    ) -> MResult<Self> {
    let (t1,t2,t3) = get_new_vote_times();
    let mut vote = VoteDesc {
      shortkey : "TODO base58 of id after calc init".to_string(),
      id : Vec::new(),
      emit_by : Vec::new(),
      subject,
      replies,
      invitations,
      end_period_envelope : t2,
      end_period_participation : t1,
      end_period_vote : t3,
      sign : Vec::new(),
      content : None,
    };

    vote.init_content();
    // very wrong
    vote.calc_init(&(user,user_private)).map_err(|e|StripleMydhtErr(e))?;
    Ok(vote)
  }

  /// possible because not in striple content (better for poc and could also be better overall)
  pub fn restart_duration (&mut self) {
    let (t1,t2,t3) = get_new_vote_times();
    self.end_period_envelope = t2;
    self.end_period_participation = t1;
    self.end_period_vote = t3;
  }
  pub fn get_vote_striple_content (&self) -> VoteDescStripleContent {
    VoteDescStripleContent{
      subject  : &self.subject,
      replies : &self.replies,
    }
  }
  // TODO this is call manually , check how to integrate it to serde deserialization
  // (deserialize_with ?? or call back after struct deser?)
  pub fn init_content(&mut self) {
    let mut dest = Cursor::new(Vec::new());
    // note that we do not put date in content : the vote could therefore be reissued, what is
    // relevant is the participation report (with all enveloppe and signed by all vote peers) and
    // its associated signature
    bincode::serialize_into(&mut dest, &self.get_vote_striple_content(), bincode::Infinite).unwrap();
    self.content = Some(BCont::OwnedBytes(dest.into_inner()));
  }
}
