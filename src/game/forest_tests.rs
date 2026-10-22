use super::*;
use hex2d::{Angle, Coordinate, Position, XY};

#[test]
fn forest_movement_hit_permissions_light_and_noise() {
    let origin = Coordinate::new(0, 0);
    for terrain in [
        tile::Ground,
        tile::ShallowWater,
        tile::Water,
        tile::TreeTrunk,
        tile::Thicket,
    ] {
        let mut loc = fixtures::room();
        let target = loc.player().head();
        loc.map[target] = Tile::new(terrain);
        loc.act(0, Action::Move(Angle::Forward));
        fixtures::occupancy(&loc);
        let walkable = matches!(terrain, tile::Ground | tile::ShallowWater);
        assert_eq!(loc.player().coord(), if walkable { target } else { origin });
        if terrain == tile::Thicket {
            assert_eq!(loc.map[target].type_, tile::Ground);
            assert!(!loc.sound_events.is_empty());
            loc.act(0, Action::Move(Angle::Forward));
            assert_eq!(loc.player().coord(), target);
        } else {
            assert_eq!(loc.map[target].type_, terrain);
        }
    }
    for action in [
        Action::Move(Angle::Back),
        Action::Move(Angle::LeftBack),
        Action::Move(Angle::RightBack),
    ] {
        let mut loc = fixtures::room();
        for c in origin.neighbors() {
            loc.map[c] = Tile::new(tile::Thicket);
        }
        loc.act(0, action);
        assert!(
            origin
                .neighbors()
                .iter()
                .all(|&c| loc.map[c].type_ == tile::Thicket)
        );
        assert!(loc.sound_events.is_empty());
    }
    for blocked in ["cooldown", "dead", "stamina"] {
        let mut loc = fixtures::room();
        let target = loc.player().head();
        loc.map[target] = Tile::new(tile::Thicket);
        let player = loc.actors_byid.get_mut(&0).unwrap();
        match blocked {
            "cooldown" => player.action_cd = 2,
            "dead" => player.hp = 0,
            _ => {
                player.stats.melee_str_req = 100;
                player.sp = 0;
            }
        }
        loc.act(0, Action::Move(Angle::Forward));
        assert_eq!(loc.map[target].type_, tile::Thicket, "{blocked}");
    }
}

#[test]
fn clearing_stops_charge_and_never_attacks_through_obstruction() {
    for terrain in [tile::Thicket, tile::TreeTrunk, tile::Water] {
        let mut loc = fixtures::room();
        let target = loc.player().head();
        let id = loc.spawn(Actor::new(actor::Race::Rat, Position::new(target, XY)));
        let hp = loc.actors_byid[&id].hp;
        loc.map[target] = Tile::new(terrain);
        loc.act(0, Action::Charge);
        assert_eq!(loc.player().coord(), Coordinate::new(0, 0));
        assert_eq!(loc.actors_byid[&id].hp, hp);
        assert!(loc.combat_events.is_empty());
    }
}

#[test]
fn forest_terrain_controls_sight_light_and_sound_not_just_movement() {
    for (terrain, opaque, sound) in [
        (tile::Ground, false, true),
        (tile::ShallowWater, false, true),
        (tile::Water, false, true),
        (tile::Thicket, true, true),
        (tile::TreeTrunk, true, false),
    ] {
        let t = Tile::new(terrain);
        assert_eq!(t.opaqueness() > 10, opaque);
        assert_eq!(t.transmits_sound(), sound);
        let mut loc = fixtures::room();
        let target = loc.player().head();
        // Broad barrier prevents side illumination; player has no torch.
        for c in Coordinate::new(0, 0).range_iter(5) {
            loc.map[c].light = 0;
        }
        for y in -4..=4 {
            loc.map[Coordinate::new(1, y)] = Tile::new(terrain);
        }
        loc.map[Coordinate::new(0, 0)].light = 5;
        loc.recalculate_light_map();
        let behind = target.scale(2);
        assert_eq!(loc.light_map[behind] == 0, opaque, "{terrain:?}");
        if terrain == tile::Thicket {
            loc.act(0, Action::Move(Angle::Forward));
            assert_eq!(loc.map[target].type_, tile::Ground);
            assert!(loc.light_map[behind] > 0);
            assert!(loc.player().sees(behind));
        }
    }
    assert!(Tile::new(tile::Thicket).sound_dampening() > Tile::new(tile::Ground).sound_dampening());
}
