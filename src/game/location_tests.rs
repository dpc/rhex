use super::*;
use crate::game::{
    actor::{Race, Slot},
    fixtures,
    item::{Item, Type},
};
use hex2d::{Angle, Position, XY};
use rand::{SeedableRng, rngs::SmallRng};

fn player_mut(loc: &mut Location) -> &mut Actor {
    loc.actors_byid.get_mut(&loc.player_id()).unwrap()
}

fn ready_act(loc: &mut Location, action: Action) {
    player_mut(loc).action_cd = 0;
    loc.act(loc.player_id(), action);
    fixtures::occupancy(loc);
}

#[test]
fn six_directions_do_not_rotate_and_turning_does_not_translate() {
    for dir in Direction::all() {
        for angle in [
            Angle::Forward,
            Angle::Left,
            Angle::Right,
            Angle::Back,
            Angle::LeftBack,
            Angle::RightBack,
        ] {
            let mut loc = fixtures::room();
            player_mut(&mut loc).pos.dir = *dir;
            ready_act(&mut loc, Action::Move(angle));
            assert_eq!(loc.player().coord(), Coordinate::from(*dir + angle));
            assert_eq!(loc.player().pos.dir, *dir);
            ready_act(&mut loc, Action::Turn(Angle::Right));
            assert_eq!(loc.player().coord(), Coordinate::from(*dir + angle));
            assert_eq!(loc.player().pos.dir, *dir + Angle::Right);
        }
    }
}

#[test]
fn wall_door_charge_and_occupied_destination_preserve_occupancy() {
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    let head = loc.player().head();
    loc.map[head] = tile::Tile::new(tile::Wall);
    ready_act(&mut loc, Action::Move(Angle::Forward));
    assert_eq!(loc.player().coord(), origin);
    loc.map[head] = tile::Tile::new(tile::Empty);
    loc.map[head].feature = Some(tile::Door(false));
    ready_act(&mut loc, Action::Charge);
    assert_eq!(loc.player().coord(), origin);
    assert_eq!(loc.map[head].feature, Some(tile::Door(true)));
    ready_act(&mut loc, Action::Charge);
    assert_eq!(loc.player().coord(), Coordinate::new(2, -2));

    let head = loc.player().head();
    loc.spawn(Actor::new(Race::Troll, Position::new(head, XY)));
    let before = loc.player().coord();
    ready_act(&mut loc, Action::Charge);
    assert_eq!(loc.player().coord(), before);
    assert_eq!(loc.player().did_attack.len(), 1);

    let mut loc = fixtures::room();
    loc.map[Coordinate::new(2, -2)] = tile::Tile::new(tile::Wall);
    ready_act(&mut loc, Action::Charge);
    assert_eq!(loc.player().coord(), Coordinate::from(XY));
}

#[test]
fn seeded_digging_exercises_success_and_failure_without_moving() {
    let mut outcomes = [false; 2];
    for seed in 0..32 {
        let mut loc = fixtures::room();
        loc.rng = SmallRng::seed_from_u64(seed);
        let head = loc.player().head();
        loc.map[head] = tile::Tile::new(tile::Wall);
        player_mut(&mut loc).equip(Box::new(Item::new(Type::Pickaxe, vec![])), 'p');
        player_mut(&mut loc).recalculate_stats();
        let sp = loc.player().sp;
        ready_act(&mut loc, Action::Move(Angle::Forward));
        outcomes[usize::from(loc.map[head].is_passable())] = true;
        assert_eq!(loc.player().coord(), Coordinate::new(0, 0));
        assert!(loc.player().sp < sp);
        assert_eq!(loc.player().noise_emision, 9);
    }
    assert_eq!(outcomes, [true, true]);
}

