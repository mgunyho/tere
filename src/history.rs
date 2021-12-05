use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::path::Component;


// Tree struct based on https://doc.rust-lang.org/stable/book/ch15-06-reference-cycles.html
pub struct HistoryTreeEntry<'a> {
    path: Component<'a>,
    parent: Weak<Self>, // option is not needed (I guess), we can just use a null weak to represent the root
    last_visited_child: RefCell<Option<Weak<Self>>>,
    children: RefCell<Vec<Rc<Self>>>,
}

struct HistoryTree<'a> {
    root: Rc<HistoryTreeEntry<'a>>,
    current_entry: Rc<HistoryTreeEntry<'a>>,
}

impl<'a> HistoryTree<'a> {

    pub fn current_entry(&self) -> &Rc<HistoryTreeEntry<'a>> {
        &self.current_entry
    }

    pub fn visit(&mut self, fname: &'a str) {
        let found_child = self.current_entry.children.borrow().iter()
            .find(|child| child.path.as_os_str() == fname).map(|c| c.clone());

        let child = found_child.unwrap_or_else(|| {
            // no such child found, create a new one
            let child = HistoryTreeEntry {
                path: Component::Normal(fname.as_ref()),
                parent: Rc::downgrade(&self.current_entry),
                children: RefCell::new(vec![]),
                last_visited_child: RefCell::new(None),
            };
            let child = Rc::new(child);
            self.current_entry.children.borrow_mut().push(Rc::clone(&child));
            child
        });

        self.current_entry.last_visited_child.replace(Some(Rc::downgrade(&child)));
        self.current_entry = child;
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_entry.parent.upgrade() {
            self.current_entry = Rc::clone(&parent);
        } // if the parent is None, we're at the root, so no need to do anything
    }

}

#[cfg(test)]
mod tests_for_history_tree {
    use super::*;

    fn init_history_tree<'a>() -> HistoryTree<'a> {
        let root = Rc::new(HistoryTreeEntry {
            path: Component::RootDir,
            parent: Weak::new(),
            last_visited_child: RefCell::new(None),
            children: RefCell::new(vec![]),
        });

        HistoryTree {
            root: Rc::clone(&root),
            current_entry: root,
        }
    }

    #[test]
    fn test_history_tree_visit() {
        let mut tree = init_history_tree();

        tree.visit("foo");
        assert_eq!(tree.current_entry().path, Component::Normal("foo".as_ref()));
        assert_eq!(tree.current_entry().parent.upgrade().unwrap().path, Component::RootDir);

        tree.visit("bar");
        assert_eq!(tree.current_entry().path, Component::Normal("bar".as_ref()));
        assert_eq!(tree.current_entry().parent.upgrade().unwrap().path, Component::Normal("foo".as_ref()));
        assert_eq!(tree.current_entry().parent.upgrade().unwrap().parent.upgrade().unwrap().path, Component::RootDir);

    }

    #[test]
    fn test_history_tree_go_up_down() {
        let mut tree = init_history_tree();

        tree.visit("foo");
        tree.visit("bar");

        let foo_path = Component::Normal("foo".as_ref());
        let bar_path = Component::Normal("bar".as_ref());

        tree.go_up();
        assert_eq!(tree.current_entry().path, foo_path);
        assert_eq!(tree.current_entry().children.borrow()[0].path, bar_path);

        tree.go_up();
        assert_eq!(tree.current_entry().path, Component::RootDir);
        assert_eq!(tree.current_entry().children.borrow()[0].path, foo_path);

        tree.go_up();
        assert_eq!(tree.current_entry().path, Component::RootDir);
        assert_eq!(tree.current_entry().children.borrow()[0].path, foo_path);

    }

    #[test]
    fn test_tree_pointer_counts() {
        let mut tree = init_history_tree();
        tree.visit("foo");
        let foo = Rc::downgrade(&tree.current_entry());
        tree.visit("bar");
        let bar = Rc::downgrade(&tree.current_entry());

        assert_eq!(Rc::weak_count(&tree.root), 1); // the child (foo)

        assert_eq!(Weak::strong_count(&foo), 1); // the root
        assert_eq!(Weak::weak_count(&foo), 3); // the child, last_visited_child of the root and the variable 'foo' above

        assert_eq!(Weak::strong_count(&bar), 2); // the parent (foo) and the tree current entry
        assert_eq!(Weak::weak_count(&bar), 2); // the variable 'bar' above, and last_visited_child of foo

        tree.go_up(); tree.go_up();
        assert_eq!(Weak::strong_count(&bar), 1); // the parent only now
        assert_eq!(Weak::weak_count(&bar), 2); // the variable 'bar' above, and last_visited_child of foo

        tree.visit("baz");
        assert_eq!(Rc::weak_count(&tree.root), 2); // two children

    }

    #[test]
    fn test_last_visisted_child() {
        let mut tree = init_history_tree();
        tree.visit("foo");
        let foo = Rc::clone(tree.current_entry());
        tree.go_up();
        assert!(Rc::ptr_eq(&foo, &tree.current_entry().last_visited_child.borrow().as_ref().unwrap().upgrade().unwrap()));
    }

}
