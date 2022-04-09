use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::path::Path;
use serde::ser::{Serialize, Serializer, SerializeMap};
use serde::de::{Deserialize, Deserializer, Visitor, MapAccess, Error as deError};


// Tree struct based on https://doc.rust-lang.org/stable/book/ch15-06-reference-cycles.html
pub struct HistoryTreeEntry {
    label: String,
    parent: RefCell<Weak<Self>>, // option is not needed (I guess), we can just use a null weak to represent the root
    last_visited_child: RefCell<Option<Weak<Self>>>,
    children: RefCell<Vec<Rc<Self>>>,
}

impl HistoryTreeEntry {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![]),
            last_visited_child: RefCell::new(None),
        }
    }

    /// Convenience method for accessing the name of the last visited child, if it exists.
    /// Returns an owned String, because I couldn't figure out the borrowing here.
    pub fn last_visited_child_label(&self) -> Option<String> {
        self.last_visited_child
            .borrow()
            .as_ref()
            .and_then(|ptr| ptr.upgrade())
            // note: tried .map(|child| child.label.as_str()), but it's no good.
            .map(|child| child.label.clone())
    }
}


pub struct HistoryTree {
    root: Rc<HistoryTreeEntry>,
    current_entry: Rc<HistoryTreeEntry>,
}

impl HistoryTree {

    pub fn current_entry(&self) -> &Rc<HistoryTreeEntry> {
        &self.current_entry
    }

    /// Parse an absolute path into a history tree, with one child for each folder.
    pub fn from_abs_path<P: AsRef<Path>>(path: P) -> Self
    {
       let root = Rc::new(HistoryTreeEntry::new("/"));
       let mut tree = Self {
           root: Rc::clone(&root),
           current_entry: root,
       };

       path.as_ref().components()
           .skip(1) // skip root component (NOTE: this will cause problems on windows...)
           .for_each(|component| tree.visit(&component.as_os_str().to_string_lossy()));
       tree
    }

    pub fn visit(&mut self, fname: &str) {
        let found_child = self.current_entry.children.borrow().iter()
            .find(|child| child.label == fname).cloned();

        let child = found_child.unwrap_or_else(|| {
            // no existing child with this name found, create a new one
            let child = HistoryTreeEntry::new(fname);
            child.parent.replace(Rc::downgrade(&self.current_entry));

            let child = Rc::new(child);
            self.current_entry.children.borrow_mut().push(Rc::clone(&child));
            child
        });

        self.current_entry.last_visited_child.replace(Some(Rc::downgrade(&child)));
        self.current_entry = child;
    }

    pub fn go_up(&mut self) {
        let maybe_parent = self.current_entry.parent.borrow().upgrade();
        if let Some(parent) = maybe_parent {
            self.current_entry = Rc::clone(&parent);
        } // if the parent is None, we're at the root, so no need to do anything
    }

    pub fn go_to_root(&mut self) {
        self.current_entry = Rc::clone(&self.root);
    }

    /// Change directory completely to a new absolute path
    pub fn change_dir<P: AsRef<Path>>(&mut self, abs_path: P) {
        self.go_to_root();
        for component in abs_path.as_ref().components().skip(1) {
            self.visit(&component.as_os_str().to_string_lossy())
        }
    }

}

impl std::fmt::Debug for HistoryTreeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_map()
            .entry(&"parent", &self.parent.borrow().upgrade().map(|p| p.label.clone()).unwrap_or("".to_string()))
            .entry(&"label", &self.label)
            .entry(&"last_visited_child", &self.last_visited_child_label().unwrap_or("".to_string()))
            .entry(&"children", &self.children.borrow())
            .finish()
    }
}

impl std::fmt::Debug for HistoryTree {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {

        // ad-hoc iterator to get all parents / full path of a node. should probably move this to its own method
        // (HistoryTreeEntry::get_full_path or something), but currently not used anywhere else
        let mut initial_entry = Some(self.current_entry.clone());
        let mut cur_parents = vec![];
        while let Some(cur) = initial_entry {
            cur_parents.push(cur.label.clone());
            initial_entry = cur.parent.borrow().upgrade();
        }
        let cur_parents: std::path::PathBuf = cur_parents.iter().rev().collect();

