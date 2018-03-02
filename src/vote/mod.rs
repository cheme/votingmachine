use mydht::utils::TimeSpecExt;
use std::io::Cursor;
use self::striples::StripleMydhtErr;
use mydht::mydhtresult::Result as MResult;
use bincode;
use serde::{Serializer,Deserializer};
use serde::de::Error as SerdeDeError;
use std::borrow::Borrow;
use striple::striple::{
  StripleIf,
  StripleFieldsIf,
  OwnedStripleIf,
  BCont,
};
use mydht::transportif::Address;

use striple::striple::{
  InstantiableStripleImpl,
  StripleImpl,
  StripleKind,
  SignatureScheme,
};
use time::{
  self,
  Timespec,
  Duration,
};

use mydht::dhtif::{
  Peer,
  Key,
  Key as KVContent,
  KeyVal,
};

use mydht::utils::{
  Ref,
  SRef,
  SToRef,
  ArcRef,
};

use mydht::keyval::{
  SettableAttachment,
  Attachment,
};
pub mod striples;
// most of the struct ar sign with a by over something : this pattern!! About being the type of the
// struct : when serializing of signing it could be good to add this typing info keyval for KeyVal
// derivation and for sign (kind of like version (fn get_about()) encoding int use in wot).

/// type for any storable element (MainDHT)
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub enum MainStoreKV {
  VoteDesc(VoteDesc),
  Envelope(Envelope),
  Participation(Participation),
  Vote(Vote),
}
pub type MainStoreKVRef = ArcRef<MainStoreKV>;



/// type for any storable element (AnoDHT)
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub enum AnoStoreKV {
  Envelope(Envelope),
}
pub type AnoStoreKVRef = ArcRef<AnoStoreKV>;
/*
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub enum MainStoreKVRef {
  VoteDesc(ArcRef<VoteDesc>),
  Envelope(ArcRef<Envelope>),
}
pub enum MainStoreKVRef2<'a> {
  VoteDesc(&'a VoteDesc),
  Envelope(&'a Envelope),
}

impl SRef for MainStoreKVRef {
  type Send = Self;
  fn get_sendable(self) -> Self::Send { self }
}
impl SToRef<MainStoreKVRef> for MainStoreKVRef {
  fn to_ref(self) -> MainStoreKVRef { self }
}
impl Ref<MainStoreKV> for MainStoreKVRef {
  type Ref = MainstoreKVRef2<'a>;
  fn get_ref(&'a self) -> &MainStoreKVRef2<'a> {
    match *self {
      MainStoreKVRef::VoteDesc(inner) => MainStoreKVRef2::VoteDesc(inner.borrow()),
      MainStoreKVRef::Envelope(inner) => MainStoreKVRef2::Envelope(inner.borrow()),
    }
  }

  fn new(t : MainStoreKV) -> Self {
    match t {
      MainStoreKV::VoteDesc(inner) => MainStoreKVRef::VoteDesc(ArcRef::new(inner)),
      MainStoreKV::Envelope(inner) => MainStoreKVRef::Envelope(ArcRef::new(inner)),
    }
  }
}
*/
//--------------------MainStoreKV---------------------------
impl SettableAttachment for MainStoreKV {
  fn set_attachment(&mut self, att : &Attachment) -> bool {
    match *self {
      MainStoreKV::VoteDesc(ref mut inner) => inner.set_attachment(att),
      MainStoreKV::Envelope(ref mut inner) => inner.set_attachment(att),
      MainStoreKV::Participation(ref mut inner) => inner.set_attachment(att),
      MainStoreKV::Vote(ref mut inner) => inner.set_attachment(att),
    }
  }
}

