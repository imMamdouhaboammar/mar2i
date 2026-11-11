use mar2i_core::keep_awake::KeepAwake;

#[test]
fn keep_awake_guard_lifecycle_and_raii_drop() {
    assert!(!KeepAwake::is_active(), "initially should be inactive");

    {
        let guard = KeepAwake::acquire();
        assert!(KeepAwake::is_active(), "active while guard is held");
        // Keep guard in scope
        let _ = &guard;
    }

    assert!(!KeepAwake::is_active(), "inactive after guard is dropped");
}