        f.debug_struct("HistoryTree")
            .field("root", &self.root)
            .field("current_entry", &cur_parents)
            .finish()
    }
}

impl Serialize for HistoryTreeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("label", &self.label)?;
        map.serialize_entry("last_visited_child", &self.last_visited_child_label())?;
        map.serialize_entry("children", &*self.children.borrow())?;
        map.end()
    }
}

// Wrapper for Rc<HistoryTreeEntry> to make it possible to impl Deserialize
struct HistoryTreeEntryPtr(Rc<HistoryTreeEntry>);

impl<'de> Deserialize<'de> for HistoryTreeEntryPtr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {

        struct HistoryTreeEntryVisitor;

        impl<'de> Visitor<'de> for HistoryTreeEntryVisitor {
            type Value = HistoryTreeEntryPtr;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("valid history tree data")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>
            {
                let mut label: Option<String> = None;
                let mut last_visited_child: Option<Option<String>> = None;
                let mut children: Option<Vec<Self::Value>> = None;

                while let Some(key) = access.next_key()? {
                    match key {
                        "label" => {
                            if label.is_some() {
                                return Err(deError::duplicate_field("label"));
                            }
                            label = Some(access.next_value()?);
                        },
                        "last_visited_child" => {
                            if last_visited_child.is_some() {
                                return Err(deError::duplicate_field("last_visited_child"));
                            }
                            let val: Option<String> = access.next_value()?;
                            last_visited_child = Some(val); //Some(if val.is_none() { None } else { Some(val) });
                        },
                        "children" => {
                            if children.is_some() {
                                return Err(deError::duplicate_field("children"));
                            }
                            let val: Vec<Self::Value> = access.next_value()?;
                            children = Some(val);
                        },
                        k => return Err(deError::unknown_field(k, &["label", "last_visited_child", "children"])),
                    }
                }

                let label = label.ok_or_else(|| deError::missing_field("label"))?;

                let children: Vec<Rc<HistoryTreeEntry>> = children
                    .ok_or_else(|| deError::missing_field("children"))?
                    .drain(..).map(|p| p.0).collect();

                let last_visited_child = last_visited_child
                    .ok_or_else(|| deError::missing_field("last_visited_child"))?
                    .map(|label| children.iter().find(|c| c.label == label).map(Rc::downgrade))
                    .flatten();

                let ret = HistoryTreeEntry {
                    label,
                    last_visited_child: RefCell::new(last_visited_child),
                    parent: RefCell::new(Weak::new()), //TODO
                    children: RefCell::new(children),
                };

                let ret = Rc::new(ret);
                for child in ret.children.borrow_mut().iter() {
                    child.parent.replace(Rc::downgrade(&ret));
                }

                Ok(HistoryTreeEntryPtr(ret))
            }
        }

        deserializer.deserialize_map(HistoryTreeEntryVisitor)
    }
}

impl Serialize for HistoryTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        self.root.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HistoryTree {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let root = HistoryTreeEntryPtr::deserialize(deserializer)?.0;
        Ok(Self {
            root: Rc::clone(&root),
            current_entry: root,
        })
    }
}

#[cfg(test)]
mod tests_for_history_tree {
    use super::*;

