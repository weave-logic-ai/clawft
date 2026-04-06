# Research Foundations: Computational Causal Analysis in Cold Case Homicide Investigation

**Prepared for**: Cold Case ECC Symposium  
**Date**: April 2026  
**Classification**: Pre-symposium research brief for law enforcement professionals

---

## 1. Current Cold Case Investigation Methods

### 1.1 How Cold Case Units Operate

Cold case units are specialized squads within law enforcement agencies that focus exclusively on unsolved cases rather than responding to new homicides. Their operational model varies by agency size and resources, but generally follows these patterns:

**Staffing Models**:
- Large metropolitan departments (500+ sworn officers) typically maintain dedicated cold case squads of 2-8 detectives
- Mid-size departments often assign cold case review as a secondary duty to active homicide detectives
- Small departments frequently lack any formal cold case capacity and rely on state-level resources or multi-jurisdictional task forces
- Many departments are staffing below authorized strength -- Orlando PD, for example, was actively seeking to hire over 100 new officers as of 2024, reflecting a broader national pattern of law enforcement staffing shortages that directly impacts cold case capacity

**Case Review Cycles**:
- Formal cold case units conduct periodic solvability assessments, typically annually or when new information surfaces
- Cases are prioritized based on solvability factors, available physical evidence (especially biological evidence suitable for DNA testing), and the likelihood that witnesses are still alive and locatable
- The National Institute of Justice recommends systematic, structured review processes rather than ad hoc case selection
- The Council on Criminal Justice and the Police Executive Research Forum (PERF) published commentary in January 2025 on scaling cold case capacity and solvability assessments, reflecting growing national attention to the problem

**Evidence Re-examination**:
- Primary trigger for case re-activation is advancement in forensic technology (DNA analysis, genetic genealogy, digital forensics)
- Secondary triggers include new witness information, deathbed confessions, jailhouse informants, and linkage to other cases
- Physical evidence must be located and verified -- chain of custody gaps are common in older cases, and evidence storage conditions vary dramatically between agencies

### 1.2 The Solvability Factors Framework

Solvability factors are criteria used by homicide detectives to assess the likelihood that a case can be cleared. While the framework is widely used, research has found that much of the available literature on cold case solvability is outdated and largely not supported by rigorous scientific or empirical data. The commonly cited factors include:

- **Witness availability**: Is there a known eyewitness? Are witnesses still alive and locatable?
- **Physical evidence**: Does biological evidence exist that can be tested with current technology? Are fingerprints, ballistics, or trace evidence available?
- **Suspect identification**: Was a suspect identified but not charged? Is there a known associates pool?
- **Motive clarity**: Is there an identifiable motive (domestic, financial, gang-related, narcotics)?
- **Documentation quality**: How thorough was the original investigation? Are reports, photos, and notes complete?
- **Victim identification**: Is the victim identified? (Unidentified victims present additional barriers)
- **Media and community attention**: Has there been sustained public interest? Are tips still coming in?
- **Forensic technology gap**: Has relevant forensic technology advanced since the original investigation?

A key limitation: solvability assessments are inherently subjective and often conducted by investigators who may lack training in systematic case evaluation. There is no nationally standardized scoring rubric, though the NIJ has published guidance on applying modern investigation methods to cold case review.

### 1.3 Why Cases Go Cold

Cases go cold for structural, evidentiary, and institutional reasons:

**Witness Attrition**:
- Witnesses die, relocate, develop memory degradation, or become uncooperative over time
- In community-violence cases, witness intimidation may have been present from the start
- Witnesses who were minors at the time may not come forward until decades later

**Evidence Degradation**:
- Biological evidence degrades if not properly stored (temperature, humidity, contamination)
- Physical evidence is lost during agency moves, storage facility changes, or purges
- Chain of custody documentation becomes incomplete or lost
- Pre-DNA-era evidence was often not collected with biological analysis in mind

**Investigator Turnover**:
- Detectives retire, transfer, or die, taking institutional knowledge with them
- Case files may be incomplete because experienced detectives kept critical information in their heads rather than in reports
- Handoffs between investigators are often poorly documented
- New detectives must spend significant time simply understanding what was already done

**Institutional Knowledge Loss**:
- Organizational restructuring can scatter case files and break continuity
- Records management system migrations can result in data loss or inaccessibility
- Informal networks of informants and community contacts are tied to individual detectives
- Retired detectives are rarely consulted in a structured way

**Resource Constraints**:
- Active caseloads consistently take priority over cold cases
- Forensic laboratory backlogs delay testing for months or years
- Budget pressures reduce staffing for specialized units first
- Training in new forensic technologies is unevenly distributed

### 1.4 Orlando PD: Local Context

**Department Overview**:
- The Orlando Police Department employs over 1,023 sworn officers and over 150 civilian employees
- The Criminal Investigations Division handles homicides, sexual assaults, and major crimes
- OPD has been actively recruiting, seeking 100+ new officers as of mid-2024, indicating staffing pressures

