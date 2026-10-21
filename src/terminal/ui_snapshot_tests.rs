//! A few reviewed composed frames; targeted cell/style tests remain the main oracle.
use super::*;
use ratatui::layout::Rect;

fn assert_snapshot(name: &str, actual: &str, expected: &str) {
    if actual != expected {
        let mut changes = String::new();
        let old_rows: Vec<_> = expected.lines().collect();
        let new_rows: Vec<_> = actual.lines().collect();
        for index in 0..old_rows.len().max(new_rows.len()) {
            let old = old_rows.get(index).copied().unwrap_or("<missing>");
            let new = new_rows.get(index).copied().unwrap_or("<missing>");
            if old != new {
                writeln!(changes, "row {index:02}\n- {old}\n+ {new}").unwrap();
            }
        }
        panic!("{name} snapshot differs:\n{changes}\nactual frame:\n{actual}");
    }
}

#[test]
fn composed_room_inventory_and_resize_recovery_snapshots() {
    let now = Instant::now();
    let mut loc = crate::terminal::fixtures::room();
    loc.map[Coordinate::new(2, -2)].feature = Some(tile::Door(false));
    loc.map[Coordinate::new(-2, 0)].feature = Some(tile::Stairs);
    loc.map[Coordinate::new(0, 2)] = tile::Tile::new(tile::Wall);
    loc.actors_byid
        .get_mut(&0)
        .unwrap()
        .pick_item(Box::new(game::item::Item::new(
            game::item::Type::Knife,
            vec![],
        )));
    let mut ui = Ui::new(game::Engine::fixture(loc), (24, 60), now);
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    ui.log("A quiet room.");
    let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 24));
    ui.render_into(&mut buffer);
    let text = |buffer: &Buffer| -> String {
        (0..24)
            .map(|y| {
                let row: String = (0..60).map(|x| buffer[(x, y)].symbol()).collect();
                format!("|{}|\n", row.trim_end())
            })
            .collect()
    };
    assert_snapshot("room", &text(&buffer), include_str!("snapshots/room.txt"));
    ui.mode_switch_to(Mode::Inventory(InvMode::View));
    ui.render_into(&mut buffer);
    assert_snapshot(
        "inventory",
        &text(&buffer),
        include_str!("snapshots/inventory.txt"),
    );
    ui.handle_event(TerminalEvent::Resize(59, 24), now);
    ui.render_into(&mut buffer);
    assert_snapshot("small", &text(&buffer), include_str!("snapshots/small.txt"));
    ui.handle_event(TerminalEvent::Resize(60, 24), now);
    ui.render_into(&mut buffer);
    assert_snapshot(
        "inventory recovery",
        &text(&buffer),
        include_str!("snapshots/inventory.txt"),
    );
}
