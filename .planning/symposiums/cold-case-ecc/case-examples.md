# ECC Cold Case Examples: Two Detailed Walkthroughs

**Purpose**: Demonstrate how the Evidence Coherence Computing (ECC) system works against real-world investigative conditions -- messy evidence, incomplete work, jurisdictional gaps, witness problems.  
**Audience**: Homicide detectives, cold case supervisors, prosecutors  
**Date**: April 2026

---

## EXAMPLE 1: "The Parking Garage Case"

**Stranger Homicide | Evidence-Rich but Disorganized | Case #2019-HM-04417**

### The Facts

**Victim**: Maria Delgado-Reyes, 34, paralegal at Hinton & Associates law firm, 6th floor of the Pacific Centre office tower. Found deceased in the stairwell between floors 3 and 4 of the attached parking garage at 6:47 AM on Tuesday, March 12, 2019, by a maintenance worker.

**Cause of death**: Blunt force trauma to the right temporal region. ME estimated time of death between 8:00 PM and 11:00 PM on Monday, March 11.

**The original investigation**: Detective Carl Brannigan caught the case. He ran a solid first 72 hours -- canvassed the building, pulled the victim's phone records, got six cameras worth of surveillance footage onto disc, collected DNA from under the victim's fingernails and partial latent prints from the stairwell door handle, and took 14 witness statements from building employees and garage users. He submitted the fingernail DNA to CODIS. No match.

**Why it went cold**: Brannigan was diagnosed with prostate cancer in November 2019 and retired on medical leave in January 2020. The case was reassigned to Detective Nora Padilla, who was already carrying 11 active homicides. She reviewed the file, ran the partial prints through AFIS again (no match), and put it in the stack. By mid-2020, COVID had shut down most cold case review. The file sat.

**What exists in the case file as of the cold case review (2026)**:

| Evidence Item | Status | Notes |
|---|---|---|
| Surveillance footage, 6 cameras | Cameras 1-3 and 5-6 reviewed for 8-11 PM window only. Camera 4 (stairwell entrance) reviewed for 9-11 PM only. Full 24hr footage preserved on disc but never fully reviewed. | Brannigan's notes say he was "working through it" when he got sick. |
| Victim's phone records (Verizon CDR pull) | Pulled for 30 days prior to death. Call log printed and in file. No analysis notes beyond highlighting calls to/from the victim's mother and boyfriend. | 347 calls and texts in the 30-day window. |
| 14 witness statements | Taken March 12-15, 2019. Some contradictory on timing. | Witness #7 (night security guard, Carl Munoz) mentioned a "regular" who always used the stairs instead of the elevator. No follow-up documented. |
| DNA from under fingernails | Submitted to CODIS March 2019. No hit. Male profile obtained. | Last CODIS check: March 2019. |
| Partial latent prints, stairwell door | Run through AFIS March 2019, no match. 3 partial prints, 2 of sufficient quality for comparison. | Last AFIS check: March 2019. |
| Victim's vehicle (2016 Honda Civic) | Processed by crime scene unit. Interior swabbed. Hair fibers collected. | Hair fibers never submitted for analysis. |
| Victim's social media accounts | Listed in file as "Facebook, Instagram." Never examined. | Brannigan's notes: "Need warrant for SM accounts -- will draft." No warrant in file. |
| Building security badge swipe logs | Never subpoenaed. | Brannigan's notes on Day 5: "Get swipe logs from Pacific Centre management -- who was in the building after 6 PM." No follow-up documented. |
| Autopsy report | Complete. COD: blunt force trauma. Defensive wounds on forearms. Toxicology negative. | Wound pattern consistent with a single heavy instrument, possibly a pipe or similar cylindrical object. |
| Crime scene photos (48) | Complete set, properly documented. | Blood spatter pattern on stairwell wall suggests victim was standing when struck, fell forward. |

---

### Step 1: Data Ingestion into the Case Graph

