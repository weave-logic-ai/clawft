//! Tombstone correctness (integration plan §10): a delete issued against
//! `working` must shadow the same id living in a frozen ancestor, even
//! though the ancestor itself can never be mutated again to actually
//! remove it -- `RvfStore::delete` requires `!read_only`, and checkpointed
//! ancestors are frozen (`MemoryNode::freeze` -> `RvfStore::freeze` sets
//! `read_only = true`). The masking has to happen at the
//! `BranchableMemory`-level tombstone set, not inside RVF.

mod common;

use clawft_cow_memory::VectorItem;

#[test]
fn delete_in_working_shadows_a_frozen_ancestors_vector() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v = common::random_vector(&mut rng, dim);
    let id = mem.ingest(&[VectorItem::new(v.clone())]).unwrap()[0];
    mem.checkpoint("holds-v").expect("checkpoint");

    // `v` now lives only in the frozen checkpoint ancestor -- `working` was
    // re-derived empty. Confirm chain-walk still finds it there first.
    assert!(mem.query(&v, 5).unwrap().iter().any(|r| r.id == id));

    mem.delete(&[id]).expect("delete");

    assert!(
        !mem.query(&v, 5).unwrap().iter().any(|r| r.id == id),
        "delete on working must shadow the same id living in a frozen ancestor"
    );
    assert_eq!(mem.diff().deleted, vec![id]);
}

#[test]
fn delete_then_reingest_in_the_same_node_unhides_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v = common::random_vector(&mut rng, dim);
    let id = mem.ingest(&[VectorItem::new(v.clone())]).unwrap()[0];

    mem.delete(&[id]).expect("delete");
    assert!(!mem.query(&v, 5).unwrap().iter().any(|r| r.id == id));

    mem.ingest(&[VectorItem::new(v.clone()).with_id(id)])
        .expect("re-ingest under the same id");
    assert!(
        mem.query(&v, 5).unwrap().iter().any(|r| r.id == id),
        "re-ingesting a tombstoned id in the same node must un-hide it"
    );
}

#[test]
fn delete_across_multiple_checkpoints_stays_masked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let dim = common::dim() as usize;
    let mut rng = rand::thread_rng();

    let v = common::random_vector(&mut rng, dim);
    let id = mem.ingest(&[VectorItem::new(v.clone())]).unwrap()[0];
    mem.checkpoint("c1").expect("checkpoint 1");
    mem.delete(&[id]).expect("delete");
    mem.checkpoint("c2").expect("checkpoint 2");

    // `id` is now tombstoned in the middle of the chain (c2, which holds no
    // local edit for it) and physically present two nodes further back
    // (c1). It must stay masked when queried from a brand-new `working`.
    assert!(!mem.query(&v, 5).unwrap().iter().any(|r| r.id == id));
    assert_eq!(mem.status().own_checkpoint_depth, 2);
}
