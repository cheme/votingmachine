use mydht::utils::TimeSpecExt;
use std::io::Cursor;
use self::striples::StripleMydhtErr;
use mydht::mydhtresult::Result as MResult;
use bincode;
use serde::{Serializer,Deserializer};
use std::borrow::Borrow;
use striple::striple::BCont;
use mydht::transportif::Address;

use striple::striple::{
  InstantiableStripleImpl,
  StripleIf,
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

/// type for any storable element
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub enum MainStoreKV {
  VoteDesc(VoteDesc),
}
pub type MainStoreKVRef = ArcRef<MainStoreKV>;
/*#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum MainStoreKVRef {
  VoteDesc(ArcRef<VoteDesc>),
}*/
/*
impl SRef for MainStoreKVRef {
  type Send = Self;
  fn get_sendable(self) -> Self::Send { self }
}
impl SToRef<MainStoreKVRef> for MainStoreKVRef {
  fn to_ref(self) -> MainStoreKVRef { self }
}
impl Borrow<MainStoreKV> for MainStoreKVRef {
  fn borrow(&self) -> &MainStoreKV {
    match *self {
      MainStoreKV::VoteDesc(rvotedesc) => rvotedesc.borrow(),
    }
  }
}
impl Ref<MainStoreKV> for MainStoreKVRef {
  fn new(t : MainStoreKV) -> Self {
    match t {
      MainStoreKV::VoteDesc(vd) => MainStoreKVRef::VoteDesc(ArcRef::new(vd)),
    }
  }
}*/
impl SettableAttachment for MainStoreKV {
  fn set_attachment(&mut self, att : &Attachment) -> bool {
    match *self {
      MainStoreKV::VoteDesc(ref mut rvotedesc) => rvotedesc.set_attachment(att),
    }
  }
}
impl SettableAttachment for VoteDesc { }

impl KeyVal for MainStoreKV {
  type Key = Vec<u8>;
  fn attachment_expected_size(&self) -> usize {
    match *self {
      MainStoreKV::VoteDesc(ref rvotedesc) => rvotedesc.attachment_expected_size(),
    }
  }
  fn get_key_ref(&self) -> &Self::Key {
    match *self {
      MainStoreKV::VoteDesc(ref rvotedesc) => rvotedesc.get_key_ref(),
    }
  }
  fn get_key(&self) -> Self::Key {
    match *self {
      MainStoreKV::VoteDesc(ref rvotedesc) => rvotedesc.get_key(),
    }
  }
  fn get_attachment(&self) -> Option<&Attachment> {
    match *self {
      MainStoreKV::VoteDesc(ref rvotedesc) => rvotedesc.get_attachment(),
    }
  }
  fn encode_kv<S:Serializer> (&self, s: S, _ : bool, _ : bool) -> Result<S::Ok, S::Error> {
    panic!("currently unused consider removal")
  }
  fn decode_kv<'de,D:Deserializer<'de>> (d : D, _ : bool, _ : bool) -> Result<Self, D::Error> {
    panic!("currently unused consider removal")
  }
}
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
  invitations : Vec<Vec<u8>>, // key to peer TODO parameterized
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

impl Eq for VoteDesc {
}


// pair key over id and vote id
pub struct Envelope {
  /// envelope id aka public key
  id : Vec<u8>,
  /// pk not sent obviously
  privateKey : Vec<u8>,
  /// vote id
  votekey : String,
  /// sign by VoteDesc privatekey
  sign : Vec<u8>,
}

pub struct Participation {
  /// participation id
  id : Vec<u8>,
  /// user id
  user : Vec<u8>,
  /// vote id
  votekey : String,
  /// vote key andenvelopes acknowledges (we include it as we may allow a bias in number of envelopes signed
  /// (convergence of every enveloppe is not easy)) - simplification to number of envelope plus
  /// votekey may be used (more realist for big).
  envelopes   : Vec<Vec<u8>>,
  /// sign of envelopes by User privatekey
  sign : Vec<u8>,
}

// key envelope id (when receiving reply envelope is no longer needed).
pub struct Reply {
  /// envelope id as key
  envelopeid : Vec<u8>,
  /// vote id (not that usefull (already in envelope))
  voteid : Vec<u8>,
  /// actual reply to vote
  reply : String,
  /// sign of reply with envelope key
  sign : Vec<u8>,
}

/// might not be send it is local info, but serialize to keep history : yes
pub struct Vote {
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
  vote : Vote,
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

