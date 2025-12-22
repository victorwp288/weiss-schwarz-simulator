use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};

#[test]
fn wsdb_header_roundtrip() {
    let cards = vec![CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    }];
    let db = CardDb::new(cards).expect("db build");
    let bytes = db.to_bytes_with_header().expect("wsdb bytes");
    let loaded = CardDb::from_wsdb_bytes(&bytes).expect("wsdb load");
    assert!(loaded.get(1).is_some());
}

#[test]
fn wsdb_bad_magic_rejected() {
    let cards = vec![CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    }];
    let db = CardDb::new(cards).expect("db build");
    let mut bytes = db.to_bytes_with_header().expect("wsdb bytes");
    bytes[0] = b'X';
    assert!(CardDb::from_wsdb_bytes(&bytes).is_err());
}

#[test]
fn wsdb_bad_schema_version_rejected() {
    let cards = vec![CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    }];
    let db = CardDb::new(cards).expect("db build");
    let mut bytes = db.to_bytes_with_header().expect("wsdb bytes");
    let bad = (CardDb::schema_version() + 1).to_le_bytes();
    bytes[4..8].copy_from_slice(&bad);
    assert!(CardDb::from_wsdb_bytes(&bytes).is_err());
}
