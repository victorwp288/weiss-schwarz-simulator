use crate::config::{CurriculumConfig, ObservationVisibility};
use crate::events::Zone;
use crate::state::TargetZone;

/// Visibility policy for revealing zone identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneIdentityVisibility {
    /// Publicly visible identity.
    Public,
    /// Visible only to the owner.
    OwnerOnly,
}

/// Determine identity visibility for a concrete zone.
pub fn zone_identity_visibility(
    zone: Zone,
    curriculum: &CurriculumConfig,
) -> ZoneIdentityVisibility {
    match zone {
        Zone::Deck | Zone::Hand | Zone::Stock => ZoneIdentityVisibility::OwnerOnly,
        Zone::Memory => {
            if curriculum.memory_is_public {
                ZoneIdentityVisibility::Public
            } else {
                ZoneIdentityVisibility::OwnerOnly
            }
        }
        _ => ZoneIdentityVisibility::Public,
    }
}

/// Determine identity visibility for a targeted zone.
pub fn target_zone_identity_visibility(
    zone: TargetZone,
    curriculum: &CurriculumConfig,
) -> ZoneIdentityVisibility {
    match zone {
        TargetZone::Hand | TargetZone::DeckTop | TargetZone::Stock => {
            ZoneIdentityVisibility::OwnerOnly
        }
        TargetZone::Memory => {
            if curriculum.memory_is_public {
                ZoneIdentityVisibility::Public
            } else {
                ZoneIdentityVisibility::OwnerOnly
            }
        }
        _ => ZoneIdentityVisibility::Public,
    }
}

/// Whether a zone should be hidden for a given viewer.
pub fn hide_zone_for_viewer(
    visibility: ObservationVisibility,
    viewer: Option<u8>,
    owner: u8,
    zone: Zone,
    curriculum: &CurriculumConfig,
) -> bool {
    if visibility != ObservationVisibility::Public {
        return false;
    }
    match zone_identity_visibility(zone, curriculum) {
        ZoneIdentityVisibility::Public => false,
        ZoneIdentityVisibility::OwnerOnly => viewer.map(|v| v != owner).unwrap_or(true),
    }
}

/// Whether a target zone should be hidden for a given viewer.
pub fn hide_target_zone_for_viewer(
    visibility: ObservationVisibility,
    viewer: Option<u8>,
    owner: u8,
    zone: TargetZone,
    curriculum: &CurriculumConfig,
) -> bool {
    if visibility != ObservationVisibility::Public {
        return false;
    }
    match target_zone_identity_visibility(zone, curriculum) {
        ZoneIdentityVisibility::Public => false,
        ZoneIdentityVisibility::OwnerOnly => viewer.map(|v| v != owner).unwrap_or(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_identity_visibility_respects_memory_flag() {
        let mut curriculum = CurriculumConfig {
            memory_is_public: false,
            ..CurriculumConfig::default()
        };

        assert_eq!(
            zone_identity_visibility(Zone::Deck, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            zone_identity_visibility(Zone::Hand, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            zone_identity_visibility(Zone::Stock, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            zone_identity_visibility(Zone::Memory, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            zone_identity_visibility(Zone::Stage, &curriculum),
            ZoneIdentityVisibility::Public
        );

        curriculum.memory_is_public = true;
        assert_eq!(
            zone_identity_visibility(Zone::Memory, &curriculum),
            ZoneIdentityVisibility::Public
        );
    }

    #[test]
    fn target_zone_identity_visibility_respects_memory_flag() {
        let mut curriculum = CurriculumConfig {
            memory_is_public: false,
            ..CurriculumConfig::default()
        };

        assert_eq!(
            target_zone_identity_visibility(TargetZone::Hand, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            target_zone_identity_visibility(TargetZone::DeckTop, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            target_zone_identity_visibility(TargetZone::Stock, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            target_zone_identity_visibility(TargetZone::Memory, &curriculum),
            ZoneIdentityVisibility::OwnerOnly
        );
        assert_eq!(
            target_zone_identity_visibility(TargetZone::Stage, &curriculum),
            ZoneIdentityVisibility::Public
        );

        curriculum.memory_is_public = true;
        assert_eq!(
            target_zone_identity_visibility(TargetZone::Memory, &curriculum),
            ZoneIdentityVisibility::Public
        );
    }

    #[test]
    fn hide_zone_for_viewer_only_hides_private_zones_in_public_mode() {
        let mut curriculum = CurriculumConfig {
            memory_is_public: false,
            ..CurriculumConfig::default()
        };

        assert!(!hide_zone_for_viewer(
            ObservationVisibility::Full,
            Some(1),
            0,
            Zone::Hand,
            &curriculum
        ));
        assert!(!hide_zone_for_viewer(
            ObservationVisibility::Public,
            Some(0),
            0,
            Zone::Hand,
            &curriculum
        ));
        assert!(hide_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            Zone::Hand,
            &curriculum
        ));
        assert!(hide_zone_for_viewer(
            ObservationVisibility::Public,
            None,
            0,
            Zone::Hand,
            &curriculum
        ));
        assert!(!hide_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            Zone::Stage,
            &curriculum
        ));
        assert!(hide_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            Zone::Memory,
            &curriculum
        ));

        curriculum.memory_is_public = true;
        assert!(!hide_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            Zone::Memory,
            &curriculum
        ));
    }

    #[test]
    fn hide_target_zone_for_viewer_only_hides_private_zones_in_public_mode() {
        let mut curriculum = CurriculumConfig {
            memory_is_public: false,
            ..CurriculumConfig::default()
        };

        assert!(!hide_target_zone_for_viewer(
            ObservationVisibility::Full,
            Some(1),
            0,
            TargetZone::Hand,
            &curriculum
        ));
        assert!(!hide_target_zone_for_viewer(
            ObservationVisibility::Public,
            Some(0),
            0,
            TargetZone::DeckTop,
            &curriculum
        ));
        assert!(hide_target_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            TargetZone::Stock,
            &curriculum
        ));
        assert!(hide_target_zone_for_viewer(
            ObservationVisibility::Public,
            None,
            0,
            TargetZone::Hand,
            &curriculum
        ));
        assert!(!hide_target_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            TargetZone::Stage,
            &curriculum
        ));
        assert!(hide_target_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            TargetZone::Memory,
            &curriculum
        ));

        curriculum.memory_is_public = true;
        assert!(!hide_target_zone_for_viewer(
            ObservationVisibility::Public,
            Some(1),
            0,
            TargetZone::Memory,
            &curriculum
        ));
    }
}
