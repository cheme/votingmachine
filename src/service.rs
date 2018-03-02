//! main service (in the main dht) for internal logic (synch of content ...),
//! with inner standerad kvstore service
use mydht_tunnel::{
  MyDHTTunnelConf,
  MyDHTTunnelConfType,
  GlobalTunnelReply,
  SSWCache,
  SSRCache,
  GlobalTunnelCommand,
};
use striple::striple::{
  StripleIf,
  OwnedStripleIf,
  StripleFieldsIf,
};
use vote::striples::{
  StripleMydhtErr,
};
// TODO replace by a better tree structure (lot of to_vec to use this...)
// or try use BTreeMap<&[u8],V>
use std::collections::BTreeMap;

use mydht::keyval::{
  KeyVal,
};
use anodht::{
  AnoServiceICommand,
  StoreAnoMsg,
  AnoAddress,
};
use std::hash::Hash;
use anodht::{
  AnoDHTConf,
  AnoTunDHTConf2,
};
 
use mydht::api::{
  DHTIn,
};
use maindht::{
  MainDHTConf,
  MainKVStoreCommand,
};



use mydht::dhtimpl::{
  SimpleRules,
};
use mydht::mydhtresult::{
  Result,
  Error,
  ErrorKind,
};
use mydht::peer::{
  Peer,
  PeerMgmtMeths,
};
use mydht::utils::{
  ArcRef,
  Ref,
  SerSocketAddr,
};
use mydht::service::{
  Service,
  SpawnerYield,
  SpawnSend,
};
use mydht::storeprop::{
  KVStoreProtoMsgWithPeer,
  KVStoreCommand,
  KVStoreReply,
  KVStoreService,
  send_user_to_global,
};
use vote::{
  VoteDesc,
  Envelope,
  Participation,
  Vote,
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
  ApiCommand,
  FWConf,
};

pub struct VotingService<P : Peer<Key = Vec<u8>, Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,RP,PM : PeerMgmtMeths<P>> 
  {
  pub store_service : KVStoreService<
    P,
    RP,
    MainStoreKV,
    MainStoreKVRef,
    MainStoreKVStore,
    SimpleRules,
    MainStoreQueryCache<P,RP>
  >,
  pub ano_dhtin : DHTIn<MyDHTTunnelConfType<AnoTunDHTConf2<P,PM>>>,
  pub votes : BTreeMap<Vec<u8>,VoteContext<RP>>,
  pub waiting_user : BTreeMap<Vec<u8>,Vec<(bool,MainStoreKVRef)>>,
  pub me_sign_key : Vec<u8>,
}
// TODO change mydht error to contain static &[u8] and list of objects to format!!!
//const no_vote_context : Error = Error("no vote context".to_string(),ErrorKind::ExpectedError,None);
#[inline]
fn no_vote_context() -> Error {
  Error("no vote context".to_string(),ErrorKind::ExpectedError,None)
}
#[inline]
fn no_envelope_context() -> Error {
  Error("no envolope context".to_string(),ErrorKind::ExpectedError,None)
}


pub struct VoteContext<RP> {
  pub vote_desc : VoteDesc,
  pub my_reply : String,
  pub my_envelope : Envelope,
  pub my_envelope_priv_key : Vec<u8>,
  pub envelopes : Vec<(Envelope,bool)>,
  pub my_participation : Option<Participation>,
  pub my_vote : Option<Vote>,
  pub votes : Vec<Vote>,
  pub votant_ctx : BTreeMap<Vec<u8>, UserContext<RP>>,
  pub participation_ok : usize,
  pub participation_ko : usize,
}

pub struct UserContext<RP> {
  pub user : RP,
  pub participation : Option<Participation>,
}

use std::borrow::Borrow;

