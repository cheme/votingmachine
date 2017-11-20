use mydht::utils::TimeSpecExt;

// most of the struct ar sign with a by over something : this pattern!! About being the type of the
// struct : when serializing of signing it could be good to add this typing info keyval for KeyVal
// derivation and for sign (kind of like version (fn get_about()) encoding int use in wot).

/// structure representing a vote with its associated information
/// TODO participant is to limiting it could be extended to "wot group" 
/// aka web of trust level or groups (especially for invitations).
/// VoteDesc access is restricted until Vote as been validated.
pub struct VoteDesc {
  /// id to query for vote TODO use something like bitcoin address
  shortkey : String,
  /// TODO another keyval to associate with this id - it is a publickey
  id : Vec<u8>,
  /// it is this information that should not be published outside invitation group
  /// this is restricted
  privateKey : Vec<u8>,
  /// could be url to description or a lot of othe file TODO expand
  subject  : String,
  /// possible replies, could also be open replies and other types TODO expand
  replies  : Vec<String>,
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