#[test]
fn inventory_pick_equip_replace_unequip_and_drop_conserve_items_and_stats() {
    let mut loc = fixtures::room();
    let head = loc.player().head();
    loc.items
        .insert(head, Box::new(Item::new(Type::Knife, vec![])));
    ready_act(&mut loc, Action::Pick);
    assert!(loc.items.is_empty());
    assert_eq!(loc.player().items_backpack.len(), 1);
    let damage = loc.player().stats.melee_dmg;
    ready_act(&mut loc, Action::Equip('a'));
    assert!(loc.player().items_backpack.is_empty());
    assert_eq!(loc.player().items_equipped[&Slot::RHand].0, 'a');
    assert!(loc.player().stats.melee_dmg > damage);
    player_mut(&mut loc).pick_item(Box::new(Item::new(Type::Sword, vec![])));
    assert!(loc.player().items_backpack.contains_key(&'b'));
    ready_act(&mut loc, Action::Equip('b'));
    assert_eq!(loc.player().items_equipped.len(), 1);
    assert!(loc.player().items_backpack.contains_key(&'a'));
    ready_act(&mut loc, Action::Equip('b'));
    assert!(loc.player().items_equipped.is_empty());
    assert_eq!(loc.player().stats.melee_dmg, damage);
    ready_act(&mut loc, Action::Drop_('a'));
    ready_act(&mut loc, Action::Drop_('b'));
    assert_eq!(loc.items.len(), 2);
    assert!(loc.player().items_backpack.is_empty());
    assert!(loc.at(loc.player().coord()).item().is_some());
    ready_act(&mut loc, Action::Equip('?'));
    ready_act(&mut loc, Action::Drop_('?'));
    assert_eq!(loc.items.len(), 2);
}

#[test]
fn full_inventory_returns_pickup_and_potion_is_consumed_once() {
    let mut loc = fixtures::room();
    while player_mut(&mut loc)
        .pick_item(Box::new(Item::new(Type::Junk, vec![])))
        .is_none()
    {}
    let count = loc.player().items_backpack.len();
    let head = loc.player().head();
    loc.items
        .insert(head, Box::new(Item::new(Type::Bow, vec![])));
    ready_act(&mut loc, Action::Pick);
    assert_eq!(loc.player().items_backpack.len(), count);
    assert_eq!(loc.at(head).item().unwrap().description(), "bow");
    player_mut(&mut loc)
        .items_backpack
        .insert('a', Box::new(Item::new(Type::HealthPotion, vec![])));
    let max_hp = loc.player().stats.base.max_hp;
    player_mut(&mut loc).hp = max_hp - 2;
    ready_act(&mut loc, Action::Equip('a'));
    assert_eq!(loc.player().hp, max_hp);
    assert!(!loc.player().items_backpack.contains_key(&'a'));
    ready_act(&mut loc, Action::Equip('a'));
    assert_eq!(loc.player().items_backpack.len(), count - 1);
}

#[test]
fn melee_samples_hit_miss_armor_stamina_and_cooldown() {
    let mut hits = 0;
    let mut misses = 0;
    let mut damages = std::collections::BTreeSet::new();
    for seed in 0..64 {
        let mut loc = fixtures::room();
        loc.rng = SmallRng::seed_from_u64(seed);
        let head = loc.player().head();
        let target = loc.spawn(Actor::new(
            Race::Troll,
            Position::new(head, XY + Angle::Back),
        ));
        let hp = loc.actors_byid[&target].hp;
        player_mut(&mut loc).sp = 0;
        ready_act(&mut loc, Action::Move(Angle::Forward));
        let result = &loc.player().did_attack[0];
        assert_eq!(
            loc.actors_byid[&target].hp,
            hp - if result.success { result.dmg } else { 0 }
        );
        if result.success {
            hits += 1;
            damages.insert(result.dmg);
        } else {
            misses += 1;
        }
        player_mut(&mut loc).action_cd = 2;
        let hp = loc.actors_byid[&target].hp;
        loc.act(loc.player_id(), Action::Move(Angle::Forward));
        assert_eq!(loc.actors_byid[&target].hp, hp);
    }
    assert!(hits > 0 && misses > 0);
    assert!(damages.len() > 1);
}