impl<P : Peer<Key = Vec<u8>, Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,PM : PeerMgmtMeths<P>> VotingService<P,ArcRef<P>,PM>
  {

  fn broadcast_msg (me_key : &<P as KeyVal>::Key, vote_desc : &VoteDesc, val : MainStoreKVRef) -> Result<<Self as Service>::CommandOut> {
    let mut dests = Vec::with_capacity(vote_desc.nb_invit());
    for destk in vote_desc.invitations.iter().filter(|k|&k[..] != &me_key[..])  {
      // TODO would be way better with peer ref
      dests.push((Some(destk.clone()),None))
    }
    return Ok(
      GlobalReply::Forward(
        None,
        Some(dests),
        FWConf {
          nb_for : 0,
          discover : true,
        },
        MainKVStoreCommand::Store(
          KVStoreCommand::Store(0,[val].to_vec()))
    ));
  }

  /// filter for validation -> TODO transform to main method and include storage
  fn vote_impl(&mut self, kv: &MainStoreKVRef, is_local : bool) -> Result<Option<<Self as Service>::CommandOut>> {
    if is_local {
    match *kv.borrow() {
      MainStoreKV::VoteDesc(ref _votedesc) => {
      },
      MainStoreKV::Envelope(ref envelope) => {
        // TODO manage envelope list and probably store it
        println!("--------------------> Env store reach");
        let mut context = self.votes.get_mut(&envelope.votekey).ok_or_else(||no_vote_context())?;
        let valid_env = envelope.check(&context.vote_desc).map_err(|e|StripleMydhtErr(e))?;
        if valid_env {
          println!("an anonymous valid envelop : {:?}", envelope.get_id());
        } else {
          println!("an anonymous invalid envelop");
          return Ok(Some(GlobalReply::NoRep));
        }
        context.envelopes.push((envelope.clone(),false));
        // query all enveloppe of anonymous dht : no query currently (add timer to do it). But
        // plain and simple broadcast (TODO allow kvstore to broadcast/query search).

        let me_key = {
          let mb : &P = self.store_service.me.borrow();
          mb.get_key_ref()
        };
        return Ok(Some(Self::broadcast_msg(me_key, &context.vote_desc, 
                ArcRef::new(MainStoreKV::Envelope(envelope.clone())))?));
      },
      MainStoreKV::Participation(ref _participation) => {
        unimplemented!("most likely nothing todo");
      },
      MainStoreKV::Vote(ref vote) => {
        println!("--------------------> Vote store reach");
        // received a vote from anodht
        let context = match self.votes.get_mut(&vote.vote_id) {
          Some(context) => {
            context
          },
          None => {
            // we consider it valid as we may be receiving ano vote for a vote we do not know
            // yet we need to query the votedesc to be able to broadcast, which is unimplemented
            unimplemented!()
          },
        };
        let env_pos = context.envelopes.iter().position(|e2|e2.0.get_id() == &vote.envelopeid[..])
          .ok_or_else(||no_envelope_context())?;
        if context.envelopes[env_pos].1 {
          // already here
          if context.votes.contains(vote) {
            // got vote twice, no need to broadcast
            return Ok(Some(GlobalReply::NoRep));
          } else {
            warn!("receive a vote for envelope two time, skipping");
            // TODO check if valid and mark envelope as wrong (different from non valid : means a
            // participant cheat : not from outside)
            return Ok(Some(GlobalReply::NoRep));
          }
        }
        let valid_vote = context.envelopes[env_pos].0.check(vote).map_err(|e|StripleMydhtErr(e))?;
        if valid_vote {
          context.envelopes[env_pos].1 = true;
          context.votes.push(vote.clone());
        } else {
          println!("an anonymous invalid vote");
          return Ok(Some(GlobalReply::NoRep));
        }
        let me_key = {
          let mb : &P = self.store_service.me.borrow();
          mb.get_key_ref()
        };
        return Ok(Some(Self::broadcast_msg(me_key, &context.vote_desc, 
                ArcRef::new(MainStoreKV::Vote(vote.clone())))?));


      },

    }
    } else {
    // distant
    match *kv.borrow() {
      MainStoreKV::VoteDesc(ref votedesc) => {
        unimplemented!()
      },
      MainStoreKV::Envelope(ref envelope) => {
        match self.votes.get_mut(&envelope.votekey) {
          Some(ref mut context) => {
            let all_env = if envelope.check(&context.vote_desc).map_err(|e|StripleMydhtErr(e))? {
              if !context.envelopes.iter().any(|e2|&e2.0 == envelope) {
                // TODO store a ref (requires refacton mainstorkv ref or store mainstorekv)
                context.envelopes.push((envelope.clone(),false));
                if context.envelopes.len() == context.vote_desc.invitations.len() {
                  // TODO end timer unused
                  true
                } else { false }
              } else { false }
            } else {
              error!("Received an invalid envelop (wrong striple signing), dropping it");
              println!("Received an invalid envelop (wrong striple signing), dropping it");
              false
            };
            if all_env {
              // at that point we got n envelopes checked, we still need to ensure our envelope is
              // present TODO end timer validation : we can check if there is to much envelope
              // (checking everyone is on the same envelope pool afterward is still interesting)
              let valid = context.envelopes.iter().any(|e2|e2.0 == context.my_envelope);
              
              // create participation (sign by our peer striple)
              let participation = {
                let mb : &P = self.store_service.me.borrow();
                Participation::new(
                &(mb, &self.me_sign_key[..]),
                &context.vote_desc,
                &context.envelopes,
                valid
                )?
              };
              // TODO remove : only for testing
              {
                let mb : &P = self.store_service.me.borrow();
                assert!(mb.check(&participation).map_err(|e|StripleMydhtErr(e))?);
              };

              // TODO refacto MainStoreKVRef to store a participation ref instead, probably need https://github.com/rust-lang/rust/pull/46706
              let send_part = ArcRef::new(MainStoreKV::Participation(participation.clone()));
              context.my_participation = Some(participation.clone());
              {
                // TODO move in a votant context function
                let vc = UserContext {
                  user : self.store_service.me.clone(),
                  participation : Some(participation),
                };
                let me_id = {
                  let mb : &P = self.store_service.me.borrow();
                  mb.get_id().to_vec()
                };

                context.votant_ctx.insert(me_id, vc);
                if valid {
                  context.participation_ok += 1;
                } else {
                  context.participation_ko += 1;
                }
              }
              // share participation (store + query all)
              let me_key = {
                let mb : &P = self.store_service.me.borrow();
                mb.get_key_ref()
              };
              return Ok(Some(Self::broadcast_msg(me_key, &context.vote_desc, send_part)?));
    
            }

          }
          None => {
            println!(concat!("receive envelope {:?}, but no vote, dropping it: ",
            "TODO a temp storage in case we do not aknowledge vote distribution,",
            "currently vote distribution is out of POC"), envelope.get_id());
          }
        }
      },
      MainStoreKV::Participation(ref participation) => {
        // TODO (not in poc) public synchro of everyone validating participation (in POC panic peer if
        // invalid)
        match self.votes.get_mut(&participation.votekey) {
          Some(ref mut context) => {
            let from = match context.votant_ctx.get_mut(&participation.user) {
              Some(f) => f,
              None => {
                let k = participation.user.clone();
                let ins = if let Some(ref mut st) = self.waiting_user.get_mut(&k) {
                  st.push((is_local,kv.clone()));
                  true
                } else { false };
                if !ins {
                  let mut st = vec![(is_local,kv.clone())];
                  self.waiting_user.insert(k,st);
                }
                // user query with callback!!
                let query_user = GlobalReply::PeerStore(KVStoreCommand::WithLocalValue(participation.user.clone(), send_user_to_global));
 
                return Ok(Some(query_user));
              },
            };
            let checked_p = {
              let fp : &P = from.user.borrow();
              participation.check(fp).map_err(|e|StripleMydhtErr(e))?
            };
            if checked_p {
              // TODO refact ref (see others comments)
              let add = match from.participation.as_ref() {
                Some(p) => {
                  if p != participation || p.is_valid != participation.is_valid {
                    // TODO broadcast invalid kv with reason
                    panic!("A user signed two different participation");
                  }
                  false
                },
                None => {
                  true
                },
              };
              if add {
                from.participation = Some(participation.clone());
                if participation.is_valid {
                  context.participation_ok += 1;
                  if context.participation_ok == context.vote_desc.invitations.len() {
       

                    // make vote (sign by enveloppe, about votedesc)
                    let vote = Vote::new(&(&context.my_envelope,&context.my_envelope_priv_key), &context.vote_desc, context.my_reply.clone())?;
                    // TODO remove : only for testing
                    assert!(context.my_envelope.check(&vote).map_err(|e|StripleMydhtErr(e))?);
         
                    // keep trace not really usefull for poc
                    context.my_vote = Some(vote.clone());
                    // share votes (store + query all) in anonymous dht
                    let c_store_vote = GlobalTunnelCommand::Inner(AnoServiceICommand(StoreAnoMsg::STORE_VOTE(vote)));
                    let command = ApiCommand::call_service(c_store_vote);
                    self.ano_dhtin.send(command)?;

                  }
                } else {
                  // TODO should not panic
                  panic!("A user do not acknowledge its participation");
                  context.participation_ko += 1;
                }
              }
            } else {
              println!("receive invalid participation {:?}, but no vote, dropping it", participation.get_id());
            };
          },
          None => {
            println!("receive participation {:?}, but no vote, dropping it", participation.get_id());
          },
        }
      },
      MainStoreKV::Vote(ref vote) => {
        // received a vote from broadcast
        
        let mut context = self.votes.get_mut(&vote.vote_id).ok_or_else(||no_vote_context())?;
        let env_pos = context.envelopes.iter().position(|e2|e2.0.get_id() == &vote.envelopeid[..])
          .ok_or_else(||no_envelope_context())?;
        if context.envelopes[env_pos].1 {
          // already here
          if context.votes.contains(vote) {
            // got vote twice, maybe broadcasted twice
            return Ok(Some(GlobalReply::NoRep));
          } else {
            warn!("receive a broadcasted vote for envelope two time, skipping");
            // TODO check if valid and mark envelope as wrong (different from non valid : means a
            // participant cheat : not from outside)
            return Ok(Some(GlobalReply::NoRep));
          }
        }
        let valid_vote = context.envelopes[env_pos].0.check(vote).map_err(|e|StripleMydhtErr(e))?;
        if valid_vote {
          context.envelopes[env_pos].1 = true;
          context.votes.push(vote.clone());
        } else {
          println!("an anonymous invalid vote");
          return Ok(Some(GlobalReply::NoRep));
        }

        if context.votes.len() == context.vote_desc.invitations.len() {

          let valid = if context.my_vote.is_some() && context.votes.contains(context.my_vote.as_ref().unwrap()) { 

            // print global vote result
            println!("Find my vote and received all votes!!!!!");

            true
          } else {
            println!("Invalid vote, missing my vote");
            false
          };

          // TODO make result (valid my vote and nb vote) : sign by user
          
          // TODO share results (store + query all)

          // TODO store everything 

        } else if context.votes.len() > context.vote_desc.invitations.len() {
          unreachable!()
        }

      },
    }
    }
    Ok(None)
  }

}

