use super::{actor::Actor, item::Item};
use std::collections::VecDeque;

/// A stationary container whose contents are rolled once, when its level is built.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chest {
    /// Opening never rerolls or spills the stored items.
    pub opened: bool,
    /// Stored transfer order; a full backpack leaves this list unchanged.
    pub contents: VecDeque<Box<Item>>,
}

impl Chest {
    /// Create a closed chest with its final contents.
    pub fn new(contents: impl IntoIterator<Item = Box<Item>>) -> Self {
        Self {
            opened: false,
            contents: contents.into_iter().collect(),
        }
    }

    /// Describe only information revealed by opening the container.
    pub fn description(&self) -> String {
        if !self.opened {
            "closed chest".to_owned()
        } else if self.contents.is_empty() {
            "empty chest".to_owned()
        } else {
            format!(
                "open chest: {}",
                self.contents
                    .iter()
                    .map(|i| i.description())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    /// Distinguish a closed chest, an open reward, and an exhausted container.
    pub fn glyph(&self) -> char {
        if !self.opened {
            '%'
        } else if self.contents.is_empty() {
            '_'
        } else {
            '['
        }
    }

    /// Open, or transfer one item atomically without invoking ground-drop placement.
    pub fn interact(&mut self, actor: &mut Actor) -> String {
        if !self.opened {
            self.opened = true;
            actor.action_cd += 2;
            actor.noise_makes(5);
            return format!("You open the chest. {}.", self.description());
        }
        if self.contents.is_empty() {
            return "The chest is empty.".to_owned();
        }
        let item = self.contents.pop_front().unwrap();
        let name = item.description();
        if let Some(item) = actor.pick_item(item) {
            self.contents.push_front(item);
            "Your backpack is full; the item stays in the chest.".to_owned()
        } else {
            format!("You take {name} from the chest.")
        }
    }
}
