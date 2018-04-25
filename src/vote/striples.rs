//! striple implementation of various vote objects
use mydht::dhtimpl::{
  NoShadow,
};
use anodht::{
  AnoAddress,
};
use bincode;
use std::error::Error as ErrorTrait;
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
  PubStriple,
  ref_builder_id_copy,
  Result as StripleResult,
  Error as StripleError,
};

use striple::storage::{
  FileStripleIterator,
  init_noread_key,
};
use serde::{Serializer,Deserializer};
use serde::de::Error as SerdeDeError;
use vote::{
  VoteDesc,
  Envelope,
  Participation,
  Vote,
};
use mydht::mydhtresult::Result as MResult;
use mydht::mydhtresult::Error as MError;
use mydht::mydhtresult::ErrorKind as MErrorKind;
use std::marker::{
  PhantomData,
};
use striple::anystriple::{
  Rsa2048Sha512,
  PubSha512,
  BASE_STRIPLES,
};

use mydht_openssl::rsa_openssl::{
  RSAPeer,
  OpenSSLConf,
};

use mydht::transportif::Address;

use mydht::dhtif::{
  Peer,
  Key as KVContent,
  KeyVal,
};
use mydht::keyval::{
  SettableAttachment,
  Attachment,
};



pub struct StripleMydhtErr(pub StripleError);
impl From<StripleMydhtErr> for MError {
  #[inline]
  fn from(e : StripleMydhtErr) -> MError {
    MError((e.0).0, MErrorKind::ExternalLib, (e.0).2)
  }
}
pub struct GenErr<E : ErrorTrait>(E);
// TODO move to mydht error lib
impl<E : ErrorTrait> From<GenErr<E>> for MError {
  #[inline]
  fn from(e : GenErr<E>) -> MError {
    MError(format!("{}, cause : {:?}",e.0.description(),e.0.cause()), MErrorKind::ExternalLib, None)
  }
}



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

/*impl<A : KVContent,B : Address> StripleImpl for StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
  type Kind = Rsa2048Sha512;
}*/

//----------------------------------Peer----------------------------------------------------------

//impl<P : Peer, S : StripleKind> StripleImpl for StriplePeer<P,S> {
impl<A : KVContent,B : Address,C : OpenSSLConf, S : StripleKind> StripleImpl for StriplePeer<A,B,C,S> {
  type Kind = S;
}


impl<A : KVContent,B : Address,C : OpenSSLConf, S : StripleKind> InstantiableStripleImpl for StriplePeer<A,B,C,S> {
  fn add_from(&mut self,
    from : Vec<u8>) {
    self.from = from;
  }
  fn init(&mut self,
    sig : Vec<u8>,
    id : Vec<u8>) {
    // assume same derivation fro rsapeer and striple
    self.id = id;
    self.sig = sig;
  }
}