**Unsolved Cases**:
- OPD maintains a public-facing [Unsolved Cases page](https://www.orlando.gov/Public-Safety/OPD/OPD-Records-Open-Data/Unsolved-Cases) seeking tips on unsolved homicides
- The department works with CrimeLine (1-800-423-8477) for anonymous tips, offering monetary rewards
- Orange County broadly has seen cold case successes through DNA and genetic genealogy -- notably, in 2024, an Orlando man was arrested for two cold case murders from more than 30 years prior after investigators used DNA advancements to match old crime scene evidence to a new swab

**Orange County Sheriff's Office**:
- Maintains a separate Unresolved Homicide unit at OCSO
- Coordinates with OPD on cases that cross jurisdictional boundaries within the Orlando metro area

**Regional Resources**:
- Florida Department of Law Enforcement (FDLE) maintains a statewide [Unsolved Cases portal](https://web.fdle.state.fl.us/unsolvedcases/public/home.jsf)
- The Florida Attorney General's Cold Case Investigations Unit was established to assist local agencies facing resource constraints, consisting of dedicated investigators and attorneys
- The Florida Sheriffs Association operates a Cold Case Advisory Commission that meets quarterly to review case presentations and discuss strategies

**Florida Legislative Context**:
- SB 350 and HB 837 proposed requiring all Florida law enforcement agencies to review an unsolved murder upon written application from a victim's family member (murder must have occurred 5+ years prior)
- Required reviews would include assessing whether investigative procedures were missed, which witnesses should be re-interviewed, and whether physical evidence should undergo additional testing
- Victim rights for next of kin of homicide victims are codified in Florida Statutes Title XLVII Chapter 960

---

## 2. Existing Technology in Criminal Investigation

### 2.1 Commercial Analytical Platforms

**IBM i2 Analyst's Notebook**:
- Industry-standard link analysis and visualization tool used by law enforcement worldwide
- Enables construction of relationship diagrams connecting people, places, events, and evidence
- Supports timeline analysis and pattern-of-life visualization
- Provides predictive modeling capabilities for criminal networks and activities
- Widely deployed but requires significant training and manual data entry

**Palantir Gotham**:
- Enterprise data integration and analytics platform designed for government and law enforcement
- Fuses massive datasets from diverse sources: crime reports, surveillance feeds, intelligence databases, financial records
- Leverages AI/ML to identify crime patterns, forecast hotspots, and simulate resource deployment
- Powerful but controversial -- significant civil liberties concerns around mass surveillance capabilities
- Used by major metropolitan police departments and federal agencies

**COPLINK (now part of Forensic Logic)**:
- Designed specifically for law enforcement investigative use
- Provides tools for quickly identifying potential suspects by overcoming limitations of disparate information sources
- Includes facial recognition search capabilities
- Focuses on cross-agency data sharing and deconfliction

**Cellebrite**:
- Digital forensics platform for extracting and analyzing data from mobile devices
- Critical for cold cases where seized phones may contain call logs, messages, GPS data
- Used in conjunction with Palantir or i2 for broader analytical context

**GovTech/Emerging Platforms**:
- New AI-powered platforms specifically designed to help police close cases are entering the market
- These combine natural language processing for report analysis, link analysis, and predictive analytics
- UK police have reported significant efficiency gains from AI-assisted investigation tools -- in one case, officers used AI to read and translate over 100,000 messages on seized phones in a single day, identifying approximately 120 possible crimes

### 2.2 Federal and National Systems

**VICAP (Violent Criminal Apprehension Program)**:
- FBI-operated database maintaining approximately 85,000 cases -- the largest investigative repository of major violent crime cases in the U.S.
- Designed to identify serial patterns across jurisdictions by collecting detailed information about homicides, sexual assaults, missing persons, and unidentified remains
- Particularly valuable for identifying serial killers where victims in different jurisdictions would not otherwise be connected
- **Critical limitation identified in 2024 DOJ Inspector General audit**: Limited participation by local agencies and significant data entry backlogs reduce the program's ability to link crimes, undermining law enforcement confidence in the system
- Participation is voluntary, and many departments lack the resources or training to submit cases

**NamUs (National Missing and Unidentified Persons System)**:
- National centralized repository for missing, unidentified, and unclaimed person cases
- Operated by the National Institute of Justice with analytical support from RTI International
- Provides free forensic services: forensic odontology, fingerprint examination, forensic anthropology, and DNA analysis through the UNT Center for Human Identification
- Since inception, NamUs has helped resolve more than 46,000 cases, with over 10,000 identifications made via NamUs services
- Estimated 4,400 unidentified bodies recovered annually in the U.S., with approximately 1,000 remaining unidentified after one year
- Analytical Division provides case support using non-governmental criminal justice databases and advanced search techniques

**CODIS (Combined DNA Index System)**:
- FBI-managed national DNA database
- Contains DNA profiles from convicted offenders, arrestees, forensic evidence, and missing persons
- Generates investigative leads when crime scene DNA matches a profile in the system
- Cold case relevance: as the database grows and testing technology improves, old evidence can generate new hits

### 2.3 Genetic Genealogy

Investigative Genetic Genealogy (IGG) has become the most transformative technology for cold case resolution in the past decade:

**How It Works**:
- Crime scene DNA is processed to generate a SNP (single nucleotide polymorphism) profile using hundreds of thousands of genetic markers
- The profile is uploaded to public genealogy databases (GEDmatch, FamilyTreeDNA) to identify distant familial relationships
- Genetic genealogists build family trees from these matches to narrow down potential suspects
- Traditional investigation then confirms or eliminates candidates

**Key Databases**:
- **GEDmatch** and **FamilyTreeDNA** are the two databases that currently allow law enforcement use
- GEDmatch users must opt-in to law enforcement matching (following policy changes after privacy violations were disclosed in November 2023, when it was revealed that forensic genealogy practitioners had circumvented user privacy settings)
- The University of New Haven operates a Forensic Investigative Genetic Genealogy Program that has trained analysts who solved two cold cases in 2024

**Notable Successes**:
- The Golden State Killer case (2018) was the breakthrough that demonstrated IGG's potential
- Orange County's oldest unsolved case was resolved using GEDmatch
- In 2024, Clayton Foreman was convicted for a 1995 murder solved through genetic genealogy, receiving a life sentence
- An Apopka, Florida man was arrested in a cold case unsolved for 33 years, with police crediting DNA advancements
- Multiple cold cases from decades ago were solved in 2024 alone using these methods

**Limitations**:
- Requires sufficient quantity and quality of DNA from the crime scene
- Dependent on the size of public genealogy databases (biased toward European ancestry)
- Expensive and time-consuming genealogical analysis (weeks to months per case)
- Privacy and consent concerns remain active

### 2.4 Social Network Analysis in Criminal Investigation

The FBI Law Enforcement Bulletin has published guidance on Social Network Analysis (SNA) as a systematic approach for investigating criminal networks:

- SNA maps and measures social relations, allowing police to discover, analyze, and visualize the social networks of criminal suspects
- Quantitative metrics (centrality, betweenness, clustering coefficient) identify key actors in criminal organizations
- Co-offending networks are constructed from arrest and investigation records, connecting individuals who have committed crimes together or within similar time frames and locations
- The approach reveals collaboration patterns indicating organized criminal activity
- Network topology can be combined with phone metadata and keyword analysis, transforming a plain link graph into a rich knowledge graph

### 2.5 Academic Research on Computational Approaches

Key academic contributions to computational cold case analysis:

- Taroni et al. published foundational work on Bayesian networks and the evaluation of scientific evidence for forensic application (Institute of Forensic Sciences, University of Lausanne)
- The Alan Turing Institute maintains an active research program on "Statistics and the Law: Probabilistic Modelling of Forensic Evidence"
- A Bayesian Hierarchical Model for Criminal Investigations was published in Bayesian Analysis (2021), proposing formal probabilistic frameworks for structuring investigative reasoning
- Nature published research on using machine learning to forecast domestic homicide via police data and super learning (Scientific Reports, 2023)
- The University of South Florida launched a joint Computer Science and Criminology program (2026) combining data analytics, machine learning, and secure computing for crime analysis

---

## 3. Causal Graph Methods Applicable to Investigation

### 3.1 Bayesian Networks for Evidence Evaluation

Bayesian Networks (BNs) are the most mature probabilistic graphical method applied to forensic evidence evaluation:

**Core Architecture**:
- Directed acyclic graphs (DAGs) where nodes represent hypotheses and evidence, and directed edges represent statistical dependence and causal relationships
- Encode probabilistic relationships among variables -- nodes represent variables (e.g., "suspect was at the scene," "DNA matches suspect") and arcs represent causal or influential relationships
- Allow calculation of marginal and conditional probabilities given observed evidence

**Forensic Applications Already in Use**:
- DNA mixture interpretation (widely accepted in courts)
- Forensic genetics analysis leveraging the natural causal structure of DNA inheritance (Dawid et al.)
- Glass fragment transfer evidence
- Gunshot residue analysis
- Document examination

**Advantages for Cold Case Investigation**:
- Mathematically and statistically rigorous handling of uncertainty
- Graphical structure facilitates communication between forensic scientists, lawyers, judges, and juries
- Supports training in probabilistic reasoning during criminal investigation
- Can be updated incrementally as new evidence is discovered
- Enables formal sensitivity analysis -- "what would change if we obtained this piece of evidence?"

**Scenario-Based Reasoning**:
- Researchers have proposed combining Bayesian networks with narrative scenarios for case evaluation
- In criminal trials, judges and juries reason about what happened based on available evidence
- A probabilistic approach suits statistical evidence analysis, while narrative approaches may be preferred for holistic case consideration
- The combination allows formal probability calculations within human-interpretable storylines

**Court Acceptance**:
- Bayesian methods are increasingly accepted for specific forensic domains (DNA, particularly)
- Broader adoption for case-level reasoning faces resistance from legal professionals unfamiliar with probabilistic thinking
- The challenge is presentation: jurors and judges must understand what the numbers mean without being misled

### 3.2 Causal DAGs for Event Sequence Reconstruction

Causal Directed Acyclic Graphs provide formal machinery for reconstructing "what happened":

**Event Chain Modeling**:
- Each node represents an event or state (victim leaves work, suspect arrives at location, phone call placed, gunshot heard, 911 called)
- Directed edges encode temporal ordering and causal influence
- Missing nodes -- events that must have occurred but for which there is no direct evidence -- can be inferred from the graph structure
- Conditional independence properties encoded in the DAG structure allow principled reasoning about what evidence is relevant to what hypotheses

**Temporal Priority in Causal Inference**:
- Causal discovery from temporal data relies on the assumption that causes precede effects
- For criminal investigation, this maps directly to timeline reconstruction
- When the temporal ordering of events is uncertain, probabilistic temporal logic (PTL) extends traditional temporal logic by assigning probabilities to propositions rather than treating them as purely true or false
- This allows representation of statements like "the suspect was probably at location X between 8 PM and 10 PM" with formal uncertainty quantification

**Counterfactual Reasoning**:
- Causal DAGs support counterfactual analysis: "Would the victim have died if the suspect had not been present?"
- This aligns with legal standards of causation (but-for causation, proximate cause)
- Enables systematic exploration of alternative hypotheses

### 3.3 Knowledge Graphs for Entity-Relationship Mapping

Knowledge graphs are the natural data structure for representing the complex web of relationships in a homicide investigation:

**Entity Types in a Cold Case Knowledge Graph**:
- **Persons**: victims, suspects, witnesses, associates, family members, informants, investigating officers
- **Locations**: crime scene(s), suspect addresses, witness locations, vehicle locations, cell tower coverage areas
- **Evidence**: physical items (weapons, clothing, biological samples), documents, digital records
- **Events**: the crime itself, prior incidents, subsequent events, investigative actions
- **Organizations**: gangs, businesses, law enforcement agencies, forensic laboratories
- **Temporal markers**: dates, times, durations, sequences

**Relationship Types**:
- Person-to-person: knows, related to, associated with, employed by, rival of, communicated with
- Person-to-location: resides at, works at, was seen at, frequents
- Person-to-evidence: collected from, belongs to, handled by, tested by
- Event-to-event: preceded by, caused by, coincident with
- Evidence-to-location: found at, originated from, transported to

**Implementation in Practice**:
- Graph databases (Neo4j, Memgraph, Amazon Neptune) are optimized for relationship-first queries
- Unlike relational databases that force data into rows and columns, graph databases are purpose-built for mapping criminal networks
- Network topology combined with phone metadata, financial records, and keyword analysis transforms plain link graphs into rich knowledge graphs
- Cognyte and similar vendors offer knowledge graph software specifically designed for law enforcement
- GraphAware has published work on combining knowledge graphs with LLMs to speed up criminal network analysis

### 3.4 Temporal Reasoning Under Uncertainty

Representing "what happened when" with formal uncertainty is critical for cold case analysis where timelines are often imprecise:

**Probabilistic Temporal Logic (PTL)**:
- Extends traditional temporal logic by incorporating probability
- Propositions are assigned probabilities rather than being purely true or false
- Captures relationships like "Event A typically happens before Event B, but not always"
- Supports reasoning about temporal gaps where no evidence exists

**Allen's Interval Algebra**:
- Formal system for representing temporal relationships between events (before, after, during, overlaps, meets, starts, finishes, equals)
- Naturally handles events with duration rather than point-in-time events
- Can be combined with probabilistic frameworks to handle uncertain temporal boundaries

**Dynamic Causal Models**:
- Learn sparse directed causal networks from multivariate time series
- Enable dynamic estimation of causal influence without requiring pre-specified network structure
- Applicable to analyzing sequences of communications, movements, and transactions

### 3.5 Gap Analysis: Identifying What Is Missing

Perhaps the most valuable application of formal graph methods to cold case investigation is systematic identification of what is NOT in the evidence:

**Structural Gap Detection**:
- A well-formed knowledge graph has expected patterns: suspects should have alibis that are confirmed or refuted, physical evidence should have chain-of-custody documentation, witnesses should have statements
- When these expected nodes or edges are missing, the graph reveals investigative gaps
- Automated gap detection can flag: uninterviewed witnesses, untested physical evidence, unexplored leads, missing forensic reports

**Inference of Hidden Entities**:
- Graph patterns can suggest the existence of unidentified persons (e.g., a communication pattern suggests an intermediary who was never identified)
- Missing links between known entities may indicate relationships that were never investigated
- Temporal gaps in a suspect's known movements suggest unaccounted-for time periods

**Prioritization of Investigative Actions**:
- Information-theoretic measures can rank which missing piece of evidence would most change the probability of case resolution
- This transforms cold case review from "where do we start?" to a principled decision about which lead to pursue first
- Expected value of information calculations can guide resource allocation

---

## 4. Data Sources Available for Cold Case Analysis

### 4.1 Law Enforcement Records

**Computer-Aided Dispatch (CAD)**:
- Records of all calls for service, including date/time, location, nature of call, responding units, and disposition
- CAD records often contain the initial report of the crime and first responder observations
- Historical CAD data may reveal prior calls to the same address or involving the same parties

**Records Management System (RMS)**:
- Central repository for all case information: reports, evidence logs, citations, and associated documents
- A quality RMS integrates with CAD, transferring initial call data directly into the case file
- Major vendors: Versaterm, Axon, CivicEye, EFORCE, SmartCOP
- Versaterm's system can visually display linkages between people and assets, including known associates, phone numbers, and previous incident reports
- **Critical limitation**: Historical records may exist in legacy systems, paper files, or migrated databases with data quality issues

**Witness Statements and Interview Transcripts**:
- Original written statements, audio recordings, and (more recently) video recordings
- Quality varies enormously -- some departments had verbatim transcription, others relied on detective summaries
- Older cases may have only handwritten notes
- Re-interview opportunities diminish as witnesses age, relocate, or die

### 4.2 Forensic Evidence

**Physical Evidence Logs and Chain of Custody**:
- Detailed records of evidence collection, storage, transfer, and testing
- ISO/IEC 27037:2012 provides guidance for identification, collection, acquisition, and preservation
- Chain of custody documentation is legally required -- any break can render evidence inadmissible
- Cold case challenge: older evidence may have incomplete chain documentation by modern standards

**Forensic Reports**:
- **Autopsy/Medical Examiner**: cause and manner of death, wound patterns, toxicology, time of death estimation
- **Ballistics**: weapon identification, bullet trajectory, cartridge case comparison (NIBIN database)
- **DNA**: STR profiles, touch DNA, mixture interpretation, genealogical SNP profiles
- **Fingerprints**: latent print analysis, AFIS/NGI database searches (can be re-run as database grows)
- **Toxicology**: presence of drugs, alcohol, poisons
- **Trace evidence**: fibers, hair, paint, glass, soil
- **Digital forensics**: device extraction, data recovery from degraded media

### 4.3 Court and Criminal History Records

- Prior convictions and arrest records for suspects and associates
- Court filings, motions, and transcripts from related proceedings
- Probation and parole records (may contain updated addresses, associate information)
- Protective orders and civil filings involving relevant parties
- Grand jury materials (sealed, but may be accessible for active investigation)

### 4.4 Communications and Digital Data

**Cell Phone Records and Tower Data**:
- Call Detail Records (CDR): numbers called/received, date/time, duration, cell tower used
- Historical tower data can establish approximate location during relevant time periods
- Older cases may lack cell data entirely (pre-mobile era) or have limited tower density
- Preservation requests to carriers have time limits -- historical data may be permanently lost

**Surveillance Footage**:
- Traffic cameras, business security cameras, ATM cameras
- Rarely preserved for more than 30-90 days unless specifically requested
- Cold case relevance: footage logs documenting what was reviewed (even if footage itself is gone) may contain useful information

**Social Media and Internet Records**:
- Historical social media posts, messages, check-ins (platforms may not retain data indefinitely)
- Archived web content (Wayback Machine, cached pages)
- Email records (subject to provider retention policies and legal process requirements)

### 4.5 Geographic and Environmental Data

**Spatial Data**:
- Crime scene location (GPS coordinates, photographs, sketches)
- Suspect and witness addresses at time of crime
- Movement patterns (if available from cell data, surveillance, or witness accounts)
- Proximity analysis: distance between key locations, travel time estimates

**Weather Data**:
- National Weather Service historical records for timeline verification
- Relevant for evaluating witness statements ("it was raining that night") and evidence condition
- Can affect time-of-death estimates and evidence preservation

### 4.6 Media and Public Records

**Newspaper and Media Coverage**:
- Sometimes contains details not in police files (reporter observations, quotes from witnesses who later became uncooperative)
- Community context and contemporaneous public reaction
- May identify witnesses who were not formally interviewed

**911 Call Recordings and Transcripts**:
- Original caller statements, background sounds, caller demeanor
- Time-stamped and location-tagged
- May capture information the caller did not repeat to responding officers

**Tip Line Records**:
- CrimeLine and similar anonymous tip databases
- Historical tips that were not followed up due to resource constraints
- Patterns in tip content may become relevant in light of new information

---

## 5. Legal and Ethical Considerations

### 5.1 Chain of Custody for Digitized Evidence

When physical evidence from cold cases is digitized for computational analysis, maintaining evidentiary integrity is paramount:

**Core Requirements**:
- A detailed, chronological record documenting the entire lifecycle of digital evidence from collection to court presentation
- Must show who collected or analyzed the evidence, when, and under what circumstances
- NIST guidelines require documentation of why evidence transfers occur
- Hash verification (MD5, SHA-256) at each stage of handling to prove evidence was not altered

**Standards and Frameworks**:
- ISO/IEC 27037:2012: identification, collection, acquisition, and preservation of digital evidence
- ISO 27050 series: electronic discovery processes
- Write-blocking hardware must be used when creating digital copies
- Secure, access-controlled storage with audit logging

**Cold Case Implications**:
- Digitizing decades-old paper files, photographs, and physical evidence creates a new chain of custody layer
- The digitization process itself must be documented: who scanned what, when, using what equipment, with what quality controls
- If original physical evidence has chain-of-custody gaps, digitized copies inherit those gaps
- AI/ML analysis of digitized evidence must be documented as a new analytical step in the chain

### 5.2 Admissibility of AI-Assisted Analysis in Court

This is a rapidly evolving area of law with significant recent developments:

**Proposed Federal Rule of Evidence 707**:
- Proposed in November 2024, with a revised version released for public comment through February 2026
- Would subject "machine-generated evidence" to the same admissibility standard as expert testimony
- Proponent must show: (1) AI output is based on sufficient facts or data, (2) produced through reliable principles and methods, and (3) demonstrates reliable application of those methods to the facts
- Effectively extends the Daubert standard to AI-generated evidence

**Current Legal Framework**:
- No specific federal rule currently governs AI evidence -- courts apply existing frameworks (Daubert, Frye, FRE 702)
- The standard for AI evidence admissibility can be no less than the standard for any other forensic evidence
- Judges must assess how evidence was created, whether it can be verified, and whether it contributes to fairness
- The National Center for State Courts published a guide for judges on AI-generated evidence

**State-Level Action**:
- Louisiana became the first state (August 1, 2025) to establish a framework for AI-generated evidence
- California SB 11 would direct the Judicial Council to review how AI affects evidence admissibility by January 2027
- Proposed FRE 901(c) addresses "potentially fabricated or altered electronic evidence"

**Practical Implications for Cold Case Analysis**:
- AI-assisted analysis should be treated as a tool that generates leads, not as standalone evidence
- The human investigator's independent verification of AI-generated findings is what creates admissible evidence
- Documentation of AI methodology, training data, error rates, and limitations will be required
- Transparency about what the AI did and did not do is essential for Daubert hearings

### 5.3 Brady Obligations and Exculpatory Evidence

Brady v. Maryland (1963) requires prosecutors to disclose all material evidence favorable to the defendant. AI-assisted investigation creates new Brady dimensions:

**The Algorithm Disclosure Problem**:
- When law enforcement uses algorithms to identify, investigate, or prosecute a suspect, trade secrecy protections may conflict with Brady disclosure obligations
- The Michigan Law Review published "The Missing Algorithm," arguing that constitutional protections must prevail over intellectual property concerns
- Police departments should disclose AI use in police reports to ensure prosecutors can meet Brady obligations
- NYU Law Review published "Big Data and Brady Disclosures" (2024) analyzing disclosure requirements for data-driven investigations

**Exculpatory Potential of Computational Analysis**:
- A properly constructed causal graph may reveal alternative suspects or exonerating evidence
- Gap analysis may identify leads that, if pursued, could establish innocence
- Brady requires disclosure of these analytical outputs if they favor the defendant
- AI systems that generate probability scores for suspect identification must also disclose low-confidence results and alternative candidates

**Practical Requirements**:
- AI-generated results should never serve as the sole basis for arrests, searches, or other rights-affecting actions
- Traditional investigative methods must precede action on algorithmic suggestions to comply with probable cause requirements
- All AI tool configurations, inputs, outputs, and analytical decisions should be preserved and discoverable

### 5.4 Privacy Concerns with Data Aggregation

**Scope of Concern**:
- Aggregating data from police reports, social media, phone records, financial records, and surveillance footage creates comprehensive profiles of individuals -- including people who are not suspects
- Palantir and similar platforms have drawn criticism for enabling mass surveillance capabilities
- The Brennan Center for Justice has published research on the dangers of unregulated AI in policing

**Genetic Privacy**:
- GEDmatch's 2023 disclosure that forensic genealogy practitioners circumvented user privacy settings highlights ongoing consent issues
- Genetic genealogy searches implicate the privacy of the suspect's biological relatives, who have not consented to investigation
- The Fourth Amendment implications of genealogical searching remain unsettled

**Data Minimization**:
- Any computational system should process only data relevant to the investigation
- Access controls must limit who can view aggregated data and for what purpose
- Data retention policies should govern how long aggregated analytical products are kept

### 5.5 Bias in Algorithmic Investigation Tools

**Documented Disparities**:
- Commercial facial recognition systems show error rates of 0.8% for light-skinned men but 34.7% for darker-skinned women
- A NIST report found that African American and Asian faces were between 10 and 100 times more likely to be misidentified than white male faces
- Predictive policing tools trained on biased historical data can perpetuate and amplify existing disparities

**Mitigation Requirements**:
- The Council on Criminal Justice released a framework for criminal justice agencies to assess AI tools
- Regular bias audits of any algorithmic system used in investigation
- Human review of all AI-generated leads before investigative action
- Diverse training data and validation across demographic groups
- Transparency about known limitations and error rates

### 5.6 Florida-Specific Legal Considerations

- Florida Statutes Title XLVII Chapter 960 codifies victim rights, including rights of next of kin of homicide victims
- Proposed legislation (SB 350 / HB 837) would create a statutory right for family members to request cold case review
- The Florida Attorney General's Cold Case Investigations Unit operates under state legal authority
- Florida follows the Daubert standard for expert testimony admissibility (adopted 2013, upheld by Florida Supreme Court in 2023)
- Florida's public records law (Sunshine Law) may affect accessibility of investigative records, though active investigation exemptions apply

---

## 6. Academic Programs, Experts, and Key Researchers

### 6.1 Northwestern University Connection

Northwestern University's Pritzker School of Law offers coursework in Crime and Criminology, examining theories of crime, measurement methodology, and criminal justice policy. The university's broader research ecosystem -- including the McCormick School of Engineering and the Institute for Policy Research -- provides interdisciplinary capacity for computational approaches to criminal justice problems.

**Key connection**: We have a relationship with a retired homicide detective who served as a professor at Northwestern. This connection provides:
- Direct practitioner experience with cold case investigation methodology
- Academic credibility bridging law enforcement and research communities
- Potential for validating computational approaches against real investigative workflow
- Access to Northwestern's network of criminal justice researchers and practitioners

This connection is significant because the gap between academic computational methods and actual investigative practice is one of the primary barriers to adoption. A practitioner-academic who has operated in both worlds can evaluate whether proposed technical solutions address real investigative needs.

### 6.2 Leading Academic Programs in Computational Criminology

**University of South Florida (USF)**:
- Launched a joint Bachelor of Science in Computer Science and Criminology (2026) between the College of Behavioral and Community Sciences and the Bellini College of Artificial Intelligence, Cybersecurity and Computing
- Curriculum covers data analytics, machine learning, and secure computing applied to cybercrime, fraud detection, and digital evidence analysis
- One of the first programs to formally merge CS and criminology at the undergraduate level

**Georgia State University -- Criminal Justice and Criminology**:
- Offers courses in Introduction to Computational Social Science and Data Visualization
- Focuses on how data illuminates human experiences: movement, communication, organization, and decision-making
- Electives in GIS mapping, text mining of public discourse, and machine learning for detecting systemic patterns

**University of Texas at Dallas -- Criminology**:
- Research-oriented MS in Criminology with advanced training in quantitative and qualitative methodologies
- Strong emphasis on crime measurement, justice policy evaluation, and empirical research design

**University of Maryland, College Park**:
- Department of Criminology and Criminal Justice consistently ranked among the top in the nation
- Active research programs in crime mapping, spatial analysis, and quantitative criminology

**University of Pennsylvania**:
- Jerry Lee Center of Criminology and Justice Policy
- Focus on evidence-based crime prevention and the systematic evaluation of criminal justice interventions

**Carnegie Mellon University**:
- While not a traditional criminology program, CMU's work in machine learning, causal inference, and social network analysis has direct applicability
- Peter Spirtes and Clark Glymour's foundational work on causal discovery algorithms originated at CMU

### 6.3 Key Researchers and Research Groups

**Bayesian Networks and Forensic Evidence**:
- **Franco Taroni** (University of Lausanne, Institute of Forensic Science): Foundational work on Bayesian networks for forensic evidence evaluation; published extensively on probabilistic reasoning in forensic science
- **Colin Aitken** (University of Edinburgh): Statistics in forensic science, evidence evaluation methodology
- **The Alan Turing Institute** (London): Active research program on "Statistics and the Law: Probabilistic Modelling of Forensic Evidence"
- **A. Philip Dawid** (University of Cambridge): Introduced use of object-oriented Bayesian networks for forensic genetics analysis

**Computational Criminology**:
- **Patricia Brantingham and Paul Brantingham** (Simon Fraser University): Pioneers of computational criminology and crime pattern theory
- The Brantinghams' environmental criminology work provides the theoretical foundation for spatial analysis in criminal investigation

**Causal Inference**:
- **Judea Pearl** (UCLA): Developer of the causal DAG framework; his do-calculus provides the mathematical foundation for interventional reasoning from observational data
- **Peter Spirtes** (Carnegie Mellon): PC algorithm for causal discovery from data; foundational for automated causal structure learning

**Genetic Genealogy**:
- **CeCe Moore**: Widely recognized genetic genealogist who has worked with law enforcement on over 200 cases; featured in ABC's "The Genetic Detective"
- **Colleen Fitzpatrick**: Co-founder of the DNA Doe Project; forensic genealogy pioneer
- **University of New Haven**: Operates the Forensic Investigative Genetic Genealogy Program, training analysts who have directly solved cold cases

**Knowledge Graphs and Criminal Networks**:
- **GraphAware**: Published applied research on combining knowledge graphs with LLMs for criminal network analysis
- **Memgraph**: Developed graph database applications specifically for crime-fighting and criminal network mapping
- Researchers at various institutions have published on knowledge graph frameworks for investigating crime on social networks (Springer, WISE conference proceedings)

### 6.4 Professional Organizations and Training Resources

- **National Institute of Justice (NIJ)**: Primary federal research funder for criminal justice technology; publishes guidance on modern cold case investigation methods
- **Bureau of Justice Assistance (BJA)**: Hosts training on cold case solvability and forensic science applications
- **International Association of Chiefs of Police (IACP)**: Professional development and best practices for investigative units
- **National Center for Justice Training and Technical Assistance (NCJTC)**: Offers specific courses on "Investigation and Prosecution of Cold Case Homicides"
- **Project Cold Case**: Nonprofit maintaining a national database of unsolved homicide cases; advocacy for victims' families
- **Murder Accountability Project**: Tracks national homicide clearance data and identifies potential serial killing patterns through statistical analysis

---

## 7. The Scale of the Problem: Why This Matters

The statistics frame the urgency:

- **250,000+** unsolved murder cases (cold cases) in the United States as of 2024
- **352,390** cases of homicide and non-negligent manslaughter went unsolved from 1965 to 2024 (FBI UCR data)
- The national homicide clearance rate improved to approximately 61.4% in 2024, up from 57.8% in 2023 -- but this still means roughly **4 in 10 murders go unsolved each year**
- The U.S. clearance rate lags far behind comparable nations (Germany clears over 90% of homicides)
- Each unsolved case represents a failure of justice for the victim, ongoing trauma for surviving family members, and a perpetrator who remains free

The gap between available computational methods and their actual application in cold case investigation is enormous. The tools described in this document -- Bayesian networks, causal DAGs, knowledge graphs, temporal reasoning, gap analysis -- are mature in academic settings but barely penetrate routine investigative practice. The barrier is not primarily technological; it is the translation gap between researchers who build these tools and investigators who could use them.

This symposium exists to begin closing that gap.

---

## Sources

### Cold Case Investigation Methods
- [Center for Improving Investigations - Cold Case Investigations](https://centerforimprovinginvestigations.org/cold-case-investigations/)
- [EBSCO Research Starters - Cold Cases](https://www.ebsco.com/research-starters/law/cold-cases)
- [ProQuest - Solvability Factors in 21st Century Cold Case Investigation](https://www.proquest.com/openview/b4ad6224713cf30edb31d27a3ceebbf7/1?pq-origsite=gscholar&cbl=18750&diss=y)
- [Future Policing Institute - New and Emerging Trends in Cold-Case Homicide Investigations](https://www.futurepolicing.org/blog/new-and-emerging-trends-in-cold-case-homicide-investigations)
- [ResearchGate - Cold Case: Factors That Promote Case Solvability](https://www.researchgate.net/publication/365315678_COLD_CASE_Factors_That_Promote_Case_Solvability)
- [NIJ - Applying Modern Investigation Methods to Solve Cold Cases](https://nij.ojp.gov/topics/articles/applying-modern-investigation-methods-solve-cold-cases)
- [Policing Institute - Prioritizing Cold-Case Murders](https://www.policinginstitute.org/onpolicing/prioritizing-cold-case-murders-what-law-enforcement-executives-can-do/)

### Orlando and Florida
- [City of Orlando - Unsolved Cases](https://www.orlando.gov/Public-Safety/OPD/OPD-Records-Open-Data/Unsolved-Cases)
- [Orange County Sheriff's Office - Unresolved Homicide](https://www.ocso.com/en-us/Crime-Information/Unresolved-Homicide)
- [FOX 35 Orlando - Orlando Man Arrested 3 Decades Later on 2 Cold Case Murders](https://www.fox35orlando.com/news/orlando-man-arrested-30-years-later-on-2-cold-case-murders.amp)
- [Florida Attorney General - Cold Case Investigations Unit](https://www.myfloridalegal.com/cold-case-investigations-unit)
- [Florida Sheriffs Association - Cold Case Advisory Commission](https://flsheriffs.org/law-enforcement-programs/cold-case-review-advisory-commission)
- [Florida Politics - Cold Case Legislation](https://floridapolitics.com/archives/648997-cold-case-legislation-would-ease-calls-for-reinvestigations-of-unsolved-murders/)
- [FDLE - Unsolved Cases](https://web.fdle.state.fl.us/unsolvedcases/public/home.jsf)

### Technology in Criminal Investigation
- [IBM Redbooks - Accelerating Law Enforcement Investigations](https://www.redbooks.ibm.com/redpapers/pdfs/redp5353.pdf)
- [GovTech - Can a New AI-Powered Platform Help Police Close Cases?](https://www.govtech.com/artificial-intelligence/can-a-new-ai-powered-platform-help-police-close-cases)
- [Brennan Center for Justice - The Dangers of Unregulated AI in Policing](https://www.brennancenter.org/our-work/research-reports/dangers-unregulated-ai-policing)
- [FBI - ViCAP](https://www.fbi.gov/wanted/vicap)
- [DOJ OIG - Audit of the FBI's Violent Criminal Apprehension Program (2024)](https://oig.justice.gov/sites/default/files/reports/24-078.pdf)
- [NamUs](https://namus.nij.ojp.gov/)
- [NIJ - National Missing and Unidentified Persons System](https://nij.ojp.gov/namus)

### Genetic Genealogy
- [GEDmatch - Orange County's Oldest Case Solved Using DNA](https://www.gedmatch.com/community-safety/orange-countys-oldest-case-solved-using-dna/)
- [University of New Haven - FIGG Program Alumni Solve Cold Cases](https://www.newhaven.edu/news/blog/2024/figg-cold-case.php)
- [ABC News - Cold Cases Solved with Cutting-Edge DNA Technology](https://abcnews.go.com/US/cold-cases-baffled-investigators-solved-cutting-edge-dna/story?id=119404745)
- [BBC Science Focus - Forensic Genealogy](https://www.sciencefocus.com/the-human-body/forensic-genealogy-how-police-are-using-family-trees-to-solve-cold-cases)
- [Psychology Today - Five Cold Cases Solved in 2024](https://www.psychologytoday.com/us/blog/crime-she-writes/202412/five-cold-cases-solved-in-2024-because-of-new-technology)

### Bayesian Networks and Causal Methods
- [Taroni et al. - Bayesian Networks and the Evaluation of Scientific Evidence](https://arch.ies.gov.pl/images/PDF/2001/vol_46/46_taroni.pdf)
- [Springer - A Method for Explaining Bayesian Networks for Legal Evidence with Scenarios](https://link.springer.com/article/10.1007/s10506-016-9183-4)
- [Alan Turing Institute - Statistics and the Law](https://www.turing.ac.uk/research/research-projects/statistics-and-law-probabilistic-modelling-forensic-evidence)
- [Bayesian Analysis - A Bayesian Hierarchical Model for Criminal Investigations](https://projecteuclid.org/journals/bayesian-analysis/volume-16/issue-1/A-Bayesian-Hierarchical-Model-for-Criminal-Investigations/10.1214/19-BA1192.pdf)
- [Springer - Using Bayesian Networks to Guide Assessment of New Evidence in an Appeal Case](https://link.springer.com/article/10.1186/s40163-016-0057-6)

### Knowledge Graphs and Network Analysis
- [FBI Law Enforcement Bulletin - Social Network Analysis: A Systematic Approach for Investigating](https://leb.fbi.gov/articles/featured-articles/social-network-analysis-a-systematic-approach-for-investigating)
- [Cognyte - Knowledge Graph Software for Law Enforcement](https://www.cognyte.com/blog/knowledge-graph-software/)
- [Memgraph - Graph Databases for Crime-Fighting](https://memgraph.com/blog/graph-databases-crime-fighting-memgraph-criminal-networks)
- [GraphAware - Speed Up Criminal Network Analysis with LLMs and Knowledge Graphs](https://graphaware.com/blog/combine-knowledge-graphs-and-llms-to-speed-up-crime-analysis/)

### Legal and Ethical
- [Justice Speakers Institute - AI on Trial: Admissibility of AI-Generated Evidence](https://justicespeakersinstitute.com/ai-generated-evidence-admissibility-on-trial/)
- [National Law Review - Proposed FRE 707 on AI-Generated Evidence](https://natlawreview.com/article/new-evidence-rule-707-would-set-standards-ai-generated-courtroom-evidence)
- [NCSC - AI-Generated Evidence: A Guide for Judges](https://www.ncsc.org/resources-courts/ai-generated-evidence-guide-judges)
- [Michigan Law Review - The Missing Algorithm](https://michiganlawreview.org/journal/the-missing-algorithm/)
- [NYU Law Review - Big Data and Brady Disclosures](https://nyulawreview.org/wp-content/uploads/2024/11/99-NYU-L-Rev-1754.pdf)
- [Jones Walker - AI Police Surveillance Bias](https://www.joneswalker.com/en/insights/blogs/ai-law-blog/ai-police-surveillance-bias-the-minority-report-impacting-constitutional-right.html)
- [Council on Criminal Justice - AI Assessment Framework](https://counciloncj.org/assessing-ai-for-criminal-justice-a-user-decision-framework/)

### Statistics and Scale
- [Project Cold Case - Cold Case Homicide Stats](https://projectcoldcase.org/cold-case-homicide-stats/)
- [Murder Accountability Project](https://www.murderdata.org/)
- [NPR - U.S. Unsolved Murder Rates at Record High](https://www.npr.org/2023/04/29/1172775448/people-murder-unsolved-killings-record-high)
- [CBS News - Nearly Half of U.S. Murders Go Unsolved](https://www.cbsnews.com/news/unsolved-murders-crime-without-punishment/)

### Academic Programs
- [Wikipedia - Computational Criminology](https://en.wikipedia.org/wiki/Computational_criminology)
- [USF - BS in Computer Science and Criminology](https://www.usf.edu/ai-cybersecurity-computing/academics/undergraduate/cs-and-criminology.aspx)
- [Chain of Custody in Digital Forensics (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC10000967/)
- [AMU - How to Maintain Chain of Custody for Digital Forensic Evidence](https://www.amu.apus.edu/area-of-study/criminal-justice/resources/how-to-maintain-chain-of-custody-for-digital-forensic-evidence/)
