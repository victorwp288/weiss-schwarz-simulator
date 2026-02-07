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
