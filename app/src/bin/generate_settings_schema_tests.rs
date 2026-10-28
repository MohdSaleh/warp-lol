use super::active_flags_for_channel;

#[test]
fn oss_channel_uses_release_flags() {
    assert_eq!(
        active_flags_for_channel("oss"),
        active_flags_for_channel("stable")
    );
}