The cold case detective (let's call her Detective Padilla, who now has capacity) loads the case file into the ECC system. The system creates the following graph:

**Nodes Created (37 total)**:

| Node ID | Type | Label |
|---|---|---|
| N-001 | PERSON:victim | `person:victim:delgado_reyes_maria` |
| N-002 | PERSON:witness | `person:witness:munoz_carl` (security guard) |
| N-003 through N-014 | PERSON:witness | 12 additional witnesses (building employees, garage users) |
| N-015 | PERSON:investigator | `person:investigator:det_brannigan` |
| N-016 | PERSON:boyfriend | `person:poi:victor_aguilar` (victim's boyfriend, cleared by alibi) |
| N-017 | EVENT:crime | `event:crime:homicide_2019-03-11` (TOD uncertainty: 180 minutes) |
| N-018 | LOCATION:crime_scene | `location:crime_scene:pacific_centre_stairwell_3-4` |
| N-019 | LOCATION:vehicle | `location:vehicle:civic_2016_garage_level_5` |
| N-020 | EVIDENCE:physical | `evidence:physical:dna_fingernails_01` (tested=true, CODIS=no_match_2019) |
| N-021 | EVIDENCE:physical | `evidence:physical:latent_prints_door_01` (tested=true, AFIS=no_match_2019) |
| N-022 | EVIDENCE:physical | `evidence:physical:hair_fibers_vehicle` (tested=false) |
| N-023 | EVIDENCE:digital | `evidence:digital:surveillance_cam1` through `evidence:digital:surveillance_cam6` (6 nodes) |
| N-029 | EVIDENCE:digital | `evidence:digital:phone_records_verizon` (partially_analyzed=true) |
| N-030 | EVIDENCE:testimonial | 14 witness statement nodes (N-030 through N-043) |
| N-044 | DOCUMENT:report | `document:report:autopsy_2019-04417` |
| N-045 | DOCUMENT:report | `document:report:csu_report_2019-04417` |

**Edges Created (52 total)**:

Key relationships the system builds from the case file:

- `N-001 (victim) --OCCURRED_AT--> N-018 (stairwell)` weight=1.0
- `N-001 (victim) --ASSOCIATED_WITH--> N-019 (vehicle)` weight=1.0
- `N-020 (DNA) --FOUND_AT--> N-018 (stairwell)` weight=1.0
- `N-020 (DNA) --IDENTIFIED_BY--> [UNKNOWN_MALE]` weight=0.0 (no identified target -- dangling edge)
- `N-021 (prints) --FOUND_AT--> N-018 (stairwell)` weight=1.0
- `N-002 (Munoz) --WITNESSED_BY--> N-017 (crime event)` weight=0.6 (indirect -- he didn't see the crime but has relevant knowledge)
- 14 witness statement edges linking each witness to their statement node
- `N-029 (phone records) --ASSOCIATED_WITH--> N-001 (victim)` weight=1.0
- Various `CONTRADICTS` edges between witness statements (3 pairs contradict each other on timing)

**What the system does NOT find**: No suspect nodes. No hypothesis nodes. No edges connecting any identified person to the crime event as a perpetrator. The graph has evidence islands -- clusters of nodes that don't connect to each other.

---

### Step 2: Initial Coherence Score

The system runs `spectral_analysis()` on the case graph.

```
CASE #2019-HM-04417: INITIAL COHERENCE ASSESSMENT
==================================================

Overall Coherence (lambda_2):  0.31
Interpretation:                POOR -- evidence graph is fragmented

Spectral Partition Analysis:
  Cluster A (16 nodes): Victim's personal life -- phone records, boyfriend, 
                        work colleagues, vehicle
  Cluster B (12 nodes): Physical crime scene -- DNA, prints, autopsy, CSU report,
                        stairwell location, crime event
  Cluster C (9 nodes):  Witness statements -- loosely connected to each other 
                        and to the crime scene

CRITICAL: Clusters A, B, and C have minimal cross-connections.
  - Phone records (Cluster A) share 0 edges with physical evidence (Cluster B)
  - Witness statements (Cluster C) reference times that were never correlated 
    with camera footage (Cluster B)
  - No identified suspect connects any cluster to any other

Untested evidence:            3 items
Unresolved database checks:   2 items (CODIS last run 2019, AFIS last run 2019)
Evidence never collected:     2 items (badge swipe logs, social media)
Partially analyzed evidence:  3 items (phone records, camera 4 footage, 
                              camera 1-3/5-6 full 24hr review)
```

**Why 0.31?** The evidence exists but it was never woven together. The phone records sit in the file as a printout, never cross-referenced against the witness list or the surveillance timeline. The witness who mentioned the "stair regular" was never shown photos or asked to elaborate. The camera footage was only spot-checked. The system sees a graph with three disconnected islands and almost no bridges between them.

---

### Step 3: Gap Analysis Output

The system runs all 8 gap detection patterns and produces this prioritized output:

```
CASE #2019-HM-04417: GAP ANALYSIS
===================================
Generated: 2026-04-04 14:32:17
Total gaps identified: 11
Priority method: Counterfactual coherence delta (highest impact first)

GAP-001 [HIGH] Badge swipe logs never subpoenaed
  Source: Det. Brannigan's notes, Day 5: "Get swipe logs from Pacific 
  Centre management"
  Issue: Building uses HID badge access on floors 1-7 and garage levels. 
  Logs would show every person who entered floors 3-7 via badge after 
  6:00 PM on March 11, 2019.
  Action: Contact Pacific Centre property management. Determine log 
  retention policy. If logs still exist, subpoena immediately. If 
  purged, determine if backup exists.
  Coherence impact: +0.23 (0.31 -> 0.54) -- connects Cluster A 
  (persons) to Cluster B (crime scene) by placing specific people 
  in the stairwell area.
  Time sensitivity: HIGH -- commercial badge systems typically retain 
  logs 1-7 years. At 7 years out, these may be gone.

GAP-002 [HIGH] Unknown phone number contacted victim 14 times in 
  final week
  Source: Verizon CDR printout, page 3. Number (503) 555-0147 appears 
  14 times between March 4-11, 2019. Calls range from 22 seconds to 
  11 minutes. No notation in file identifying this number.
  Issue: High-frequency contact in the week before a homicide is a 
  standard lead. This number was never run through any database, 
  never served with a subpoena, never identified.
  Action: Run reverse lookup. If prepaid/burner, subpoena carrier 
  records for that number's call history and cell site data for 
  March 11.
  Coherence impact: +0.17 (0.31 -> 0.48) -- if identified, creates 
  a new PERSON node with direct communication link to victim in 
  the critical pre-homicide window.

GAP-003 [HIGH] Witness #7 (Munoz) mentioned "stair regular" -- 
  never followed up
  Source: Witness statement, Carl Munoz, March 12, 2019: "There's a 
  guy who comes through most nights, always takes the stairs instead 
  of the elevator. I figured he was doing it for exercise. White male, 
  30s or 40s, usually wearing a backpack. I saw him that Monday night 
  around 9, 9:30."
  Issue: A person placed in the stairwell on the night of the homicide, 
  within the TOD window, who was never identified, never shown a photo 
  lineup, never interviewed.
  Action: Re-interview Munoz. Show photo arrays. Cross-reference 
  description against badge swipe logs (GAP-001). Check if cameras 
  captured anyone matching description.
  Coherence impact: +0.14 (0.31 -> 0.45) -- creates a potential 
  PERSON:poi node directly connected to the crime scene and time window.

GAP-004 [HIGH] Camera 4 (stairwell entrance) only reviewed for 
  2-hour window
  Source: Evidence log notes Camera 4 footage reviewed 9:00-11:00 PM 
  only. TOD window begins at 8:00 PM. Additionally, full 24-hour 
  review could capture the suspect arriving earlier and establishing 
  presence.
  Issue: One hour of the TOD window was never reviewed on the most 
  relevant camera. If the attack happened at 8:15 PM, the suspect 
  entering the stairwell at 8:10 PM would have been on footage 
  nobody watched.
  Action: Review Camera 4 footage for full 24 hours of March 11, 2019. 
  Cross-reference any persons seen with badge swipe logs and witness 
  descriptions.
  Coherence impact: +0.11 (0.31 -> 0.42) -- could directly capture 
  suspect on video entering stairwell.

GAP-005 [MEDIUM-HIGH] CODIS resubmission -- database last checked 2019
  Source: Lab report, March 2019. Male DNA profile obtained from 
  fingernail scrapings. CODIS search negative as of March 22, 2019.
  Issue: CODIS grows by approximately 300,000 profiles per year. In 
  7 years, roughly 2.1 million new profiles have been added. The 
  person whose DNA is under the victim's fingernails may have been 
  arrested, convicted, or otherwise entered into the system since 2019.
  Action: Resubmit DNA profile to CODIS for updated search. Estimated 
  turnaround: 30-90 days depending on lab backlog.
  Coherence impact: +0.36 (0.31 -> 0.67) IF there is a match. 
  Probability of match estimated at 8-12% based on HNSW comparison 
  with similar cold cases where resubmission produced a hit.
  Expected value: +0.036 (impact * probability). Still worth doing -- 
  cost is near zero and a hit is case-breaking.

GAP-006 [MEDIUM-HIGH] AFIS resubmission -- database last checked 2019
  Source: Latent print report, March 2019. 2 of 3 partial prints from 
  stairwell door handle were of comparison quality. AFIS search 
  negative March 2019.
  Issue: Same growth logic as CODIS. NGI database adds hundreds of 
  thousands of records annually.
  Action: Resubmit to NGI/AFIS. Consider enhanced processing 
  techniques developed since 2019 on the third partial print.
  Coherence impact: +0.28 IF match. Probability estimated at 5-8%.

GAP-007 [MEDIUM] Cameras 1-3, 5-6 reviewed for 3-hour window only
  Source: Evidence log. Footage reviewed for 8-11 PM window across 
  5 cameras. No full-day review conducted.
  Issue: If the perpetrator surveilled the garage earlier in the day, 
  entered hours before, or was captured on camera at other times, 
  those images exist on unreviewed footage.
  Action: Full 24-hour review of all cameras for March 11, 2019. 
  Run facial recognition / person-tracking if department has access.
  Coherence impact: +0.08 (incremental over GAP-004 review).

GAP-008 [MEDIUM] Hair fibers from victim's vehicle never submitted
  Source: CSU evidence log item #14. Hair fibers collected from 
  driver's seat headrest and passenger seat. Stored in evidence locker.
  Issue: If the perpetrator was in the victim's vehicle at any point, 
  these fibers could provide DNA or at minimum hair comparison evidence.
  Action: Submit for mitochondrial DNA analysis. Cross-reference 
  against fingernail DNA profile.
  Coherence impact: +0.06 (connects vehicle evidence cluster to 
  physical crime scene cluster if DNA matches).

GAP-009 [MEDIUM] Victim's social media accounts never examined
  Source: Det. Brannigan's notes: "Facebook, Instagram -- need warrant." 
  No warrant in file.
  Issue: Victim's online activity in the days and weeks before death 
  could reveal threats, stalking behavior, planned meetings, or 
  unknown associates. Social media DMs are a primary communication 
  channel that bypasses phone records entirely.
  Action: Serve preservation request / legal process on Meta for 
  account data. Note: 7 years post-mortem, account may be 
  memorialized. Meta retains data for law enforcement requests on 
  memorialized accounts.
  Coherence impact: +0.09 (could introduce new PERSON nodes and 
  communication edges not visible in phone records).

GAP-010 [MEDIUM-LOW] Three pairs of contradictory witness statements 
  never reconciled
  Source: Statements from witnesses #2/#5 (disagree on whether garage 
  lights were on at 9 PM), witnesses #4/#9 (disagree on whether they 
  heard shouting), witnesses #8/#11 (disagree on timing of a car alarm).
  Issue: Contradictions were documented but never investigated. 
  Re-interviews to resolve contradictions could firm up the timeline.
  Action: Re-interview the 6 witnesses involved in contradictory pairs. 
  Use camera footage timestamps (once reviewed) to resolve factual 
  disputes.
  Coherence impact: +0.04 (replaces Contradicts edges with confirmed 
  timeline, improving graph connectivity).

GAP-011 [LOW] Phone records: 347 calls/texts in 30-day window -- 
  only 2 numbers annotated
  Source: Verizon CDR printout. Brannigan highlighted calls to victim's 
  mother and boyfriend. 345 other entries have no annotation.
  Issue: Comprehensive phone analysis is baseline investigative work. 
  Every number should be identified or at least categorized 
  (known contact, business, unknown).
  Action: Run full CDR analysis. Cross-reference all numbers against 
  witness contact information, building tenant directory, and known 
  associates.
  Coherence impact: +0.07 (connects phone record cluster to witness 
  and person clusters across the graph).
```

---

### Step 4: Counterfactual Analysis

The system runs "what if" simulations, temporarily adding synthetic nodes and edges to model what happens when each gap is resolved.

```
COUNTERFACTUAL ANALYSIS: PRIORITIZED ACTION RANKING
====================================================

Current coherence: 0.31

Scenario                                    | New Score | Delta | Cost Est.  | Priority
--------------------------------------------|-----------|-------|------------|----------
A. CODIS resubmission produces a match      |    0.67   | +0.36 | $0 (lab)   | 1 (free)
B. Badge swipe logs obtained + analyzed     |    0.54   | +0.23 | 4-8 hrs    | 2
C. AFIS resubmission produces a match       |    0.59   | +0.28 | $0 (lab)   | 3 (free)
D. Unknown phone # (503-555-0147) identified|    0.48   | +0.17 | 2-4 hrs    | 4
E. Munoz re-interviewed, stair regular IDed |    0.45   | +0.14 | 2-3 hrs    | 5
F. Camera 4 full review completed           |    0.42   | +0.11 | 6-10 hrs   | 6
G. Social media accounts obtained           |    0.40   | +0.09 | 2-4 weeks  | 7
H. Full CDR analysis completed              |    0.38   | +0.07 | 4-6 hrs    | 8

COMBINED SCENARIO: Resolve A + B + D simultaneously
  Projected coherence: 0.72
  Interpretation: Case moves from "fragmented" to "substantially coherent"

RECOMMENDED SEQUENCE:
  1. Submit CODIS and AFIS resubmissions TODAY (zero cost, high upside)
  2. Subpoena badge swipe logs THIS WEEK (time-sensitive -- may be purged)
  3. Run the unknown phone number THIS WEEK
  4. Re-interview Munoz and start Camera 4 full review NEXT WEEK
  5. Draft social media warrant (longer lead time, lower priority)
```

---

### Step 5: What Actually Happens

Detective Padilla follows the prioritized list.

**Week 1**:

She resubmits the fingernail DNA to CODIS and the latent prints to NGI. She subpoenas the badge swipe logs from Pacific Centre's property management company. She runs (503) 555-0147 through department databases.

The phone number comes back to a prepaid phone purchased at a Walmart in December 2018. Subscriber information is a dead end (fake name on the registration), but carrier records show the phone's cell site data for March 11 -- it pinged a tower 0.3 miles from the Pacific Centre at 8:42 PM, 9:17 PM, and 10:03 PM.

**The system updates**:

```
NEW NODE: person:poi:unknown_prepaid_user (connected to phone number,
  connected to cell tower near crime scene on night of murder)
NEW EDGES: 
  - ASSOCIATED_WITH: unknown_prepaid -> victim (14 calls in final week)
  - CORROBORATES: cell_tower_ping_842pm -> event:crime (within TOD window)
  
Updated coherence: 0.31 -> 0.47
```

**Week 2**:

Pacific Centre's access control vendor (Kastle Systems) confirms they retain badge swipe data for 10 years. Logs are produced for March 11, 2019, 6:00 PM through midnight. Twenty-three badge swipes after 6:00 PM. Cross-referencing tenant names against the prepaid phone carrier records (cell tower proximity) and the witness list narrows the field.

Three people badged into the building after 8:00 PM:
- Rebecca Holt, attorney on floor 7 -- badged in at 8:14 PM, badged out at 9:52 PM
- James Wen, IT contractor -- badged in at 7:33 PM, badged out at 8:05 PM
- Darren Kohler, building maintenance -- badged in at 8:38 PM. No badge-out recorded.

**The system updates**:

```
NEW NODES: 3 PERSON:poi nodes
NEW EDGES: Multiple ASSOCIATED_WITH and TIMELINE edges
ALERT: Darren Kohler -- no badge-out recorded. Badge-in at 8:38 PM 
  falls within TOD window. Maintenance staff have stairwell access 
  without additional badge swipes.
ALERT: Cross-reference hit -- "Darren Kohler" matches description 
  from Witness #7 (Munoz): white male, 30s-40s, backpack, uses stairs.

Updated coherence: 0.47 -> 0.58
```

Padilla re-interviews Carl Munoz. Shows him a photo array that includes Kohler's employee badge photo. Munoz picks Kohler's photo: "Yeah, that's the stairs guy. I saw him that night, heading into the stairwell around 9, 9:15. He had his backpack."

```
NEW EDGE: WITNESSED_BY -- Munoz places Kohler in stairwell during 
  TOD window (weight=0.82, eyewitness identification from photo array)

Updated coherence: 0.58 -> 0.64
```

**Week 4**:

CODIS resubmission comes back: **HIT**. The male DNA profile from under Maria Delgado-Reyes' fingernails matches a profile entered into CODIS in November 2022. The profile belongs to Darren Michael Kohler, arrested for aggravated assault in Portland, Oregon, in October 2022. His DNA was collected at booking.

```
NEW EDGE: IDENTIFIED_BY -- DNA from victim's fingernails matches 
  Kohler (weight=0.999, CODIS confirmed match)

Updated coherence: 0.64 -> 0.81

HYPOTHESIS AUTO-GENERATED:
  hypothesis:primary:kohler_perpetrator
  Supporting evidence: 7 nodes
    - DNA under victim's fingernails (CODIS match)
    - Badge swipe into building at 8:38 PM, no exit
    - Eyewitness places him in stairwell during TOD window
    - Cell tower places prepaid phone (14 calls to victim) 
      near building during TOD window
    - Maintenance role = stairwell access
    - Victim's defensive wounds consistent with struggle 
      (DNA under nails)
    - No alibi documented
  Contradicting evidence: 0 nodes
  Hypothesis coherence: 0.81
```

Camera 4 full review, now conducted with Kohler identified, captures a figure matching his build entering the stairwell at 8:51 PM and exiting through the ground-level fire door at 10:27 PM. Coherence reaches 0.87. Arrest warrant is drafted.

---

### Step 6: Graph Evolution Timeline

```
March 2019 (original investigation):
  37 nodes, 52 edges
  Coherence: would have been ~0.31 if measured
  3 disconnected clusters, no suspect identified

January 2020 (Brannigan retires):
  No changes to graph. Work stopped.

2020-2025:
  Case file sits in a cabinet. Zero activity.

April 2026, Day 1 (case loaded into ECC):
  37 nodes, 52 edges, coherence = 0.31
  11 gaps identified, ranked by coherence impact

April 2026, Week 1 (phone number + cell tower):
  40 nodes, 61 edges, coherence = 0.47
  Unknown prepaid user placed near crime scene

April 2026, Week 2 (badge swipe logs + Munoz re-interview):
  46 nodes, 78 edges, coherence = 0.64
  Kohler identified as POI, placed in stairwell by witness

April 2026, Week 4 (CODIS hit + camera review):
  49 nodes, 91 edges, coherence = 0.87
  Kohler identified by DNA, captured on video
  Arrest warrant issued

Total investigative time: ~40 hours over 4 weeks
Gap that broke the case: Badge swipe logs (never pulled in 2019) 
  + CODIS resubmission (7 years of database growth)
```

The case was not unsolvable in 2019. The evidence was there. What was missing was someone connecting the dots -- and a system that could tell the next detective exactly which dots needed connecting.

---
---

## EXAMPLE 2: "The Riverside Shooting"

**Gang-Adjacent Homicide | Witness Intimidation | Cross-Jurisdictional Ballistics | Case #2017-HM-07823**

### The Facts

**Victim**: DeShawn Marcus Carter, 22, part-time warehouse worker at FedEx Ground. Found dead from two gunshot wounds (chest and abdomen) at Riverside Park, lying on the walking path near the boat ramp, at 10:58 PM on Saturday, June 17, 2017. Pronounced dead at scene by responding paramedics.

**Background**: Carter grew up in the Terrace View apartments on the east side. He had childhood friends in the East Terrace Crew (ETC), a loosely organized neighborhood group that had ongoing conflict with the Riverside Blocc (RB), who controlled the park and surrounding blocks. Carter was not a documented gang member. He had one prior arrest -- misdemeanor marijuana possession in 2015, charges dropped. His mother described him as "trying to get out." He had applied to a community college welding program two weeks before his death.

**The players** (as documented in the original investigation):

| Person | Role | Status in 2017 Investigation |
|---|---|---|
| DeShawn Carter | Victim | -- |
| Keisha Brown | Victim's girlfriend | Cooperative initially. Said Carter was "meeting someone about money." Would not say who. |
| Marcus "Lil Reese" Thompkins | Suspect A | ETC-affiliated. Argued with Carter 2 weeks prior at a barbecue. Alibi: says he was at his cousin's house. |
| Jamal "JB" Williams | Suspect B | RB-affiliated. Seen near the park that night by Witness #3. Has prior weapons charge. |
| Terrence Odom | Suspect C | ETC-affiliated. Considered low-priority by original detective because his girlfriend said he was home all night. No criminal history. |
| Darius Holt | Person of Interest | RB-affiliated. Carter owed him money (per street-level informant). Not located for interview. |
| Antoine "Toine" Briggs | Person of Interest | RB-affiliated. Seen at the park earlier that evening. Refused to talk to police. |
| Witnesses #1-8 | Various | See below. |

**Witness situation**:

| Witness | Initial Statement | Status by December 2017 |
|---|---|---|
| #1, park jogger (civilian) | Heard 2 shots around "9:30 or 10." Didn't see shooter. | Cooperative throughout. |
| #2, woman walking dog | Saw "two or three men" near the boat ramp at ~9:45 PM. One was "big, wearing a red hoodie." | Moved out of state. Available by phone. |
| #3, teenager on bike | Saw Jamal "JB" Williams near the park entrance around 9:15 PM. | Recanted after receiving threats. Family won't allow contact. |
| #4, gas station clerk | Carter bought a Gatorade at the Shell station (0.4 mi from park) at 9:22 PM. Has receipt. | Cooperative. Provided surveillance from store camera showing Carter alone. |
| #5, Carter's cousin | Said Carter told him earlier that day he was "gonna handle something tonight." | Stopped cooperating after 3 months. |
| #6, RB associate | Initially said he was at the park but "left before anything happened." Placed JB Williams and Antoine Briggs at the park at 9 PM. | Recanted. Lawyer'd up. |
| #7, neighbor near park | Heard shots at "about quarter to 10." Looked out window, saw someone running toward Elm Street. | Consistent statement throughout. Available. |
| #8, anonymous tipster | Called CrimeLine 3 days after murder. Said "Terrence did it over money DeShawn owed Darius." | Anonymous. Never identified. Could not be re-contacted. |

**Physical evidence**:

- Two 9mm shell casings recovered at scene (Federal HST hollow point)
- Bullet fragments recovered at autopsy (insufficient for NIBIN comparison)
- Ballistics on casings entered into NIBIN -- **matched to casings from a drive-by shooting in Millville (30 miles away) on September 22, 2017**. This match was returned to the original detective in October 2017 but was never investigated because Millville PD is a different jurisdiction and the detective's notes say only: "NIBIN hit -- Millville case. Will follow up." No follow-up documented.
- Cell tower data pulled for Suspects A and B for June 17, 2017. Not pulled for Suspect C, Darius Holt, or Antoine Briggs.
- Social media: the original detective screenshotted 11 Instagram and Facebook posts from various people posting from or near the park on June 17, 2017. The screenshots are in the file. The original accounts are now deleted or set to private.
- No DNA collected at scene (outdoor shooting, no close-contact evidence).
- No fingerprints (casings wiped or handled with gloves, per CSU notes).

---

### Step 1: Social Network Analysis

The ECC system ingests the case file and immediately begins mapping relationships. This is where the system's `spectral_partition()` and community detection capabilities earn their keep.

**Network graph built (63 nodes, 104 edges)**:

The system maps every person mentioned in every statement, every phone record, every social media screenshot, and every criminal history cross-reference. It doesn't just map the 5 suspects and POIs -- it maps their associates, the victim's associates, and the overlaps.

```
COMMUNITY DETECTION OUTPUT (Fiedler vector partition):
======================================================

Cluster 1 -- East Terrace Crew orbit (18 persons):
  Core: Marcus Thompkins (Suspect A), Terrence Odom (Suspect C),
        4 other documented ETC members
  Periphery: Carter (victim), Carter's cousin (Witness #5), 
        6 associates with ETC social connections but no 
        gang documentation
  NOTE: Carter (victim) and Odom (Suspect C) share 3 mutual 
  associates and attended the same barbecue on June 3, 2017

Cluster 2 -- Riverside Blocc orbit (14 persons):
  Core: JB Williams (Suspect B), Darius Holt (POI), 
        Antoine Briggs (POI), 5 other documented RB members
  Periphery: 4 associates

Cluster 3 -- Civilian / No affiliation (8 persons):
  Witnesses #1, #2, #4, #7, Keisha Brown (girlfriend), 
  3 family members

CRITICAL BRIDGE NODES (high betweenness centrality):
  - Darius Holt: connects Cluster 1 and Cluster 2. Holt has 
    documented associations with BOTH ETC and RB members. 
    Carter owed him money. He was never located for interview.
    Betweenness centrality: 0.67 (highest in the network)
  - Witness #6 (RB associate who recanted): connects Cluster 2 
    to the crime event. Only person who placed specific RB 
    members at the park before the shooting.
```

**What this tells the detective**: Darius Holt is not a peripheral player. He is the single most connected node between the two rival groups, and the victim owed him money. The original investigation treated him as a low-priority POI because he wasn't at the scene. The network structure says he may be the reason the victim was at the park.

---

### Step 2: Cross-Jurisdictional Pattern Matching

The NIBIN hit linking the Riverside Park casings to the Millville drive-by is already in the case file -- it just was never investigated. The ECC system flags this immediately as a GAP, but it also does something the original detective couldn't easily do: it pulls the Millville case into the same graph.

```
CROSS-JURISDICTIONAL LINK DETECTED
====================================
NIBIN Match: Federal HST 9mm casings
  Case A: #2017-HM-07823 (Riverside Park, June 17, 2017)
  Case B: Millville PD #2017-4102 (Drive-by, Cherry St, Sept 22, 2017)

HNSW similarity search found Case B in regional database.
Importing Case B evidence into unified graph...

Case B summary (from Millville PD records):
  - Drive-by shooting, 2 victims wounded (non-fatal)
  - 4 casings recovered, same Federal HST 9mm
  - Millville PD identified a cooperating witness who stated the 
    shooter was "T-Money from over east" driving a silver Altima
  - "T-Money" identified by Millville PD as Terrence Odom, DOB 
    1994-08-11, last known address: 445 Terrace View Apt 3B
  - Odom was never charged in the Millville case (insufficient 
    evidence beyond single witness statement, witness recanted 
    before grand jury)

SYSTEM ALERT: Terrence Odom (Suspect C in Case A) is the identified 
shooter in Case B. Same gun used in both shootings. Original detective 
in Case A classified Odom as "low-priority" based on girlfriend's alibi.

NEW EDGES CREATED:
  - evidence:ballistic:casings_riverside --CORROBORATES--> 
    evidence:ballistic:casings_millville (weight=0.97, NIBIN confirmed)
  - person:suspect:odom --ASSOCIATED_WITH--> event:crime:millville_driveby
  - person:suspect:odom --ASSOCIATED_WITH--> evidence:ballistic:firearm_9mm 
    (the linked gun)

Updated coherence: 0.26 -> 0.44
```

One database query. Two cases connected. A "low-priority" suspect just became the primary suspect.

---

### Step 3: Witness Credibility Mapping

The system does not assign credibility scores to witnesses. What it does is map which statements are independently corroborated by other evidence, and which statements contradict other evidence. This is the difference between "is this witness believable?" (a judgment call) and "does this witness's account match the physical evidence?" (a measurable fact).

```
WITNESS CORROBORATION MATRIX
==============================

Witness #8 (anonymous tipster): "Terrence did it over money 
  DeShawn owed Darius"
  
  Statement components:
    [a] Terrence Odom is the shooter
    [b] Motive was money owed to Darius Holt
    [c] Implicit: Odom acted on behalf of or in connection with Holt

  Independent corroboration:
    [a] CORROBORATED by NIBIN match linking Odom to the same gun 
        used 3 months later (weight=0.97)
    [a] CORROBORATED by Millville PD witness identifying Odom as 
        shooter with same gun (weight=0.65, witness later recanted)
    [b] CORROBORATED by street-level informant report that Carter 
        owed Holt money (weight=0.55, informant reliability unknown)
    [b] CORROBORATED by Keisha Brown's statement that Carter was 
        "meeting someone about money" (weight=0.70)
    [c] CORROBORATED by network analysis: Holt is the bridge node 
        between ETC (Odom's group) and the victim

  Contradicting evidence:
    Odom's girlfriend states he was home all night (weight=0.30 -- 
    unverified alibi from intimate partner, low independent value)

  CORROBORATION SCORE: 0.78
  NOTE: The anonymous tipster's account is the single most 
  corroborated statement in the case file, despite being 
  uncorroborable in the traditional sense (can't cross-examine 
  an anonymous caller). The corroboration comes entirely from 
  independent physical evidence and witness statements.

---

Witness #3 (teenager, recanted after threats): Saw JB Williams 
  near park entrance ~9:15 PM

  Statement components:
    [a] JB Williams was near the park at 9:15 PM

  Independent corroboration:
    [a] CORROBORATED by cell tower data: Williams' phone pinged 
        tower covering park area at 9:08 PM and 9:31 PM 
        (weight=0.85)
    [a] CORROBORATED by Witness #6's original statement (before 
        recanting) placing Williams at the park at ~9 PM 
        (weight=0.50, reduced due to recantation)

  Contradicting evidence:
    Williams' own statement: says he was at a recording studio. 
    Cell tower data contradicts this -- studio is 4 miles from 
    park and served by different towers.

  CORROBORATION SCORE: 0.72
  NOTE: Witness #3 recanted under threats but the original 
  statement is corroborated by cell tower data that Williams 
  cannot explain away. The recantation does not erase the 
  corroboration -- it means the witness was pressured, not 
  that the statement was false.
```

**What this means for the detective**: You have two witnesses (one anonymous, one recanted) whose statements would be difficult to use in court standing alone. But the system shows that independent physical evidence -- ballistics, cell towers -- corroborates what they said. The statements become investigative leads worth pursuing, not dead ends because the witnesses are "unreliable."

---

### Step 4: Timeline Reconstruction with Uncertainty

Eight witnesses give different accounts of when the shooting happened. The original file has "TOD: between 9:15 and 10:30 PM" -- a 75-minute window. The system narrows it.

```
PROBABILISTIC TIMELINE RECONSTRUCTION
=======================================
Method: Constraint propagation using cell tower pings, social media 
post timestamps, 911 call time, gas station receipt, and witness 
statements weighted by corroboration score.

Fixed anchor points:
  9:22 PM -- Carter buys Gatorade at Shell station (receipt + camera)
  10:51 PM -- 911 call received (dispatch records)
  10:58 PM -- First responders arrive, victim pronounced dead

Witness time estimates (weighted by corroboration):
  Witness #1 (jogger): "9:30 or 10" -- weight 0.60 (vague)
  Witness #7 (neighbor): "quarter to 10" -- weight 0.80 (consistent, 
    corroborated by looking at clock)
  Witness #2 (dog walker): "9:45 or so" -- weight 0.65

Digital evidence:
  9:22 PM -- Carter at Shell (0.4 mi from park)
  9:31 PM -- Carter's phone pings tower serving park (last ping 
    before phone goes silent)
  9:08 PM, 9:31 PM -- Williams' phone pings park-area tower
  9:38 PM -- Instagram post by RB associate shows park area in 
    background (posted, then deleted; screenshot in file)
  9:47 PM -- Instagram story by separate person, geo-tagged 0.2 mi 
    from park: "heard some shit, everybody running"

Reconstruction:
  9:22 PM       Carter at Shell station (CONFIRMED, receipt)
  9:25-9:32 PM  Carter walks to park (estimated 8-10 min walk)
  9:31 PM       Carter's phone pings park tower (last activity)
  9:38 PM       Social media post from park area (scene still active)
  9:40-9:55 PM  ** SHOOTING WINDOW ** (highest probability: 9:42 PM)
  9:47 PM       "heard some shit" Instagram story
  ~10:00 PM     Witness #7 hears shots (stated "quarter to 10" but 
                human time estimation typically shifts 5-15 min)
  10:51 PM      911 call (51-minute delay between shooting and call -- 
                consistent with community-violence cases where 
                bystanders don't call immediately)

REFINED TOD: 9:40-9:55 PM (15-minute window vs. original 75-minute)
Confidence: 0.82

IMPACT: Narrowed window eliminates Suspect A (Thompkins) alibi 
question -- his cell tower data shows him 6 miles away at 9:42 PM. 
He was not at the park. Williams (Suspect B) WAS at the park per 
cell tower. Odom (Suspect C) has no cell tower data -- GAP.
```

---

### Step 5: Gap Analysis

```
CASE #2017-HM-07823: GAP ANALYSIS
===================================
Total gaps identified: 9

GAP-001 [CRITICAL] Cell tower records never pulled for Suspect C 
  (Odom), Darius Holt, or Antoine Briggs
  Issue: Cell tower data was pulled for Suspects A and B but NOT 
  for the three other persons of interest. Odom is now the primary 
  suspect based on ballistic evidence. His location during the 
  shooting window is completely undocumented.
  Action: Subpoena historical cell site records from carrier. Note: 
  carriers retain CDR/cell site data for varying periods -- AT&T 
  retains for 7 years, T-Mobile for 5 years, Verizon for rolling 
  period. At 9 years out, these records may be at or past retention 
  limits. Act immediately.
  Coherence impact: +0.21 if Odom's phone places him at or near 
  the park. Case-breaking combined with ballistic evidence.
  Time sensitivity: EXTREME. Records may already be purged.

GAP-002 [CRITICAL] Millville case cross-jurisdictional follow-up 
  never conducted
  Issue: NIBIN match returned October 2017. No documented contact 
  with Millville PD. Millville's cooperating witness identified 
  Odom. Millville may have additional evidence, interview transcripts, 
  or intelligence on Odom's activities and associates.
  Action: Contact Millville PD detective assigned to Case #2017-4102. 
  Request full case file. Specifically request: cooperating witness 
  identity and current location, vehicle information (silver Altima), 
  any surveillance footage, Odom interview transcripts if any.
  Coherence impact: +0.18 (additional evidence from second case 
  strengthens firearms link and may provide new witnesses).

GAP-003 [HIGH] Odom's alibi never verified
  Issue: Odom's girlfriend stated he was home all night. This was 
  accepted at face value. No independent verification attempted -- 
  no cell tower check, no social media review, no neighbor canvass 
  at Odom's address.
  Action: Pull Odom's cell tower data (see GAP-001). Check if 
  Odom's social media (even if deleted) shows activity from home 
  or elsewhere. Canvass Odom's building neighbors -- "Did you see 
  Terrence on the evening of June 17, 2017?" 9 years later this 
  is a long shot, but if a neighbor has a Ring doorbell or similar, 
  footage retention may still capture the era.
  Coherence impact: +0.15 if alibi is broken.

GAP-004 [HIGH] Darius Holt never located or interviewed
  Issue: Holt is the network bridge node (betweenness centrality 
  0.67). Carter owed him money. The anonymous tipster implicated 
  him as the motive source. He was never found.
  Action: Current address search via NCIC, DMV, probation/parole, 
  social media, LexisNexis. If located, interview regarding his 
  relationship with Carter, the debt, and his whereabouts on 
  June 17, 2017.
  Coherence impact: +0.13 (could establish motive chain: 
  Holt -> money dispute -> Carter at park -> Odom as enforcer).

GAP-005 [HIGH] Deleted social media accounts -- content not preserved
  Issue: 11 screenshots in file but original accounts are deleted. 
  Screenshots show usernames and timestamps but not full context 
  (comments, likes, tagged people, DMs).
  Action: Check Wayback Machine and social media archive sites for 
  cached versions. Serve legal preservation requests on Meta and 
  Snap Inc. for account data associated with the known usernames. 
  Note: Meta retains deactivated account data for law enforcement 
  for a limited period; this may be expired at 9 years.
  Coherence impact: +0.08 (incremental -- screenshots capture the 
  most critical data, but full account pulls could reveal DM 
  conversations).

GAP-006 [MEDIUM-HIGH] Gun never recovered
  Issue: The 9mm firearm used in both the Riverside Park shooting 
  and the Millville drive-by has never been recovered. If Odom 
  still has it, or if it's been used in additional incidents, 
  NIBIN may have more matches.
  Action: Run expanded NIBIN search for the specific firearm 
  signature across all participating jurisdictions. Check if any 
  firearms were recovered from Odom during his other contacts with 
  law enforcement. Search for firearms registered to or seized 
  from Odom and his known associates.
  Coherence impact: +0.10 (gun recovery with DNA/prints would be 
  case-breaking; even without recovery, additional NIBIN matches 
  strengthen the link).

GAP-007 [MEDIUM] Silver Altima from Millville case never traced 
  to Riverside Park case
  Issue: Millville PD's witness described Odom driving a silver 
  Altima. No vehicle investigation was done in the Riverside Park 
  case at all.
  Action: DMV search for vehicles registered to Odom in 2017. 
  Check if any surveillance cameras near the park captured a silver 
  Altima on June 17, 2017 (gas station at Shell, traffic cameras, 
  business cameras). The Shell station where Carter bought Gatorade 
  at 9:22 PM has camera footage in the file -- review for vehicles 
  in the parking lot.
  Coherence impact: +0.07

GAP-008 [MEDIUM] Witness #6 (RB associate) original statement 
  never tested against physical evidence
  Issue: Before recanting, Witness #6 placed JB Williams and Antoine 
  Briggs at the park at 9 PM. This was corroborated by cell tower 
  for Williams. Briggs' location was never checked.
  Action: Note that Williams being at the park does not make him the 
  shooter -- the ballistics point to a different gun owner. Williams 
  may be a witness, not a suspect. Consider approaching Williams 
  with the new ballistic evidence to see if he'll cooperate now 
  that the gun points away from him.
  Coherence impact: +0.09 (Williams as a cooperating witness, rather 
  than a suspect, could provide direct testimony about who fired 
  the shots).

GAP-009 [MEDIUM-LOW] Keisha Brown's "meeting someone about money" -- 
  never pressed for details
  Issue: Brown said Carter was meeting someone about money but 
  wouldn't say who. At 9 years out, she may be willing to talk. 
  Relationships change. Fear diminishes. The person she was 
  protecting may no longer be a threat, or she may have separated 
  from that world.
  Action: Re-approach Brown. Explain that the case has new evidence. 
  Ask specifically about Darius Holt and the debt.
  Coherence impact: +0.06 (could confirm motive and identify who 
  arranged the meeting at the park).
```

---

### Step 6: The Break

Detective work, guided by the prioritized gap list:

**The Millville connection** (GAP-002): Padilla contacts Millville PD. Their detective retired in 2020 but the case file is available. The cooperating witness -- a woman named Tanya Reed who was in the car during the drive-by -- had identified Odom as the shooter and described the gun as "Terrence's nine, he always carries it." Reed's contact information is in the Millville file. Padilla locates her. Reed is now living in a different state, has a steady job, and is willing to talk. She confirms: "Terrence was always doing dirt for Darius. Whatever Darius needed handled, Terrence handled it."

**Cell tower records** (GAP-001): Padilla serves emergency subpoenas to all four major carriers for Odom's phone records from June 2017. T-Mobile responds -- Odom was a T-Mobile customer. Records are at the edge of retention but still available. On June 17, 2017:

- 8:15 PM: Odom's phone pings tower near his apartment (where girlfriend says he was)
- 8:44 PM: Odom's phone pings tower 1.2 miles from Riverside Park
- 9:18 PM: Odom's phone pings tower serving the park area
- 9:47 PM: Odom's phone pings tower 0.8 miles south of park (moving away)
- 10:33 PM: Odom's phone pings tower near his apartment again

```
SYSTEM UPDATE:
  Odom's alibi is broken. Girlfriend claimed he was home all night. 
  Cell tower shows he left between 8:15 and 8:44 PM, was in the 
  park area at 9:18 PM (within the 9:40-9:55 PM shooting window 
  considering tower coverage radius), and returned home by 10:33 PM.

  NEW EDGES:
    - CONTRADICTS: cell_tower_odom_844pm -> alibi_girlfriend_statement
    - CORROBORATES: cell_tower_odom_918pm -> event:crime:shooting 
      (places Odom in area during shooting window)
    - TIMELINE edges tracking Odom's movement: home -> park -> home

  Updated coherence: 0.44 -> 0.73

  HYPOTHESIS: odom_shooter_for_holt
    Supporting evidence: 11 nodes
      - NIBIN: same gun used in both cases
      - Millville witness: Odom identified as gun owner
      - Cell tower: Odom at park during shooting window
      - Cell tower: Odom's alibi contradicted
      - Anonymous tip: "Terrence did it over money DeShawn owed Darius"
      - Keisha Brown: victim was meeting someone about money
      - Informant: Carter owed Holt money
      - Network analysis: Holt is bridge between groups
      - Tanya Reed (Millville witness): Odom worked for Holt
      - Witness #7: heard shots at ~9:45 PM (consistent with timeline)
      - Social media: "heard some shit" post at 9:47 PM (consistent)
    Contradicting evidence: 1 node
      - Girlfriend's alibi statement (contradicted by cell tower)
    Hypothesis coherence: 0.73
```

The case is presentable to the DA. The detective has: ballistic evidence linking Odom to the murder weapon, cell tower data placing him at the scene, an identified motive (debt collection for Holt), a cross-jurisdictional witness (Reed) identifying him as the gun's owner and Holt's enforcer, and an anonymous tip corroborated by independent physical evidence. The girlfriend's alibi is demolished by cell tower data.

The system also flags for Brady compliance: JB Williams (Suspect B) was at the park but is NOT linked to the murder weapon. If Williams was ever considered a suspect, the evidence clearing him must be disclosed. The system generates this automatically because the `Inhibits` edge (cell tower placing him at scene) combined with the absence of any `EvidenceFor` edge linking him to the ballistics creates a `brady_disclosed: false` flag that requires resolution.

---
---

## 10 Things This System Does That Nothing Else Can

| # | What It Does | Why You Care | Example |
|---|---|---|---|
| 1 | **Finds the questions nobody asked** | Every cold case has leads that were never followed -- not because the detective was lazy, but because they got sick, got transferred, got buried under 11 other cases. The system reads the entire file and surfaces every loose thread in one list. | Det. Brannigan wrote "get the badge swipe logs" on Day 5. He never did. Seven years later, the system reads his own notes and puts it at the top of your action list. Those logs named the killer. |
| 2 | **Tells you what to do NEXT, in order** | You've got a cold case file three inches thick. Where do you start? The system ranks every possible action by how much it would actually move the case forward. No guessing, no gut -- a number. | "Resubmit DNA to CODIS (costs you nothing, 12% chance of a hit that blows the case open). Do that before you spend 40 hours re-interviewing witnesses who moved to another state." |
| 3 | **Connects your case to cases in other cities automatically** | A NIBIN hit that crosses jurisdictional lines is just a piece of paper in a file unless someone picks up the phone. The system doesn't just flag the match -- it pulls the other case into your graph and shows you what their investigation found that yours didn't. | The Millville PD had a cooperating witness who named your suspect and described his gun. That witness statement sat in their file for 6 years while your case sat in yours. The system put them together in one screen. |
| 4 | **Shows you when shaky witnesses are actually telling the truth** | A witness with a pending case, an anonymous tipster, a teenager who recanted -- you know you can't put them on the stand. But the system shows you when their statements are independently backed up by cell towers, ballistics, or timestamps they couldn't have known about. That changes how you investigate. | Your anonymous tipster said "Terrence did it over money." You can't find the tipster. But NIBIN says Terrence's gun was used, cell towers say Terrence was there, and a separate witness in another city says Terrence carries that gun. The tipster was right -- and now you have the physical evidence to prove it without them. |
| 5 | **Tracks every analytical step so you don't get burned in court** | Defense attorneys love to argue that the investigation was biased, that detectives cherry-picked evidence, that exculpatory leads were ignored. Every single thing the system does -- every search, every link, every gap it flagged -- is recorded in an immutable audit log with timestamps. | "Detective, did you consider any suspects other than my client?" "Yes, counselor. The system identified 5 persons of interest, ranked them by evidence strength, and your client scored highest on 11 independent corroborating factors. Here's the log showing every step." |
| 6 | **Surfaces exculpatory evidence before the defense does** | Brady violations end careers and free guilty people. The system automatically flags when evidence points AWAY from a suspect. If you're building a case against Suspect A and the file contains untested evidence that could clear them, the system tells you before you file charges. | JB Williams was at the park the night of the murder. Cell towers confirm it. But the ballistics point to a completely different gun owner. The system flags: "Williams present at scene but not linked to murder weapon. Exculpatory for Williams. Brady disclosure required if Williams is charged." |
| 7 | **Never retires, never transfers, never forgets** | Det. Brannigan kept half the case in his head. When he retired, that knowledge walked out the door. The system holds everything -- every note, every hunch documented in the margins, every "I should follow up on this" that got written on a Post-it and lost. | Seven years after the original detective retired, the system still knows that Witness #7 mentioned a "stair regular" who was never identified. It doesn't care that it's 2026. It puts that lead in front of you like it was written yesterday. |
| 8 | **Gives your case a number the brass can understand** | When you go to your lieutenant and say "I need 40 hours on a 2017 cold case," they want to know why THIS case and not the other 200 in the cabinet. The system gives you a coherence score: 0.31 means the evidence is all over the place but there are clear leads. A counterfactual analysis says "close these 3 gaps and you're at 0.73." That's a resource request the captain can approve. | "Lieutenant, this case is at 0.31 coherence with 3 high-impact leads that haven't been touched. If I pull the badge swipe logs, resubmit DNA, and run one phone number, the model projects 0.72 coherence. I need two weeks." Compare that to "I have a feeling about this one." |
| 9 | **Lets you test a theory before you invest 100 hours in it** | You think it's Suspect A. Plug them in. The system shows you whether the evidence gets stronger or weaker. You can test every suspect in 10 minutes instead of spending months chasing one who doesn't fit. | "If Odom did it, coherence jumps from 0.44 to 0.73 -- ballistics, cell tower, witness, and motive all line up. If Williams did it, coherence drops to 0.29 -- ballistics don't match and his cell tower data puts him at the park entrance, not the boat ramp. Odom is your guy." |
| 10 | **Finds the serial pattern you didn't know existed** | Your case is one shooting. But the system searches every case in the database by method, weapon, geography, victim profile, and timing. It finds the case in Millville you never heard of. It finds the case in the next county with the same MO from two years earlier. It connects dots across jurisdictions that no single detective could see. | You had a NIBIN hit sitting in your file for 6 years. The system doesn't just flag the hit -- it pulls in the other case, finds the witness who identified the shooter, and hands you a suspect with a name, a motive, and a gun that matches your casings. Two cases, two cities, one graph. |

---

*These examples are entirely fictional. All names, case numbers, addresses, and details are invented. Any resemblance to actual cases is coincidental. The ECC system capabilities described reflect the technical architecture documented in the ecc-application-mapping.md specification.*
