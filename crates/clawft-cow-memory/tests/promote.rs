//! Promote round-trip (integration plan §10): a turn's checkpoint chain
//! (plus working) collapses into `base`; the checkpoint chain and old
//! `working` are discarded; a fresh `working` is derived from the updated
//! `base`.

mod common;

use std::time::Instant;

use clawft_cow_memory::VectorItem;

#[test]
fn promote_moves_working_and_checkpoints_into_base() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v1 = common::random_vector(&mut rng, dim);
    let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
    mem.checkpoint("c1").expect("checkpoint");

    let v2 = common::random_vector(&mut rng, dim);
    let id2 = mem.ingest(&[VectorItem::new(v2.clone())]).unwrap()[0];

    let t0 = Instant::now();
    let report = mem.promote().expect("promote");
    println!(
        "promote latency = {:?}, ingested={}, deleted={} (upstream reference: ~897µs)",
        t0.elapsed(),
        report.ingested,
        report.deleted
    );
    assert_eq!(
        report.ingested, 2,
        "one from the checkpoint, one from working"
    );
    assert_eq!(report.deleted, 0);

    assert_eq!(
        mem.status().own_checkpoint_depth,
        0,
        "checkpoint chain collapses into base"
    );

    let diff = mem.diff();
    assert!(
        diff.added.is_empty() && diff.deleted.is_empty(),
        "fresh working has nothing pending"
    );

    assert!(mem.query(&v1, 5).unwrap().iter().any(|r| r.id == id1));
    assert!(mem.query(&v2, 5).unwrap().iter().any(|r| r.id == id2));

    assert_eq!(
        mem.status().base.total_vectors,
        2,
        "both vectors now live directly in base"
    );
}

#[test]
fn promote_replays_tombstones_into_base() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v1 = common::random_vector(&mut rng, dim);
    let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
    mem.promote().expect("first promote lands v1 in base");
    assert_eq!(mem.status().base.total_vectors, 1);

    // A later turn deletes it -- visible immediately via tombstone masking,
    // even before the delete itself is promoted.
    mem.delete(&[id1]).expect("delete");
    assert!(
        !mem.query(&v1, 5).unwrap().iter().any(|r| r.id == id1),
        "tombstone masks base before promote"
    );

    let report = mem.promote().expect("promote the delete");
    assert_eq!(report.deleted, 1);
    assert!(
        !mem.query(&v1, 5).unwrap().iter().any(|r| r.id == id1),
        "id must stay hidden after the delete is promoted into base itself"
    );
}
