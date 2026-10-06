# Lot, then vote

A design for filling a legislative chamber in which **candidates are drawn by
lot**, jury-style, and **voters then choose among them as normal**. The second
step is what this crate computes: given voters' graded preferences over a
candidate list, allocate the seats. The first step is what this note is about:
where the candidate list comes from.

Design note, 6 October 2026. The author's decisions are marked *decided*; the
rest is open. Nothing here is implemented in the engine, and nothing in the
engine depends on it. `examples/tenure.rs` is a toy model of the one problem
the design has to solve.

## The design in one paragraph

Each electoral cycle a stratified random sample of eligible citizens is drawn
for each seat, the way a jury panel is. Service is voluntary with opt-outs,
paid, and job-protected. Draws are vetted and redrawn until the quotas (age,
sex, region, and whatever else the statute names) are met. These people, plus
any eligible sitting member, are the only candidates. Voting then proceeds under
the existing system, in Ireland PR-STV, which this crate's quota and surplus
phases model. The design applies first to an advisory second chamber, or to a
reserved subset of seats in the lower chamber, not to the whole of it.

## Ancestry

- Venice elected its doge through nine alternating rounds of lot and ballot.
- Florence ran the two stages the other way round: a vote screened the names,
  then the lot drew from the bag.
- The Irish Citizens' Assembly (2016 on) and Ostbelgien's permanent citizens'
  council recruit exactly as the pool here is recruited: stratified,
  voluntary, paid, redrawn to quota.
- Van Reybrouck, *Against Elections*; Guerrero, "lottocracy"; Bouricius,
  multi-body sortition, for the wider argument.

The combination of a lottery pool with a conventional ballot among the pool is
the part that is new, and it is where the one real flaw lives.

## The flaw: incumbency

If sitting members can always stand and challengers can only come from the
pool, every contest is a known name against strangers drawn last year.
Conventional voting would then return incumbents more reliably than the
present system does. The example measures this.

## Experience without entrenchment (*decided*, 6 October 2026)

The chamber should keep experience; no person should be promised a path. Those
are separable, and the design keeps them separate.

1. **One automatic renewal.** A sitting member gets a ballot place for exactly
   one re-election.
2. **A decaying ticket after that.** They go back into the draw with a
   weighted ticket that halves each term: certain, then a half, a quarter, an
   eighth. Nobody is barred; the tail of long-servers shrinks on its own.
3. **Staggered terms.** A third or a half of the chamber is drawn at a time,
   so the chamber always holds a majority who have done it before.
4. **An alumni college.** Members who leave join a paid body that briefs and
   mentors incoming members and staffs committees. Most of what a
   long-serving member knows is procedure and institutional memory, and that
   transfers without the seat.
5. **A step in pay for the second term.** Retains people through the years
   they are most useful, at no cost in seats.

With five-year terms this gives a reliable ten years and an unusual but
possible twenty.

Alternatives considered and set aside: *earned renewal* (a review decides who
keeps the ballot place; rewards experience that was used, but creates a grader
to capture) and a *senior bench* (a quarter of seats reserved for second-term
members; a hard ceiling on careerism, but a visible two-tier chamber).

### What the toy model shows

`cargo run --example tenure` runs a 60-seat chamber, a third of the seats
contested each cycle, for about 133 elections per seat under each renewal
rule, with an incumbent who reaches the ballot winning 70% of the time, and
reports how much of the chamber is long-serving. Service is counted in terms
and read as five years a term. Share of members with 15 or more years'
service (three or more terms):

| Rule | Incumbent wins 70% | Incumbent wins 85% |
| --- | --- | --- |
| Always eligible (the original proposal) | about half | about three quarters |
| Two terms and out | none | none |
| One renewal, then a decaying ticket | about a sixth | about a fifth |

Uncalibrated, and a fixed-seed run rather than a sweep, so read the ordering
and not the digits. The decaying ticket keeps a seasoned minority without the
chamber becoming a retirement home.

## Parties (*decided*, 6 October 2026: prior affiliation only)

The lottery removes the one real power a local branch holds, the selection
convention. What remains is canvassing, clinics, fundraising and the ladder
from council to Dáil; the alumni college replaces the ladder. The brand, the
cheapest information a voter gets, was the open question. Three settings were
considered:

- **Prior affiliation only** (*decided*). A pool candidate may state the party
  they belonged to before the draw, on the record, and nothing else. Parties
  may not nominate, endorse or spend. The label survives, the pool cannot be
  courted after the fact, and a stratified pool shows the real spread of
  partisanship.
- **No role.** Labels and party spending banned; a state platform and
  hustings carry the information. Low information favours the articulate and
  the locally well known, which is a different elite.
- **Endorsement after the draw.** Parties pick from the pool. Keeps the
  machines on board, but it is Florence in reverse: the lot screens, the party
  chooses.

Grouping inside the chamber is free. An advisory chamber needs no whips.

## Sizing (*decided*, 6 October 2026)

The lower house has as many seats as the cube root of the voting
population. The upper house has about a third of that. The cabinet, or
executive council, has about a quarter of the upper house again, not
counting deputy ministers. The first rule is Taagepera's cube root law
(1972), which most national assemblies already sit near; the other two are
the author's, and they put a chamber of the drawn at a size where every
member can know every other, and an executive at a size that can meet
around one table.

| Voting population | Lower house | Upper house | Cabinet |
| --- | --- | --- | --- |
| 1 million | 100 | 33 | 8 |
| 3.5 million (Ireland) | 152 | 51 | 13 |
| 50 million | 368 | 123 | 31 |
| 240 million | 621 | 207 | 52 |

