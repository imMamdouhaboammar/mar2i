use mar2i_core::profiles::{get_profile_by_id, standard_profiles};

#[test]
fn ministry_profiles_contain_mandated_presets() {
    let profiles = standard_profiles();
    assert!(profiles.len() >= 4);

    let archival = get_profile_by_id("master_archive").expect("master_archive profile must exist");
    assert_eq!(archival.mode, "archive");
    assert_eq!(archival.codec, "ffv1");
    assert_eq!(archival.container, "mkv");
    assert!(!archival.hardware_accelerated, "archival must not use hardware acceleration");
    assert_eq!(archival.verification, "framemd5_lossless");

    let broadcast = get_profile_by_id("broadcast_hd").expect("broadcast_hd profile must exist");
    assert_eq!(broadcast.mode, "publish");
    assert_eq!(broadcast.codec, "hevc");
    assert!(broadcast.hardware_accelerated);

    let press = get_profile_by_id("press_universal").expect("press_universal profile must exist");
    assert_eq!(press.mode, "publish");
    assert_eq!(press.codec, "h264");

    let field = get_profile_by_id("field_mobile").expect("field_mobile profile must exist");
    assert_eq!(field.mode, "publish");
    assert_eq!(field.codec, "av1");
}