impl<A : KVContent,B : Address,C : OpenSSLConf, S : StripleKind> StriplePeer<A,B,C,S> {
//impl<A : KVContent,B : Address> StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
  pub fn new(p : RSAPeer<A,B,C>, secaddress : B) -> MResult<Self> {
    let mut peer = StriplePeer {
      inner : p,
      secaddress,
      id : Vec::new(),
      sig : Vec::new(),
      from : Vec::new(),
      content : None,
      _ph : PhantomData,
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


#[inline]
//fn init_content_peer<'de,D : Deserializer<'de>,A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind>(p : &mut StriplePeer<A,B,C,S>) -> Result<(),D::Error> {
//fn init_content_peer<E,A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind>(p : &mut StriplePeer<A,B,C,S>) -> Result<(),E> {
fn init_content_peer<E : SerdeDeError,A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind>(mut p : StriplePeer<A,B,C,S>) -> Result<StriplePeer<A,B,C,S>,E> {
  p.init_content();
  // check by default TODO a feature to disable this default deser check
  match p.check(&STRIPLEREFS.pub_peer) {
    Ok(true) => Ok(p),
    Ok(false) => 
      Err(SerdeDeError::custom("RSA Peer checking error, signature invalid")),
    Err(e) => 
      Err(SerdeDeError::custom(format!("{}, cause : {:?}",e.description(),e.cause()))),
  }
}

/*  pub fn init_content<A : KVContent,B : Address>(p : &mut StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>>) {
    let mut dest = Cursor::new(Vec::new());
    bincode::serialize_into(&mut dest, &p.inner.peerinfo, bincode::Infinite).unwrap();
    p.content = Some(BCont::OwnedBytes(dest.into_inner()));
  }*/


//impl<A : KVContent,B : Address, C : OpenSSLConf> StripleFieldsIf for StriplePeer<RSAPeer<A,B,C>> {
//impl<A : KVContent,B : Address> StripleFieldsIf for StriplePeer<RSAPeer<A,B,RSA2048SHA512AES256>> {
impl<A : KVContent,B : Address,C : OpenSSLConf, S : StripleKind> StripleFieldsIf for StriplePeer<A,B,C,S> {
 
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
    ByteSlice::Owned(&self.id[..])
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
#[serde(finish_deserialize = "init_content_peer")]
pub struct StriplePeer<A : KVContent,B : Address,C : OpenSSLConf, S : StripleKind> {
  pub inner : RSAPeer<A,B,C>,


  id : Vec<u8>,
  from : Vec<u8>,
  sig : Vec<u8>,
  pub secaddress : B,

  //#[serde(skip_serializing,deserialize_with="init_content")]
  #[serde(skip_serializing,skip_deserializing)]
  /// obviously wrong, striple lib need some refacto to avoid such a buffer
  /// (or allow bcont as bytes producer (meaning Read) from self)
  content : Option<BCont<'static>>,
  #[serde(skip_serializing,skip_deserializing)]
  _ph : PhantomData<S>,
}
impl<A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind> PartialEq for StriplePeer<A,B,C,S> {

  /// fast comp (fine if striple are checked)
  fn eq(&self, other: &StriplePeer<A,B,C,S>) -> bool {
    self.id == other.id
  }
}

impl<A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind> Eq for StriplePeer<A,B,C,S> {
}

impl<A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind> AnoAddress for StriplePeer<A,B,C,S> {
  type Address = B;
  fn get_sec_address (&self) -> &Self::Address {
    &self.secaddress
  }
  fn get_pri_key(&self) -> Vec<u8> {
    self.inner.get_pri_key()
  }
}

impl<A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind> Peer for StriplePeer<A,B,C,S> {
  type Address = <RSAPeer<A,B,C> as Peer>::Address;
  /// Public auth could not use default asymetric shadower of RSAPeer (need Private Auth)
  type ShadowWAuth = NoShadow;
  type ShadowRAuth = NoShadow;
  type ShadowWMsg = <RSAPeer<A,B,C> as Peer>::ShadowWMsg;
  type ShadowRMsg = <RSAPeer<A,B,C> as Peer>::ShadowRMsg;

  fn get_address (&self) -> &Self::Address {
    self.inner.get_address()
  }
  fn to_address (&self) -> Self::Address {
    self.inner.to_address()
  }
  fn get_shadower_r_auth (&self) -> Self::ShadowRAuth {
    NoShadow
  }
  fn get_shadower_r_msg (&self) -> Self::ShadowRMsg {
    self.inner.get_shadower_r_msg()
  }
  fn get_shadower_w_auth (&self) -> Self::ShadowWAuth {
    NoShadow
  }
  fn get_shadower_w_msg (&self) -> Self::ShadowWMsg {
    self.inner.get_shadower_w_msg()
  }
}
/// replace rsa id by striple id (more info in striple id)
impl<A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind> KeyVal for StriplePeer<A,B,C,S> {
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
  fn encode_kv<S1:Serializer> (&self, _s : S1, _ : bool, _ : bool) -> Result<S1::Ok, S1::Error> {
    panic!("TODO rem from trait")
  }
  /// First boolean indicates if the encoding is locally used (not send to other peers).
  /// Second boolean indicates if attachment must be added in the encoding (or just a reference
  /// kept).
  /// Default implementation decode through encode trait.
  fn decode_kv<'de,D:Deserializer<'de>> (_d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("TODO rem from trait")
  }

}
impl<A : KVContent,B : Address,C : OpenSSLConf,S : StripleKind> SettableAttachment for StriplePeer<A,B,C,S> { }
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


//------------------------VoteDesc------------------------------------



//vote desc is a public striiple (no private key exchanged for enveloppe creation)
impl StripleImpl for VoteDesc {
  type Kind = PubSha512;
}
// TODO bad design (redundant with previous Kind : at least macro this def??)
impl PubStriple for VoteDesc { }

impl InstantiableStripleImpl for VoteDesc {
  fn add_from(&mut self,
    from : Vec<u8>) {
    self.emit_by = from;
  }
  fn init(&mut self,
    sig : Vec<u8>,
    id : Vec<u8>) {
    self.id = id;
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
    ByteSlice::Owned(&self.id[..])
  }
  // TODO change striple interface to allow calculate each time
  fn get_content<'a>(&'a self) -> Option<&'a BCont<'a>> {
    self.content.as_ref()
  }

  fn get_content_ids(&self) -> Vec<&[u8]> {
    self.invitations.iter().map(|i|&i[..]).collect()
  }

  fn get_key(&self) -> &[u8] {
    // public striple signing with its own id is not a nice id, need a random key
    // &self.id[..]
    &self.key[..]
  }

  fn get_sig(&self) -> &[u8] {
    &self.sign[..]
  }

}


//------------------------Envelope------------------------------------

impl StripleImpl for Envelope {
  type Kind = Rsa2048Sha512;
}

impl StripleFieldsIf for Envelope {
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
    ByteSlice::Owned(&self.votekey[..])
  }
  fn get_about(&self) -> ByteSlice {
    // this is null about TODO create a striple to fill it ? (from is already defining user : about
    // could be 'is an instance of' TODO change striple to allow returning &'static or & (use enum) 
    ByteSlice::Owned(&self.id[..])
  }
  // TODO change striple interface to allow calculate each time
  fn get_content<'a>(&'a self) -> Option<&'a BCont<'a>> {
    self.content.as_ref()
  }

  fn get_content_ids(&self) -> Vec<&[u8]> {
    Vec::new()
  }

  fn get_key(&self) -> &[u8] {
    &self.publickey[..]
  }

  fn get_sig(&self) -> &[u8] {
    &self.sign[..]
  }

}

impl InstantiableStripleImpl for Envelope {
  fn add_from(&mut self,
    from : Vec<u8>) {
    self.votekey = from;
  }
  fn init(&mut self,
    sig : Vec<u8>,
    id : Vec<u8>) {
    self.id = id;
    self.sign = sig;
  }
}

//------------------------Participation------------------------------------
impl StripleImpl for Participation {
  // Participation public : could be private but no need at this point
  type Kind = PubSha512;
}

impl StripleFieldsIf for Participation {
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
    ByteSlice::Owned(&self.user[..])
  }
  fn get_about(&self) -> ByteSlice {
    // For now simply the vote desc id : incorrect 
    // TODO create a striple to fill it : another striple impl of the vote desc
    // to derive a vote participation striple desc : right now it is incorrect(same about use for others
    // objects types).
    ByteSlice::Owned(&self.votekey[..])
  }
  // TODO change striple interface to allow calculate each time
  fn get_content<'a>(&'a self) -> Option<&'a BCont<'a>> {
    self.content.as_ref()
  }

  fn get_content_ids(&self) -> Vec<&[u8]> {
    Vec::new()
  }

  fn get_key(&self) -> &[u8] {
    &self.pkey[..]
  }

  fn get_sig(&self) -> &[u8] {
    &self.sign[..]
  }

}

