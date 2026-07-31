//! End-to-end smoke: single topology boots + fake sensor + chain round-trip.

use clawft_worldmodel_service::{
    DeploymentTopology, ServiceConfig, WorldModelService,
};
use weftos_worldmodel::{
    AttestationPayload, EVENT_KIND_LEWM_FRAME_ATTESTATION, LATENT_DIM_U16,
};

#[test]
fn single_node_boots_end_to_end_with_fake_sensor() {
    let mut svc =
        WorldModelService::new(ServiceConfig::for_topology(DeploymentTopology::Single))
            .expect("config");
    let report = svc.boot().expect("boot");
    assert!(report.available);
    assert_eq!(report.latent_dim, LATENT_DIM_U16);

    let frames = svc
        .run_fake_sensor_pipeline(8, true)
        .expect("fake sensor pipeline");
    assert_eq!(frames.len(), 8);

    // Chain also carries periodic SIGReg health logs (WEFT-528); filter
    // to frame attestations for the WEFT-533 contract.
    let attests: Vec<_> = svc
        .chain
        .entries
        .iter()
        .filter(|e| e.kind == EVENT_KIND_LEWM_FRAME_ATTESTATION)
        .collect();
    assert_eq!(attests.len(), 8);

    for (i, entry) in attests.iter().enumerate() {
        let payload = AttestationPayload::from_json_bytes(&entry.payload).expect("json");
        assert_eq!(payload.manifold_major, 1);
        assert_eq!(payload.latent_dim, LATENT_DIM_U16);
        let tuple = payload.to_tuple().expect("tuple");
        assert!(tuple.manifold_matches_local());
        assert_eq!(tuple.frame_seq, i as u64);
        assert_eq!(frames[i].chain_seq, entry.seq);
    }
}

#[test]
fn topologies_selectable_via_config() {
    for name in ["single", "hot_standby", "peer_to_peer", "p2p", "standby"] {
        let topo = DeploymentTopology::parse(name).expect(name);
        let cfg = ServiceConfig::for_topology(topo);
        let mut svc = WorldModelService::new(cfg).expect(name);
        assert!(svc.boot().is_ok(), "{name}");
    }
}
