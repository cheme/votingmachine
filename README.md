votingmachine
=============

An implementation of a distributed vote process.

Build
-----

Use [cargo](http://crates.io) tool to build and test.

Status
------

TODO add travis-ci , rust-ci for doc

wip
TODO : type and store for trusted keyval

Process Overview
----------------

- a vote description is initiated, with a subject, and possible answers. It is given an Id.
TODO vote access with a wot.
It is issued for specific users (votant).
Access to this Vote must be restrained, as it could result to simple blocking attacks (usage of subvote to diminished those attacks also reduced effectivness of anonymate (probability to get n identical vote where you got n votant)).

- we enter envelope period : each peer with access to the vote description will generate a Envelope (key pair) and send it (not the private key obviously) anonymously.
Anomously is done by mode of send propagate. TODO enable tor transport sending. Enveloppe is (envelope id, vote id, publickey).

- At the end of envelope period (no new envelope expected), every peer search for all envelope, and locally constituate them vote. 

- We enter participation period : peer sign the envelopes and send their participation (this sign).

- end of participation period, peer vote if participant and envelope seems ok with him (number of envelope/number of participant, presence of its own envelope signed by participants...), if wrong he vote rejected vote with its envelope (or subvote process).

- anonymous send (send propagate tunnel and TODO tor), of reply (using own envelope pkey: id envelope + id vote + vote + envsign).

- end of vote periode. Every voter can open all replies and update its vote results (after valdating replies of course (no dup... two reply for an enveloppe invalidate the envelope))

- synchro of results between votant to reach consensus : TODO (similar to getting participants).

When reply receive envelope is no longer needed, the fact that we do not imediatly send reply is to avoid sending information in a vote where we could have a corrupt protocol and avoid participant signing enveloppes depending on their content.

