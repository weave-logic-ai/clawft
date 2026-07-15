//! AutoPause/AutoResume (fork-adoption plan §1 Phase 3) round-trip: parking
//! a lineage in place must block writes while leaving reads fully served
//! through the chain-walk, and a paused lineage must survive a process
//! restart via the durable lineage manifest (`crate::manifest`).

mod common;

use std::time::Instant;

use clawft_cow_memory::{BranchableMemory, CowMemoryError, VectorItem};

#[test]
fn pause_blocks_writes_but_reads_still_serve() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v1 = common::random_vector(&mut rng, dim);
    let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
    mem.checkpoint("c1").expect("checkpoint");

    let v2 = common::random_vector(&mut rng, dim);
    let id2 = mem.ingest(&[VectorItem::new(v2.clone())]).unwrap()[0];

    assert!(!mem.is_paused());
    mem.pause("idle").expect("pause");
    assert!(mem.is_paused());
    assert!(mem.status().paused);

    // Writes fail closed.
    assert!(matches!(
        mem.ingest(&[VectorItem::new(common::random_vector(&mut rng, dim))]),
        Err(CowMemoryError::Paused)
    ));
    assert!(matches!(mem.delete(&[id1]), Err(CowMemoryError::Paused)));
    assert!(matches!(
        mem.checkpoint("nope"),
        Err(CowMemoryError::Paused)
    ));
    assert!(matches!(mem.rollback(None), Err(CowMemoryError::Paused)));
    assert!(matches!(mem.promote(), Err(CowMemoryError::Paused)));
    let branch_dir = tempfile::tempdir().expect("tempdir");
    assert!(matches!(
        mem.branch(branch_dir.path()),
        Err(CowMemoryError::Paused)
    ));

    // Reads still walk the (working-less) chain fine.
    assert!(mem.query(&v1, 5).unwrap().iter().any(|r| r.id == id1));
    assert!(mem.query(&v2, 5).unwrap().iter().any(|r| r.id == id2));
    assert_eq!(mem.diff().added, Vec::<u64>::new());
    assert_eq!(mem.diff().deleted, Vec::<u64>::new());

    // Idempotent: pausing again while paused is a no-op.
    mem.pause("idle-again").expect("pause is idempotent");
    assert!(mem.is_paused());
}

#[test]
fn resume_restores_writes_and_is_o1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v1 = common::random_vector(&mut rng, dim);
    let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
    mem.pause("idle").expect("pause");

    let t0 = Instant::now();
    mem.resume().expect("resume");
    println!("resume latency = {:?} (derive, not copy)", t0.elapsed());

    assert!(!mem.is_paused());
    // Fresh working starts with zero own vectors -- the derive-not-copy
    // guarantee, observable via an empty diff() immediately after resume.
    assert!(mem.diff().added.is_empty());
    assert!(mem.diff().deleted.is_empty());

    // Writes work again.
    let v2 = common::random_vector(&mut rng, dim);
    let id2 = mem.ingest(&[VectorItem::new(v2.clone())]).unwrap()[0];
    assert!(mem.query(&v2, 5).unwrap().iter().any(|r| r.id == id2));

    // Pre-pause data is still visible through the chain.
    assert!(mem.query(&v1, 5).unwrap().iter().any(|r| r.id == id1));

    // Idempotent: resuming again while not paused is a no-op.
    mem.resume().expect("resume is idempotent");
    assert!(!mem.is_paused());
}