    fn init_history_tree() -> HistoryTree {
        let root = Rc::new(HistoryTreeEntry {
            label: "/".to_string(),
            parent: RefCell::new(Weak::new()),
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
        assert_eq!(tree.current_entry().label, "foo");
        assert_eq!(tree.current_entry().parent.borrow().upgrade().unwrap().label, "/");

        tree.visit("bar");
        assert_eq!(tree.current_entry().label, "bar");
        assert_eq!(tree.current_entry().parent.borrow().upgrade().unwrap().label, "foo");
        assert_eq!(tree.current_entry().parent.borrow().upgrade().unwrap().parent.borrow().upgrade().unwrap().label, "/");

    }

    #[test]
    fn test_history_tree_go_up_down() {
        let mut tree = init_history_tree();

        tree.visit("foo");
        tree.visit("bar");

        tree.go_up();
        assert_eq!(tree.current_entry().label, "foo");
        assert_eq!(tree.current_entry().children.borrow()[0].label, "bar");

        tree.go_up();
        assert_eq!(tree.current_entry().label, "/");
        assert_eq!(tree.current_entry().children.borrow()[0].label, "foo");

        tree.go_up();
        assert_eq!(tree.current_entry().label, "/");
        assert_eq!(tree.current_entry().children.borrow()[0].label, "foo");

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

    #[test]
    fn test_go_to_root() {
        let mut tree = init_history_tree();
        let root = Rc::clone(tree.current_entry());
        tree.visit("foo");
        tree.visit("bar");
        tree.visit("baz");
        tree.go_to_root();
        assert!(Rc::ptr_eq(&root, tree.current_entry()));
        assert_eq!(tree.current_entry().label, "/");
    }

    #[test]
    fn test_from_abs_path() {
        let mut tree = HistoryTree::from_abs_path("/foo/bar/baz");
        assert_eq!(tree.current_entry().label, "baz");
        tree.go_to_root();
        assert_eq!(tree.current_entry().last_visited_child_label().unwrap(), "foo");
        tree.visit("foo");
        assert_eq!(tree.current_entry().label, "foo");
        assert_eq!(tree.current_entry().last_visited_child_label().unwrap(), "bar");
        tree.visit("bar");
        assert_eq!(tree.current_entry().label, "bar");
        assert_eq!(tree.current_entry().last_visited_child_label().unwrap(), "baz");
        tree.visit("baz");
        assert_eq!(tree.current_entry().label, "baz");
        assert_eq!(tree.current_entry().last_visited_child_label(), None);
    }

    #[test]
    fn test_change_dir() {
        let mut tree = HistoryTree::from_abs_path("/foo/bar/baz");
        tree.change_dir("/foo/bax");
        assert_eq!(tree.current_entry().label, "bax");
        tree.go_up();
        assert_eq!(
            vec!["bar".to_string(), "bax".to_string()],
            tree.current_entry().children.borrow().iter()
                .map(|child| child.label.clone()).collect::<Vec<String>>()
            );
        tree.visit("bar");
        assert_eq!(tree.current_entry().last_visited_child_label(), Some("baz".to_string()));

        tree.change_dir("/");
        assert!(Rc::ptr_eq(&tree.current_entry(), &tree.root));

        tree.change_dir("/foo/bax");

        //println!("{:#?}", tree); panic!();
    }

    #[test]
    fn test_debug_print() {
        let mut tree = HistoryTree::from_abs_path("/foo/bar/baz");
        tree.change_dir("/foo/bar/boo");
        tree.change_dir("/qux/zoo");
        println!("{:#?}", tree);
        //panic!(); // uncomment this to see print
    }

    #[test]
    fn test_serialize() {
        let mut tree = HistoryTree::from_abs_path("/foo/bar");
        tree.change_dir("/foo/baz");
        let ser = serde_json::to_string(&tree.root.as_ref()).unwrap();
        assert_eq!(ser, r#"{"label":"/","last_visited_child":"foo","children":[{"label":"foo","last_visited_child":"baz","children":[{"label":"bar","last_visited_child":null,"children":[]},{"label":"baz","last_visited_child":null,"children":[]}]}]}"#);
    }

    #[test]
    fn test_deserialize() {
        //let mut tree = HistoryTree::from_abs_path("/");
        let mut tree = HistoryTree::from_abs_path("/foo/bar");
        tree.change_dir("/foo/baz");
        println!("{:#?}", tree);

        let ser = serde_json::to_string(&tree).unwrap();
        println!("{}", ser); //{"label":"/","last_visited_child":null,"children":[]}
        let tree2: HistoryTree = serde_json::from_str(&ser).unwrap();
        println!("{:#?}", tree2);

        assert_eq!(ser, serde_json::to_string(&tree2).unwrap());

    }

}
