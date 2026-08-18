//! Compile-time distribution-channel policy.
//!
//! Direct builds retain the GitHub release checker. Microsoft Store builds set
//! `PANOPTICON_DISTRIBUTION_CHANNEL=store` while compiling so package updates
//! remain exclusively managed by Microsoft Store.

const DEFAULT_CHANNEL: &str = "direct";

pub(crate) fn channel() -> &'static str {
    option_env!("PANOPTICON_DISTRIBUTION_CHANNEL").unwrap_or(DEFAULT_CHANNEL)
}

pub(crate) fn updates_managed_by_store() -> bool {
    channel_is_store(channel())
}

fn channel_is_store(value: &str) -> bool {
    value.eq_ignore_ascii_case("store") || value.eq_ignore_ascii_case("microsoft-store")
}

#[cfg(test)]
mod tests {
    use super::{channel_is_store, DEFAULT_CHANNEL};

    #[test]
    fn direct_is_the_safe_default() {
        assert_eq!(DEFAULT_CHANNEL, "direct");
        assert!(!channel_is_store(DEFAULT_CHANNEL));
    }

    #[test]
    fn store_aliases_disable_the_direct_update_channel() {
        assert!(channel_is_store("store"));
        assert!(channel_is_store("STORE"));
        assert!(channel_is_store("Microsoft-Store"));
    }

    #[test]
    fn unrelated_channels_do_not_disable_direct_updates() {
        assert!(!channel_is_store("portable"));
        assert!(!channel_is_store("github"));
        assert!(!channel_is_store(""));
    }
}
