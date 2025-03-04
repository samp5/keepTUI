use std::collections::{btree_map::Iter, BTreeMap};

use ratatui::{
    style::{Modifier, Style, Styled},
    widgets::ListItem,
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Debug, Clone, Serialize, Deserialize, Ord, Eq, PartialEq, PartialOrd)]
pub struct TagID(pub u8);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagID,
    pub name: String,
    pub refs: u8,
}

impl Into<TagID> for Tag {
    fn into(self) -> TagID {
        self.id.clone()
    }
}
impl Into<TagID> for &Tag {
    fn into(self) -> TagID {
        self.id.clone()
    }
}

impl<'a> Into<ListItem<'a>> for &'a Tag {
    fn into(self) -> ListItem<'a> {
        let text = format!("{}\t refs: {}", self.name.clone(), self.refs);

        let item = ListItem::new(text);
        item.set_style(Style::default().add_modifier(Modifier::ITALIC))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCollection {
    pub tags: BTreeMap<TagID, Tag>,
    pub max_id: TagID,
}

pub struct TagCollectionIter<'a> {
    pub iter: Iter<'a, TagID, Tag>,
}

impl<'a> Iterator for TagCollectionIter<'a> {
    type Item = TagID;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(id, _)| id.clone())
    }
}

impl TagCollection {
    pub fn iter<'a>(&'a self) -> TagCollectionIter<'a> {
        TagCollectionIter {
            iter: self.tags.iter(),
        }
    }

    pub fn get(&self, id: TagID) -> Option<&Tag> {
        self.tags.get(&id)
    }

    pub fn get_mut(&mut self, id: TagID) -> Option<&mut Tag> {
        self.tags.get_mut(&id)
    }
    pub fn add(&mut self, name: impl AsRef<str>) {
        let id = self
            .tags
            .last_key_value()
            .map(|(&k, _)| TagID(k.0 + 1))
            .unwrap_or(TagID(0));

        self.tags.insert(
            id,
            Tag {
                id,
                name: name.as_ref().to_string(),
                refs: 0,
            },
        );
    }
    pub fn increase_ref(&mut self, id: &TagID) {
        self.tags.get_mut(id).map(|t| t.refs = t.refs + 1);
    }

    pub fn remove_by_id(&mut self, id: &TagID) {
        self.tags.remove(id);
        if &self.max_id == id {
            self.max_id = self
                .tags
                .last_key_value()
                .map(|(&k, _)| k)
                .unwrap_or(TagID(0));
        }
    }
}