impl<P : Peer<Key = Vec<u8>, Address = SerSocketAddr> + AnoAddress<Address = SerSocketAddr>,PM : PeerMgmtMeths<P>> Service for VotingService<P,ArcRef<P>,PM>
  {
 
  //KVStoreService<P,RP,MainStoreKV,MainStoreKVRef,MainStoreKVStore,SimpleRules,MainStoreQueryCache<P,RP>> {
  type CommandIn = GlobalCommand<ArcRef<P>,MainKVStoreCommand<P>>;
  type CommandOut = GlobalReply<P,ArcRef<P>,MainKVStoreCommand<P>,KVStoreReply<MainStoreKVRef>>;

  fn call<Y : SpawnerYield>(&mut self, req: Self::CommandIn, async_yield : &mut Y) -> Result<Self::CommandOut> {

    let is_local = req.is_local(); 
    // filters :
    match req.get_inner_command() {
      &MainKVStoreCommand::Store(KVStoreCommand::Store(_,ref vals)) => for v in vals.iter() {
        if let Some(r) = self.vote_impl(v,is_local)? {
          return Ok(r);
        }
      },
      &MainKVStoreCommand::Store(KVStoreCommand::StoreLocally(ref v,..)) => if let Some(r) = self.vote_impl(v,is_local)? {
        return Ok(r);
      },
      _ => (),
    }
    // TODO switch to map and move some code from vote_impl
    let command_out =
    if let GlobalCommand::Local(MainKVStoreCommand::Vote(vote_desc,my_reply)) = req {
       // keep localy envelope with pk (pk not serialized through serde so vec null is send).
       let (envelope,my_envelope_priv_key) = Envelope::new(&vote_desc)?;
       let votedesc_id = vote_desc.get_id().to_vec();
       let nb_invit = vote_desc.nb_invit();
       let context = VoteContext {
         vote_desc : vote_desc,
         my_envelope : envelope.clone(),
         my_envelope_priv_key,
         my_reply,
         envelopes : Vec::with_capacity(nb_invit),
         votes : Vec::with_capacity(nb_invit),
         my_participation : None,
         my_vote : None,
         votant_ctx : BTreeMap::new(),
         participation_ok : 0,
         participation_ko : 0,
       };
       self.votes.insert(votedesc_id, context);
       println!("initialized envolope");

       //  assert!(envelope.check(votedesc).unwrap()==true); useless check except for debuging purpose)

       // store enveloppe with pk : not in POC (use this object for next steps no persistence)

      // share enveloppe anonymously (store + query all)
      //TODO run with reply ?? or do another store after a triggered timer (required to create
      //mainloop timer (cf already needed to maintain pool)
      let c_store_env = GlobalTunnelCommand::Inner(AnoServiceICommand(StoreAnoMsg::STORE_ENVELOPE(envelope)));
      let command = ApiCommand::call_service(c_store_env);
      self.ano_dhtin.send(command)?;


      // TODO call self with a local kvstore
      GlobalReply::NoRep
    } else if let GlobalCommand::Local(MainKVStoreCommand::PeerReply(op)) = req {
      match op {
        Some(peer) => {
          
          let k = {
            let mp : &P = peer.borrow();
            mp.get_id().clone()
          };
          match self.waiting_user.remove(k) {
            Some(commands) => {
              let mut res = Vec::new();
              for (is_local,command) in commands.into_iter() {
                if let Some(r) = self.vote_impl(&command,is_local)? {
                  res.push(r)
                }
              }
              return Ok(GlobalReply::Mult(res));
            },
            None => {
              warn!("Query user reply but no content")
            },
          }
        },
        None => {
          unimplemented!("TODO try connect or search peer -> requires to pass key and change if")
        },
      };
      GlobalReply::NoRep
    // query votedesc : if no participation set for votedesc : reply only if user in list of voters -> create a command for it that use owith in command TODO evo MyDHT to add owith to local or global commad
    // for a start simply reply?? (key being the secret)
    } else if let GlobalCommand::Local(MainKVStoreCommand::Store(req)) = req {
      self.store_service.call(GlobalCommand::Local(req),async_yield)?
    } else if let GlobalCommand::Distant(a,MainKVStoreCommand::Store(req)) = req {
      self.store_service.call(GlobalCommand::Distant(a,req),async_yield)?
    } else {
      GlobalReply::NoRep
    };
/*    match command_out {
      GlobalReply::Api(KVStoreReply::FoundApi(Some(ref val),_)) => (),
      GlobalReply::Api(KVStoreReply::FoundApiMult(ref vals,_)) => (),
      _ => (),
    }*/
    Ok(to_main_reply(command_out))
  }
}