Ireland's Dáil, Seanad and cabinet ceiling (174, 60 and 15) sit close to
the row for its electorate, which is why the rule reads as a description
before it reads as a reform. A pool of drawn candidates is sized from the
same arithmetic: so many names per seat, the seat count fixed by the rule.

## AI mediation and synthesis (*decided*, 6 October 2026)

A drawn stranger needs, in a week, the map of their constituents that a
party machine used to supply, and the quiet candidate lacks it most. That is
the case for machine synthesis of what citizens say, and the reason to build
it under constraints rather than not at all. Three jobs, kept apart:
elicitation gathers what people say in their own words; synthesis finds the
structure; mediation puts the structure back to the people and to the
chamber in a form they can argue with. The author's ordering, opinions over
values over needs, is a depth ordering: opinions change weekly, values are
slower and conflict within one person, needs are few and are where people
who disagree about everything else agree.

The rules, each answering a known failure:

- every claim in a synthesis cites the utterances behind it, or it is the
  machine's opinion;
- disagreement is represented as structure, clusters and bridges, never as
  one paragraph that sands off the minority;
- interpretation is never generated; the machine clusters and quotes, and
  saying what the structure means is a signed human act;
- several mediators rather than one, with their agreement reported, so a
  structure that survives a change of method can be told from an artefact;
- the synthesis is a briefing to the drawn chamber, whose members may dissent
  from it on the record, with the dissents published beside it;
- confidence is carried as how many, how stable, how contested, with
  contradictions tracked rather than resolved.

The reference implementation is `episteme/population.py` in
`dermot-r-cochran/swarm` (ADR-0004 there), which emits cited structure,
OPINION claims and no beliefs, and can produce no interpretation. Its first
population is The Archipelago's simulated citizens, read from a published
export of `dermot-r-cochran/virtual-anthropology`, who have no privacy to
lose and a hash-chained record to check the synthesis against. Its home in
this design is the alumni college's briefing of the newly drawn.

## Where it applies first

- **A local council.** Article 28A of the Irish Constitution leaves council
  candidacy to ordinary law, so a lottery pool for a council needs no
  referendum. Councils are low-stakes, already the venue where citizens'
  assemblies have been tried in Dublin, and the place where a party is most
  plainly just its canvassers. The pilot.
- **Seanad.** Article 16 makes every adult citizen eligible for the Dáil, so a
  lottery-gated candidacy there needs a referendum. Article 19 already allows
  Seanad election through vocational groups, and the present panels are a
  failed attempt at representative composition. A stratified lottery pool is a
  more honest version of what they were meant to be, and the Seanad does not
  have to form a government. The Manning report (2015) and the Seanad Bill
  2020 are the reform track to attach to. The second step.
- **Lower-chamber subset.** One seat in each five-seat PR-STV constituency
  reserved for pool candidates, ranked by voters as normal. Keeps parties out
  of the pool. The renewal rules matter most here.

## Younger and quieter members

Elections reward the performative and the already known, and a lottery pool
only gets a young or quiet person onto the ballot. Getting them elected, and
making the term worth having, takes more.

What would improve the parties, once selection is gone:

- **Fund parties for training delivered, not votes won.** Public funding tied
  to members put through a candidate-preparation curriculum, accredited by
  the alumni college. Branches become schools; a youth wing becomes an
  apprenticeship rather than a canvass pool.
- **Make branches deliberative.** Facilitated round tables on live proposals,
  the citizens' assembly format, with a written output to the party's policy
  forum. Quiet members are good at this, and canvassing never used it.
- **Prior affiliation puts membership on the ballot.** A party then has an
  interest in members who would do well if drawn, which is a reason to
  recruit broadly and train, not to recruit loud.

For younger members:

- **Pool eligibility at eighteen.** Councils already allow it by law; the
  Dáil and Seanad are at twenty-one by Articles 16 and 18, so the pilot can do
  it and the chambers cannot without a referendum.
- **Stratify by age band**, so the youngest band is in every pool at its
  population share. The quota is in the draw; the vote stays free.
- **Protection that fits a young life.** A college place held and exams
  deferred, childcare provided, and alumni college membership recognised as a
  credential afterwards. Job protection alone protects people who already
  have a job.
- **A paid run-in year** of reading, procedure and shadowing, so a
  twenty-two-year-old enters knowing the standing orders.

For quieter members:

- **Candidate information that does not reward performance.** A standard
  written statement, a structured question-and-answer with equal space, and
  hustings run with facilitated equal time rather than open debate. This is
  what the state platform is for.
- **Procedure inside the chamber.** Written contributions and committee work
  weighted equal to floor speech in the record; speaking order by lot, the
  Athenian practice; small-group deliberation before plenary, which is how a
  citizens' assembly draws out the people who would not stand up in a hall.
- **A mentor from the alumni college** for the first term.
- **Keep the opt-out cheap rather than stratify on temperament.** If
  opt-outs skew the pool toward the confident, the fix is pay and protection
  high enough that the quiet say yes; there is no quota for quietness.

## Open points

- **Vetting.** Criteria must be in statute and narrow, or the pool quietly
  excludes the people a representative sample is meant to include.
- **Opt-out skew.** Pay and job protection fix part; redraw-to-quota fixes the
  rest.
- **Pool size and run-in.** How many names per seat, and how long before
  polling day the draw happens. A year is the working assumption.
- **The name.** "Lot, then vote" is a placeholder.

## Elsewhere in the account

`dermot-r-cochran/virtual-anthropology` carries the same design read against
the Archipelago's governance, where there is no randomness and a draw would
have to be a declared enumeration: `the-archipelago/docs/lot-then-vote.md`.
The two notes were written on the same day from the same conversation and say
the same thing; a decision changed in one is changed in the other.
