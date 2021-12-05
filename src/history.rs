use std::cell::RefCell;
use std::rc::{Rc, Weak};


// Tree struct based on https://doc.rust-lang.org/stable/book/ch15-06-reference-cycles.html
pub struct HistoryTreeEntry {
    name: String, //TODO: use Path / PathComponent instead? or None? to represent root (and what else?) correctly
    parent: RefCell<Weak<Self>>, // option is not needed (I guess), we can just use a null weak to represent the root
    //last_visited_child: Option<RefCell<Self>>, //TODO
    children: RefCell<Vec<Rc<Self>>>,
}

struct HistoryTree {
    root: Rc<HistoryTreeEntry>,
    current_entry: Rc<HistoryTreeEntry>,
}

impl HistoryTree {

    pub fn current_entry(&self) -> &Rc<HistoryTreeEntry> {
        &self.current_entry
    }

    pub fn visit(&mut self, fname: &str) {
        if let Some(child) = self.current_entry.clone().children.borrow().iter()
            .find(|child| child.name == fname) {
                //self.current_entry.last_visited_child = Some(Rc::downgrade(child.clone()))
                self.current_entry = child.clone()
        }
        //no such child found, create a new one
        let child = HistoryTreeEntry {
            name: fname.to_string(),
            parent: RefCell::new(Rc::downgrade(&self.current_entry)),
            children: RefCell::new(vec![]),
        };

        let child = Rc::new(child);
        self.current_entry.children.borrow_mut().push(child.clone());

        self.current_entry = child;
    }

    pub fn go_up(mut self) -> Self {
        todo!()
    }

}

#[cfg(test)]
mod tests_for_history_tree {
    use super::*;

    #[test]
    fn test_history_tree_visit() {
        let root = Rc::new(HistoryTreeEntry {
            name: "/".to_string(),
            parent: RefCell::new(Weak::new()),
            //last_visited_child: None,
            children: RefCell::new(vec![]),
        });
        let mut tree = HistoryTree {
            root: root.clone(),
            current_entry: root,
        };

        tree.visit("foo");
        assert_eq!(tree.current_entry.name, "foo");
        assert_eq!(tree.current_entry.parent.borrow().upgrade().unwrap().name, "/");

    }

}
