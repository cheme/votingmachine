//! striple implementation of various vote objects


use serde::{Serializer,Deserializer};
use vote::{
  VoteDesc,
  
};
use mydht::mydhtresult::Result as MResult;
use std::marker::PhantomData;
use striple::striple::{
  StripleKind,
  StripleRef,
  AsStriple,
};
use striple::anystriple::{
  Rsa2048Sha512,
};

use mydht_openssl::rsa_openssl::{
  RSAPeer,
  RSA2048SHA512AES256,
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

impl<A : KVContent,B : Address> StriplePeerIf for RSAPeer<A,B,RSA2048SHA512AES256> {
  type Kind = Rsa2048Sha512;
}

/*
// for poc we do not load a full striple hierarchy and use dummy trusted striple (in real world the
// root only should be use) : here we cannot check : those striple must be seen as public one
const PEER_STRIPLE_ID : Vec<u8> = 1;
const VOTEDESC_STRIPLE_ID = 2;
const ENVELOPPE_STRIPLE_ID = 3;
const REL_PEER_ENV_VOTE_STRIPLE_ID = 4;
*/
pub trait StriplePeerIf : Peer {
  type Kind : StripleKind;
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(bound(deserialize = ""))]
pub struct StriplePeer<P : StriplePeerIf> {
  pub inner : P,

  striple_key : Vec<u8>,
  sig : Vec<u8>,

}

impl<P : StriplePeerIf> StriplePeer<P> {
  pub fn new(p : P) -> MResult<Self> {
    // TODO sign then key for striple
    unimplemented!()

  }
}

impl<P : StriplePeerIf> Peer for StriplePeer<P> {
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

impl<P : StriplePeerIf> KeyVal for StriplePeer<P> {
  type Key = <P as KeyVal>::Key;
  fn attachment_expected_size(&self) -> usize {
    self.inner.attachment_expected_size()
  }
  fn get_key_ref(&self) -> &Self::Key {
    self.inner.get_key_ref()
  }
  fn get_key(&self) -> Self::Key {
    self.inner.get_key()
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
impl<P : StriplePeerIf> SettableAttachment for StriplePeer<P> { }
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

// TODO Striple Peer mgmt over rsa peer mgmt : rsa peer mgmt should switch to RSAPeerMgmtBase
// over StriplePeerIf