impl KeyVal for MainStoreKV {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize {
    match *self {
      MainStoreKV::VoteDesc(ref inner) => inner.attachment_expected_size(),
      MainStoreKV::Envelope(ref inner) => inner.attachment_expected_size(),
      MainStoreKV::Participation(ref inner) => inner.attachment_expected_size(),
      MainStoreKV::Vote(ref inner) => inner.attachment_expected_size(),
    }
  }
  fn get_key_ref(&self) -> &Self::Key {
    match *self {
      MainStoreKV::VoteDesc(ref inner) => inner.get_key_ref(),
      MainStoreKV::Envelope(ref inner) => inner.get_key_ref(),
      MainStoreKV::Participation(ref inner) => inner.get_key_ref(),
      MainStoreKV::Vote(ref inner) => inner.get_key_ref(),
    }
  }
  fn get_key(&self) -> Self::Key {
    match *self {
      MainStoreKV::VoteDesc(ref inner) => KeyVal::get_key(inner),
      MainStoreKV::Envelope(ref inner) => KeyVal::get_key(inner),
      MainStoreKV::Participation(ref inner) => KeyVal::get_key(inner),
      MainStoreKV::Vote(ref inner) => KeyVal::get_key(inner),
    }
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    match *self {
      MainStoreKV::VoteDesc(ref inner) => inner.get_attachment(),
      MainStoreKV::Envelope(ref inner) => inner.get_attachment(),
      MainStoreKV::Participation(ref inner) => inner.get_attachment(),
      MainStoreKV::Vote(ref inner) => inner.get_attachment(),
    }
  }
  fn encode_kv<S:Serializer> (&self, s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}

impl MainStoreKV {
  pub fn get_votedesc(&self) -> Option<&VoteDesc> {
    if let MainStoreKV::VoteDesc(ref inner) = *self {
      Some(inner)
    } else {
      None
    }
  }
}
//----------------------------AnoStoreKV---------------------
impl SettableAttachment for AnoStoreKV {
  fn set_attachment(&mut self, att : &Attachment) -> bool {
    match *self {
      AnoStoreKV::Envelope(ref mut inner) => inner.set_attachment(att),
    }
  }
}

impl KeyVal for AnoStoreKV {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize {
    match *self {
      AnoStoreKV::Envelope(ref inner) => inner.attachment_expected_size(),
    }
  }
  fn get_key_ref(&self) -> &Self::Key {
    match *self {
      AnoStoreKV::Envelope(ref inner) => inner.get_key_ref(),
    }
  }
  fn get_key(&self) -> Self::Key {
    match *self {
      AnoStoreKV::Envelope(ref inner) => KeyVal::get_key(inner),
    }
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    match *self {
      AnoStoreKV::Envelope(ref inner) => inner.get_attachment(),
    }
  }
  fn encode_kv<S:Serializer> (&self, s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}


//---------------------------VoteDesc--------------------------------


impl SettableAttachment for VoteDesc { }
impl KeyVal for VoteDesc {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize { 0 }
  fn get_key_ref(&self) -> &Self::Key {
    &self.id
  }
  fn get_key(&self) -> Self::Key {
    self.id.clone()
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    None
  }
  fn encode_kv<S:Serializer> (&self, s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}


#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(finish_deserialize = "init_content_votedesc")]
/// structure representing a vote with its associated information
/// TODO participant is to limiting it could be extended to "wot group" 
/// aka web of trust level or groups (especially for invitations).
/// VoteDesc access is restricted until Vote as been validated.
pub struct VoteDesc {
  /// id to query for vote TODO use something like bitcoin address
  /// TODO init from base58 of id
  pub shortkey : String,
  /// TODO another keyval to associate with this id - it is a publickey
  id : Vec<u8>,
  key : Vec<u8>,
  /// creator of the vote (a user)
  emit_by : Vec<u8>,
/*  /// it is this information that should not be published outside invitation group
  /// this is restricted
  privateKey : Vec<u8>,*/
  /// could be url to description or a lot of othe file TODO expand
  pub subject  : String,
  /// possible replies, could also be open replies and other types TODO expand
  pub replies  : Vec<String>,
  /// invitations TODO evolve to Open (every body join and of course multiple times : it is just
  /// some open poll : can add ip filter...), Wot (in this case it is probable we got various vote
  /// versions).
  pub invitations : Vec<Vec<u8>>, // key to peer TODO parameterized
  /// Deadline for participating, after this peers will not include participant in vote
  /// TODO link to coin chain to avoid fraud (must be optional).
  end_period_envelope : TimeSpecExt,
  end_period_participation : TimeSpecExt,
  end_period_vote : TimeSpecExt,
  /// sign of its info
  sign : Vec<u8>,
  #[serde(skip_serializing,skip_deserializing)]
  /// obviously wrong, striple lib need some refacto to avoid such a buffer
  /// (or allow bcont as bytes producer (meaning Read) from self)
  content : Option<BCont<'static>>,
}

impl VoteDesc {
  #[inline]
  pub fn nb_invit(&self) -> usize {
    self.invitations.len()
  }
}
#[inline]
fn init_content_votedesc<E>(mut vote : VoteDesc) -> Result<VoteDesc,E> {
  vote.init_content();
  Ok(vote)
  // could not check as signature is not static (might require to search for peer)
}



#[derive(Debug,Serialize)]
/// structure representing a vote with its associated information
/// TODO participant is to limiting it could be extended to "wot group" 
/// aka web of trust level or groups (especially for invitations).
/// VoteDesc access is restricted until Vote as been validated.
pub struct VoteDescStripleContent<'a> {
  pub subject  : &'a String,
  pub replies  : &'a Vec<String>,
}
 

impl PartialEq for VoteDesc {
  /// fast comp (fine if striple are checked)
  fn eq(&self, other: &VoteDesc) -> bool {
    self.id == other.id
  }
}

impl Eq for VoteDesc { }

//---------------------------Envelope--------------------------------

#[derive(Debug,Serialize,Deserialize,Clone)]
/// pair key over id and vote id
pub struct Envelope {
  /// envelope id aka derived public key
  id : Vec<u8>,
  publickey : Vec<u8>,
  /// vote id
  pub votekey : Vec<u8>,
  /// sign by VoteDesc privatekey
  sign : Vec<u8>,
}


impl PartialEq for Envelope {
  /// fast comp (fine if striple are checked)
  fn eq(&self, other: &Envelope) -> bool {
    self.id == other.id
  }
}

impl Eq for Envelope { }

impl SettableAttachment for Envelope { }

impl KeyVal for Envelope {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize { 0 }
  fn get_key_ref(&self) -> &Self::Key {
    &self.id
  }
  fn get_key(&self) -> Self::Key {
    self.id.clone()
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    None
  }
  fn encode_kv<S:Serializer> (&self, s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}

impl Envelope {
  pub fn new (
    vote_desc : &VoteDesc,
    ) -> MResult<(Self,Vec<u8>)> {

    let (vkey,pkey) = <<<Envelope as StripleImpl>::Kind as StripleKind>::S as SignatureScheme>::new_keypair().map_err(|e|StripleMydhtErr(e))?;

    let mut envelope = Envelope {
      id: Vec::new(),
      publickey: vkey, 
      votekey: Vec::new(), // init in calc_init
      sign: Vec::new(),
    };

    envelope.calc_init(vote_desc).map_err(|e|StripleMydhtErr(e))?;
    Ok((envelope,pkey))
  }
 
}
//---------------------- Participation
#[derive(Debug,Serialize,Deserialize,Clone)]
#[serde(finish_deserialize = "init_content_participation")]
pub struct Participation {
  /// participation id
  id : Vec<u8>,
  /// pub key : striple technical
  pkey : Vec<u8>,
  /// user id
  pub user : Vec<u8>,
  /// vote id
  pub votekey : Vec<u8>,
  /// vote key andenvelopes acknowledges (we include it as we may allow a bias in number of envelopes signed
  /// (convergence of every enveloppe is not easy)) - simplification to number of envelope plus
  /// votekey may be used (more realist for big).
  /// TODO a merkle tree hash for scaling
  envelopes : Vec<Vec<u8>>,
  /// tells if all is fine until now
  pub is_valid : bool,
  #[serde(skip_serializing,skip_deserializing)]
  /// obviously wrong, striple lib need some refacto to avoid such a buffer
  /// (or allow bcont as bytes producer (meaning Read) from self)
  content : Option<BCont<'static>>,
  /// sign of envelopes by User privatekey
  sign : Vec<u8>,
}
fn init_content_participation<E : SerdeDeError>(mut p : Participation) -> Result<Participation,E> {

  // should use bincode encoding (depends of from definition)
  let v : u8 = if p.is_valid { 1 } else { 0 };
  p.content = Some(BCont::OwnedBytes(vec![v]));
  Ok(p)
  // no check : would require a reference to user TODO consider doing it : would require a
  // parameter for deserializing (vote store reference).
}

impl PartialEq for Participation {
  /// fast comp (fine if striple are checked)
  fn eq(&self, other: &Participation) -> bool {
    self.id == other.id
  }
}

impl Eq for Participation { }

impl SettableAttachment for Participation { }

impl KeyVal for Participation {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize { 0 }
  fn get_key_ref(&self) -> &Self::Key {
    &self.id
  }
  fn get_key(&self) -> Self::Key {
    self.id.clone()
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    None
  }
  fn encode_kv<S:Serializer> (&self, _s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (_d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}

impl Participation {
  pub fn new<P : OwnedStripleIf> (
    user : &P,
    vote_desc : &VoteDesc,
    envelopes : &[(Envelope,bool)],
    is_valid : bool) -> MResult<Self> {

    let envelopes = envelopes.iter().map(|e|e.0.get_id().to_vec()).collect();

    let (pkey,_) = <<<Participation as StripleImpl>::Kind as StripleKind>::S as SignatureScheme>::new_keypair().map_err(|e|StripleMydhtErr(e))?;
    let mut participation = Participation {
      id : Vec::new(),
      pkey,
      content : None,
      sign : Vec::new(),
      user : Vec::new(), // init in calc_init
      votekey : vote_desc.get_id().to_vec(),
      envelopes,
      is_valid,
    };
    participation.calc_init(user).map_err(|e|StripleMydhtErr(e))?;
    Ok(participation)
  }
}




//------------------------ Vote

#[derive(Debug,Serialize,Deserialize,Clone)]
#[serde(finish_deserialize = "init_content_vote")]
pub struct Vote {

  /// id
  id : Vec<u8>,
  /// envelope id as key
  pub envelopeid : Vec<u8>,
  /// vote id (not that usefull (already in envelope))
  pub vote_id : Vec<u8>,
  /// actual reply to vote
  reply : String,
  pkey : Vec<u8>,
  /// sign of reply with envelope key
  sign : Vec<u8>,
  #[serde(skip_serializing,skip_deserializing)]
  /// obviously wrong, striple lib need some refacto to avoid such a buffer
  /// (or allow bcont as bytes producer (meaning Read) from self)
  content : Option<BCont<'static>>,
 
}
fn init_content_vote<E : SerdeDeError>(mut p : Vote) -> Result<Vote,E> {

  p.content = Some(BCont::OwnedBytes(p.reply.clone().into_bytes()));
  Ok(p)
  // no check : would require a reference to envelope TODO consider doing it : would require a
  // parameter for deserializing (vote store reference).
}

impl PartialEq for Vote {
  /// fast comp (fine if striple are checked)
  fn eq(&self, other: &Vote) -> bool {
    self.id == other.id
  }
}

impl Eq for Vote { }

impl SettableAttachment for Vote { }

impl KeyVal for Vote {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize { 0 }
  fn get_key_ref(&self) -> &Self::Key {
    &self.id
  }
  fn get_key(&self) -> Self::Key {
    self.id.clone()
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    None
  }
  fn encode_kv<S:Serializer> (&self, _s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (_d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}

impl Vote {
  pub fn new (
    owned_envelope : &(&Envelope,&[u8]),
    vote_desc : &VoteDesc,
    value : String) -> MResult<Self> {

    let (pkey,_) = <<<Vote as StripleImpl>::Kind as StripleKind>::S as SignatureScheme>::new_keypair().map_err(|e|StripleMydhtErr(e))?;
    let mut vote = Vote {
      id : Vec::new(),
      envelopeid : Vec::new(),
      content : None,
      pkey,
      sign : Vec::new(),
      vote_id : vote_desc.get_id().to_vec(),
      reply : value,
    };
    vote.calc_init(owned_envelope).map_err(|e|StripleMydhtErr(e))?;
    Ok(vote)
  }
}



//------------------------
/// might not be send it is local info, but serialize to keep history : yes
/// TODO useless : correspond to VoteDesc but at final state
pub struct VoteEnd {
  /// id to query for vote TODO use something like bitcoin address
  shortkey : String,
  /// original desc TODO do not serialize it or send it
  votedesc : VoteDesc,
  /// participations
  participant : Vec<Vec<u8>>, // key to peer TODO parameterized
  /// all published participation query before end_period_accept
  envelopes   : Vec<Vec<u8>>, // key to peer TODO parameterized
  /// all replies
  replies     : Vec<Vec<u8>>,
}
// TODO implement KeyVal over shortkey


// Note that subvote creation must be deterministic for every peer (except private key), so that
// we know wich used even after vote period
pub struct SubVoteDesc ;


// TODO those derive from a vote and run as a single vote but we need rules to link both and to
// avoid misuse leading to loss of anonymate
pub struct SubVote {
  /// original vote TODO do not serialize it
  vote : VoteEnd,
  subparticipant : Vec<Vec<u8>>, // key to peer TODO parameterized
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
  pub fn new<C : StripleIf> (
    user : &C,
    user_private : &[u8],
    subject : String,
    replies : Vec<String>,
    invitations : Vec<Vec<u8>>,
    ) -> MResult<Self> {
    let (t1,t2,t3) = get_new_vote_times();
    // warning we implies it is a public scheme here
    let (vkey,_) = <<<VoteDesc as StripleImpl>::Kind as StripleKind>::S as SignatureScheme>::new_keypair().map_err(|e|StripleMydhtErr(e))?;
    let mut vote = VoteDesc {
      shortkey : "TODO base58 of id after calc init".to_string(),
      id : Vec::new(),
      key : vkey,
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


