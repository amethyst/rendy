use cranelift_entity::{
    PrimaryMap, entity_impl,
    packed_option::PackedOption,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct List(u32);
entity_impl!(List);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Node(u32);
entity_impl!(Node);

struct ListData {
    head: PackedOption<Node>,
    tail: PackedOption<Node>,
}

struct ListNode<T> {
    prev: PackedOption<Node>,
    next: PackedOption<Node>,
    value: T,
}

pub struct LinkedListPool<T> {
    lists: PrimaryMap<List, ListData>,
    nodes: PrimaryMap<Node, ListNode<T>>,
}

impl<'a, T> LinkedListPool<T> {

    pub fn new() -> Self {
        Self {
            lists: PrimaryMap::new(),
            nodes: PrimaryMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.lists.clear();
        self.nodes.clear();
    }

    pub fn create(&mut self) -> List {
        self.lists.push(ListData {
            head: None.into(),
            tail: None.into(),
        })
    }

    pub fn head(&self, list: List) -> Option<Node> {
        self.lists[list].head.expand()
    }

    pub fn tail(&self, list: List) -> Option<Node> {
        self.lists[list].tail.expand()
    }

    pub fn next(&self, node: Node) -> Option<Node> {
        self.nodes[node].next.expand()
    }

    pub fn prev(&self, node: Node) -> Option<Node> {
        self.nodes[node].prev.expand()
    }

    /// Takes elemenents out of l1 and l2, and returns a new list consisting of
    /// them concatinated
    pub fn concat(&mut self, l1: List, l2: List) -> List {

        let (lh, lt) = match (
            self.lists[l1].head.expand(),
            self.lists[l1].tail.expand(),
            self.lists[l2].head.expand(),
            self.lists[l2].tail.expand(),
        ) {
            (Some(l1h), Some(l1t), Some(l2h), Some(l2t)) => {
                debug_assert!(self.nodes[l1h].prev.is_none());
                debug_assert!(self.nodes[l1t].next.is_none());
                debug_assert!(self.nodes[l2h].prev.is_none());
                debug_assert!(self.nodes[l2t].next.is_none());
                self.nodes[l1t].next = Some(l2h).into();
                self.nodes[l2h].prev = Some(l1t).into();
                (Some(l1h), Some(l2t))
            },
            (Some(l1h), Some(l1t), None, None) => {
                debug_assert!(self.nodes[l1h].prev.is_none());
                debug_assert!(self.nodes[l1t].next.is_none());
                (Some(l1h), Some(l1t))
            },
            (None, None, Some(l2h), Some(l2t)) => {
                debug_assert!(self.nodes[l2h].prev.is_none());
                debug_assert!(self.nodes[l2t].next.is_none());
                (Some(l2h), Some(l2t))
            },
            (None, None, None, None) => {
                (None, None)
            },
            _ => unreachable!(),
        };

        self.lists[l1].head = None.into();
        self.lists[l1].tail = None.into();
        self.lists[l2].head = None.into();
        self.lists[l2].tail = None.into();

        self.lists.push(ListData {
            head: lh.into(),
            tail: lt.into(),
        })
    }

    pub fn push(&mut self, list: List, value: T) {
        if let Some(last) = self.tail(list) {
            let new_last = self.nodes.push(ListNode {
                prev: Some(last).into(),
                next: None.into(),
                value,
            });
            self.lists[list].tail = Some(new_last).into();
            debug_assert!(self.nodes[last].next.is_none());
            self.nodes[last].next = Some(new_last).into();
        } else {
            let new = self.nodes.push(ListNode {
                prev: None.into(),
                next: None.into(),
                value,
            });
            debug_assert!(self.lists[list].head.is_none());
            debug_assert!(self.lists[list].tail.is_none());
            self.lists[list].head = Some(new).into();
            self.lists[list].tail = Some(new).into();
        }
    }

}
