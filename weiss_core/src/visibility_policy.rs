use crate::config::{CurriculumConfig, ObservationVisibility};
use crate::events::Zone;
use crate::state::TargetZone;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneIdentityVisibility {
    Public,
    OwnerOnly,
}

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
