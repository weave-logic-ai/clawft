//! Isolation invariant (integration plan §10, "freeze-then-derive-both
//! guarantee", `index.js:359-379`): after `branch`/`fork`, writes to the
//! child are invisible to the parent and vice versa, while the child still
//! sees everything the parent saw as of the fork point.

mod common;

use std::time::Instant;

use clawft_cow_memory::VectorItem;

#[test]
fn fork_sees_parent_history_but_writes_stay_isolated() {
    let parent_dir = tempfile::tempdir().expect("tempdir");
    let fork_dir = tempfile::tempdir().expect("tempdir");
    let mut parent = common::new_memory(parent_dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let shared_vector = common::random_vector(&mut rng, dim);
    let shared_id = parent
        .ingest(&[VectorItem::new(shared_vector.clone())])
        .unwrap()[0];
    parent
        .promote()
        .expect("land shared vector in parent's base before forking");

    let t0 = Instant::now();
    let mut child = parent.branch(fork_dir.path()).expect("branch");
    println!("branch latency = {:?}", t0.elapsed());

    assert!(
        child
            .query(&shared_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == shared_id),
        "fork must see pre-fork history"
    );

    let child_vector = common::random_vector(&mut rng, dim);
    let child_id = child
        .ingest(&[VectorItem::new(child_vector.clone())])
        .unwrap()[0];
    assert!(
        child
            .query(&child_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == child_id)
    );
    assert!(
        !parent
            .query(&child_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == child_id),
        "fork writes must never reach the parent"
    );

    let parent_vector = common::random_vector(&mut rng, dim);
    let parent_id = parent
        .ingest(&[VectorItem::new(parent_vector.clone())])
        .unwrap()[0];
    assert!(
        parent
            .query(&parent_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == parent_id)
    );
    assert!(
        !child
            .query(&parent_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == parent_id),
        "parent writes made after the fork must never reach the (already independent) child"
    );
}

#[test]
fn fork_from_specific_checkpoint_excludes_later_history() {
    let parent_dir = tempfile::tempdir().expect("tempdir");
    let fork_dir = tempfile::tempdir().expect("tempdir");
    let mut parent = common::new_memory(parent_dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let early_vector = common::random_vector(&mut rng, dim);
    let early_id = parent
        .ingest(&[VectorItem::new(early_vector.clone())])
        .unwrap()[0];
    let checkpoint = parent.checkpoint("early").expect("checkpoint");

    let late_vector = common::random_vector(&mut rng, dim);
    let late_id = parent
        .ingest(&[VectorItem::new(late_vector.clone())])
        .unwrap()[0];

    let child = parent
        .fork(checkpoint, fork_dir.path())
        .expect("fork from checkpoint");

    assert!(
        child
            .query(&early_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == early_id),
        "clone-of-clone: fork sees everything at/before the checkpoint it forked from"
    );
    assert!(
        !child
            .query(&late_vector, 5)
            .unwrap()
            .iter()
            .any(|r| r.id == late_id),
        "fork must not see history newer than its fork point"
    );
}

#[test]
fn fork_from_unknown_checkpoint_fails_closed() {
    let parent_dir = tempfile::tempdir().expect("tempdir");
    let fork_dir = tempfile::tempdir().expect("tempdir");
    let parent = common::new_memory(parent_dir.path());

    let other_dir = tempfile::tempdir().expect("tempdir");
    let mut other = common::new_memory(other_dir.path());
    let stray = other.checkpoint("stray").expect("checkpoint");

    assert!(parent.fork(stray, fork_dir.path()).is_err());
}