#[test]
fn pause_drop_open_resume_query_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let (v1, id1, v2, id2, v3, id3) = {
        let mut mem = common::new_memory(dir.path());

        let v1 = common::random_vector(&mut rng, dim);
        let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
        mem.checkpoint("c1").expect("checkpoint 1");

        let v2 = common::random_vector(&mut rng, dim);
        let id2 = mem.ingest(&[VectorItem::new(v2.clone())]).unwrap()[0];
        mem.checkpoint("c2").expect("checkpoint 2");

        let v3 = common::random_vector(&mut rng, dim);
        let id3 = mem.ingest(&[VectorItem::new(v3.clone())]).unwrap()[0];

        mem.pause("parked-for-restart").expect("pause");
        assert!(mem.is_paused());
        (v1, id1, v2, id2, v3, id3)
        // `mem` dropped here -- simulates a process restart.
    };

    let mut reopened = BranchableMemory::open(dir.path()).expect("open after pause");
    assert!(
        reopened.is_paused(),
        "manifest must restore the paused flag"
    );
    assert!(matches!(
        reopened.ingest(&[VectorItem::new(common::random_vector(&mut rng, dim))]),
        Err(CowMemoryError::Paused)
    ));

    reopened.resume().expect("resume after reopen");
    assert!(!reopened.is_paused());

    // Every pre-pause vector, across the full multi-checkpoint chain, must
    // be visible through the restored lineage.
    assert!(reopened.query(&v1, 5).unwrap().iter().any(|r| r.id == id1));
    assert!(reopened.query(&v2, 5).unwrap().iter().any(|r| r.id == id2));
    assert!(reopened.query(&v3, 5).unwrap().iter().any(|r| r.id == id3));
    assert_eq!(reopened.status().own_checkpoint_depth, 3);

    // Writes work fully after resume.
    let v4 = common::random_vector(&mut rng, dim);
    let id4 = reopened.ingest(&[VectorItem::new(v4.clone())]).unwrap()[0];
    assert!(reopened.query(&v4, 5).unwrap().iter().any(|r| r.id == id4));
}

#[test]
fn reopen_without_pause_restores_checkpoint_chain_via_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let (v1, id1) = {
        let mut mem = common::new_memory(dir.path());
        let v1 = common::random_vector(&mut rng, dim);
        let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
        mem.checkpoint("c1").expect("checkpoint");
        (v1, id1)
        // Dropped without pausing -- `working`'s own (empty) state is lost,
        // but the checkpoint chain was captured by `checkpoint()`'s manifest
        // write and must survive.
    };

    let reopened = BranchableMemory::open(dir.path()).expect("open");
    assert!(!reopened.is_paused());
    assert_eq!(
        reopened.status().own_checkpoint_depth,
        1,
        "manifest must restore the checkpoint even though the lineage was never paused"
    );
    assert!(reopened.query(&v1, 5).unwrap().iter().any(|r| r.id == id1));
}

#[test]
fn manifest_leftover_tmp_file_is_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let (v1, id1) = {
        let mut mem = common::new_memory(dir.path());
        let v1 = common::random_vector(&mut rng, dim);
        let id1 = mem.ingest(&[VectorItem::new(v1.clone())]).unwrap()[0];
        mem.checkpoint("c1").expect("checkpoint");
        (v1, id1)
    };

    // Simulate a crash mid-write: a stray temp file sits next to the real,
    // already-renamed-into-place manifest.
    std::fs::write(
        dir.path().join("manifest.json.tmp"),
        b"not valid json at all",
    )
    .expect("write stray tmp file");

    let reopened = BranchableMemory::open(dir.path()).expect("open must ignore the stray tmp file");
    assert!(reopened.query(&v1, 5).unwrap().iter().any(|r| r.id == id1));
}

#[test]
fn corrupt_manifest_errors_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    {
        let mut mem = common::new_memory(dir.path());
        mem.checkpoint("c1")
            .expect("checkpoint to produce a manifest");
    }

    std::fs::write(
        dir.path().join("manifest.json"),
        b"{ this is not valid json",
    )
    .expect("corrupt the manifest");

    let result = BranchableMemory::open(dir.path());
    assert!(
        matches!(result, Err(CowMemoryError::ManifestCorrupt { .. })),
        "a corrupt manifest must fail closed, never a silent partial restore"
    );
}