fn to_main_reply<P : Peer>(rep : GlobalReply<P,ArcRef<P>,KVStoreCommand<P,ArcRef<P>,MainStoreKV,MainStoreKVRef>,KVStoreReply<MainStoreKVRef>>)
  -> GlobalReply<P,ArcRef<P>,MainKVStoreCommand<P>,KVStoreReply<MainStoreKVRef>> {
  match rep {
    GlobalReply::Forward(a,b,c,kvc) => GlobalReply::Forward(a,b,c,MainKVStoreCommand::Store(kvc)),
    GlobalReply::ForwardOnce(a,b,c,kvc) => GlobalReply::ForwardOnce(a,b,c,MainKVStoreCommand::Store(kvc)),
    GlobalReply::PeerForward(a,b,c,d) => GlobalReply::PeerForward(a,b,c,d),
    GlobalReply::Api(a) => GlobalReply::Api(a),
    GlobalReply::PeerApi(a) => GlobalReply::PeerApi(a),
    GlobalReply::MainLoop(a) => GlobalReply::MainLoop(a),
    GlobalReply::PeerStore(a) => GlobalReply::PeerStore(a),
    GlobalReply::NoRep => GlobalReply::NoRep,
    GlobalReply::Mult(m) => GlobalReply::Mult(m.into_iter().map(|kvc|to_main_reply(kvc)).collect()),
  }
}