#[test]
fn ranged_requires_ready_bow_and_occupied_target_and_records_damage() {
    let mut loc = fixtures::room();
    let target_coord = Coordinate::new(3, -3);
    let target = loc.spawn(Actor::new(Race::Troll, Position::new(target_coord, XY)));
    let hp = loc.actors_byid[&target].hp;
    ready_act(&mut loc, Action::Ranged(target_coord));
    assert!(loc.player().did_attack.is_empty());
    assert_eq!(loc.actors_byid[&target].hp, hp);
    player_mut(&mut loc).equip(Box::new(Item::new(Type::Bow, vec![])), 'b');
    player_mut(&mut loc).action_cd = 2;
    loc.act(loc.player_id(), Action::Ranged(target_coord));
    assert_eq!(loc.actors_byid[&target].hp, hp);
    ready_act(&mut loc, Action::Ranged(Coordinate::new(-4, 0)));
    assert!(loc.player().did_attack.is_empty());
    let mut hits = 0;
    for seed in 0..16 {
        loc.rng = SmallRng::seed_from_u64(seed);
        loc.actors_byid.get_mut(&target).unwrap().hp = hp;
        player_mut(&mut loc).sp = 100;
        let cost = loc.player().melee_sp_cost();
        ready_act(&mut loc, Action::Ranged(target_coord));
        let result = &loc.player().did_attack[0];
        assert_eq!(
            loc.actors_byid[&target].hp,
            hp - if result.success { result.dmg } else { 0 }
        );
        hits += usize::from(result.success);
        assert_eq!(loc.player().sp, 100 - cost);
    }
    assert!(hits > 0 && hits < 16);
}

#[test]
fn opened_door_exposes_cells_and_updates_discovery_in_the_same_tick() {
    let origin = Coordinate::new(0, 0);
    let head = Coordinate::new(1, -1);
    let beyond = Coordinate::new(2, -2);
    let mut map = crate::game::Map::default();
    for coord in [origin, head, beyond] {
        map[coord] = tile::Tile::new(tile::Empty);
        map[coord].light = 8;
    }
    map[head].feature = Some(tile::Door(false));
    let mut player = Actor::new(Race::Human, Position::new(origin, XY));
    player.set_player();
    let mut loc = Location::fixture(map, player);
    loc.recalculate_light_map();
    let mut player = loc.actors_byid.remove(&0).unwrap();
    player.pre_spawn(&loc);
    player.post_spawn(&loc);
    loc.actors_byid.insert(0, player);
    assert!(!loc.player().sees(beyond));
    ready_act(&mut loc, Action::Move(Angle::Forward));
    assert_eq!(loc.player().coord(), origin);
    assert!(loc.player().sees(beyond));
    assert!(loc.player().discovered.contains(&beyond));
    ready_act(&mut loc, Action::Wait);
    assert!(!loc.player().discovered.contains(&beyond));
    assert!(loc.player().knows(beyond));
}

#[test]
fn death_drops_backpack_and_equipped_items_in_letter_order_once() {
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    player_mut(&mut loc).equip(Box::new(Item::new(Type::Bow, vec![])), 'b');
    player_mut(&mut loc)
        .items_backpack
        .insert('a', Box::new(Item::new(Type::Knife, vec![])));
    player_mut(&mut loc).hp = -3;
    loc.post_any_tick();
    assert_eq!(loc.at(origin).item().unwrap().description(), "knife");
    assert_eq!(loc.items.len(), 2);
    assert!(loc.player().items_equipped.is_empty());
    assert!(loc.player().items_backpack.is_empty());
    loc.post_any_tick();
    assert_eq!(loc.items.len(), 2);
    fixtures::occupancy(&loc);
}

#[test]
fn facing_light_discovery_and_movement_preserve_knowledge() {
    let mut loc = fixtures::room();
    let before = loc.player().known.clone();
    assert!(!before.is_empty());
    assert!(loc.player().visible.is_subset(&before));
    ready_act(&mut loc, Action::Turn(Angle::Back));
    assert!(before.is_subset(&loc.player().known));
    assert!(loc.player().visible.is_subset(&loc.player().known));
    let discovered = loc.player().discovered.clone();
    assert!(discovered.is_disjoint(&before));
    ready_act(&mut loc, Action::Wait);
    assert!(loc.player().discovered.is_empty());
    let coords: Vec<_> = loc.map.iter().map(|(c, _)| *c).collect();
    for coord in coords {
        loc.map[coord].light = 0;
    }
    player_mut(&mut loc).base_stats.infravision = 0;
    loc.post_any_tick();
    ready_act(&mut loc, Action::Turn(Angle::Back));
    assert!(loc.player().sees(loc.player().head()));
    assert!(!loc.player().sees(Coordinate::new(3, 0)));
    assert!(loc.player().knows(Coordinate::new(3, 0)));
    let old = loc.player().pos;
    ready_act(&mut loc, Action::Charge);
    assert_eq!(loc.player().prev_pos(), old);
    assert!(loc.player().known.is_superset(&before));
}