impl InstantiableStripleImpl for Participation {
  fn add_from(&mut self,
    from : Vec<u8>) {
    self.user = from;
  }
  fn init(&mut self,
    sig : Vec<u8>,
    id : Vec<u8>) {
    self.id = id;
    self.sign = sig;
  }
}

//------------------------Vote------------------------------------
impl StripleImpl for Vote {
  // Vote public : could be private but no need at this point
  type Kind = PubSha512;
}

impl StripleFieldsIf for Vote {
  #[inline]
  fn get_algo_key(&self) -> ByteSlice {
    ByteSlice::Static(<<Self as StripleImpl>::Kind as StripleKind>::get_algo_key())
  }
  fn get_enc(&self) -> ByteSlice {
    // TODO get static value from loaded ref!!
    // here it is standard string utf8
    ByteSlice::Static(NOKEY)
  }
  fn get_id(&self) -> &[u8] {
    &self.id[..]
  }
  fn get_from(&self) -> ByteSlice {
    ByteSlice::Owned(&self.envelopeid[..])
  }
  fn get_about(&self) -> ByteSlice {
    // For now simply the vote desc id : incorrect 
    // TODO create a striple to fill it : another striple impl of the vote desc
    // to derive a vote participation striple desc : right now it is incorrect(same about use for others
    // objects types).
    // Note that vote id does not require to be sign
    ByteSlice::Owned(&self.vote_id[..])
  }
  // TODO change striple interface to allow calculate each time
  fn get_content<'a>(&'a self) -> Option<&'a BCont<'a>> {
    self.content.as_ref()
  }

  fn get_content_ids(&self) -> Vec<&[u8]> {
    Vec::new()
  }

  fn get_key(&self) -> &[u8] {
    &self.pkey[..]
  }

  fn get_sig(&self) -> &[u8] {
    &self.sign[..]
  }

}

impl InstantiableStripleImpl for Vote {
  fn add_from(&mut self,
    from : Vec<u8>) {
    self.envelopeid = from;
  }
  fn init(&mut self,
    sig : Vec<u8>,
    id : Vec<u8>) {
    self.id = id;
    self.sign = sig;
  }
}


