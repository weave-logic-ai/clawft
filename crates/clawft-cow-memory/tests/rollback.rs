//! Rollback round-trip (integration plan §10): insert good data, checkpoint,
//! insert junk, rollback -- junk gone, pre-checkpoint data intact.

mod common;

use std::time::Instant;

use clawft_cow_memory::VectorItem;

#[test]
fn rollback_restores_pre_checkpoint_state_exactly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let good_vector = common::random_vector(&mut rng, dim);
    let good_id = mem
        .ingest(&[VectorItem::new(good_vector.clone())])
        .expect("ingest good")[0];

    let checkpoint = mem.checkpoint("pre-turn").expect("checkpoint");

    let junk_vector = common::random_vector(&mut rng, dim);
    let junk_id = mem
        .ingest(&[VectorItem::new(junk_vector.clone())])
        .expect("ingest junk")[0];

    // Sanity: junk is visible, and diff() reports it, before rollback.
    assert!(
        mem.query(&junk_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == junk_id)
    );
    assert_eq!(mem.diff().added, vec![junk_id]);

    let t0 = Instant::now();
    mem.rollback(Some(checkpoint)).expect("rollback");
    println!(
        "rollback latency = {:?} (upstream reference: ~0.57ms p50)",
        t0.elapsed()
    );

    let post_junk = mem.query(&junk_vector, 5).unwrap();
    assert!(
        !post_junk.iter().any(|r| r.id == junk_id),
        "junk vector must not survive rollback"
    );

    let post_good = mem.query(&good_vector, 5).unwrap();
    assert!(
        post_good.iter().any(|r| r.id == good_id),
        "pre-checkpoint data must survive rollback"
    );

    let diff_after = mem.diff();
    assert!(
        diff_after.added.is_empty(),
        "fresh working must have no pending adds"
    );
    assert!(
        diff_after.deleted.is_empty(),
        "fresh working must have no pending deletes"
    );

    assert_eq!(
        mem.status().own_checkpoint_depth,
        1,
        "rollback target checkpoint is kept, not discarded"
    );
}

#[test]
fn rollback_to_none_discards_the_whole_checkpoint_chain() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v1 = common::random_vector(&mut rng, dim);
    let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
    mem.checkpoint("c1").expect("checkpoint 1");

    let v2 = common::random_vector(&mut rng, dim);
    let id2 = mem.ingest(&[VectorItem::new(v2.clone())]).unwrap()[0];

    mem.rollback(None).expect("rollback to base");

    assert_eq!(
        mem.status().own_checkpoint_depth,
        0,
        "rollback(None) clears the whole chain"
    );
    assert!(
        !mem.query(&v1, 5).unwrap().iter().any(|r| r.id == id1),
        "checkpointed data is discarded too"
    );
    assert!(
        !mem.query(&v2, 5).unwrap().iter().any(|r| r.id == id2),
        "working data is discarded"
    );
}

#[test]
fn rollback_to_unknown_checkpoint_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());

    // A CheckpointId from a *different* lineage (or one already rolled
    // back/promoted away) must not silently no-op or target the wrong node.
    let other_dir = tempfile::tempdir().expect("tempdir");
    let mut other = common::new_memory(other_dir.path());
    let stray = other.checkpoint("stray").expect("checkpoint");

    let result = mem.rollback(Some(stray));
    assert!(
        result.is_err(),
        "rollback to a checkpoint id not in this lineage must fail closed"
    );
}
