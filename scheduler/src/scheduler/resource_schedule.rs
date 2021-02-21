use std::cmp::Ordering;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::ops::Mul;

use cranelift_entity::EntityRef;
use cranelift_entity_set::BoundEntitySet;

use rendy_core::hal;

use super::super::{
    interface::EntityId,
    builder::{
        ProceduralBuilder, ResourceId, ResourceKind,
    },
};

/// State machine for walking through resource uses on a index basis. This makes
/// it possible for us to do a single pass over each resource to update
/// cumulative states.
struct WalkFSM<A> {
    state: [Option<(usize, A)>; 3],
    cursor: usize,
}
impl<A> WalkFSM<A>
where
    A: Copy,
{

    fn new() -> Self {
        Self {
            state: [None, None, None],
            cursor: 0,
        }
    }

    fn advance(
        &mut self,
        mut next: impl FnMut() -> Option<(usize, A)>,
    ) -> (usize, [Option<(usize, A)>; 3])
    {
        if self.cursor == 0 {

            // Initialization is a special case.
            self.state = match next() {
                Some((0, a)) => [None, Some((0, a)), next()],
                Some(n) => [None, None, Some(n)],
                None => [None, None, None],
            };

        } else {

            // General state update
            let s = self.state;
            self.state = match s {
                // General case.
                // The cursor is on next, get a new one and push to end.
                [a, None, c @ Some(_)] if c.unwrap().0 == self.cursor => [a, c, next()],
                [_, b, c @ Some(_)] if c.unwrap().0 == self.cursor => [b, c, next()],
                // The cursor is not yet on next. Only thing we need to do
                // is push current out of its slot.
                [_, a @ Some(_), b @ Some(_)] => [a, None, b],
                // The cursor is not yet on next, and we have already pushed
                // current back. Do nothing.
                [_, None, Some(_)] => s,
                // We are approaching the end, and have gone past current.
                [_, a @ Some(_), None] => {
                    debug_assert!(next().is_none());
                    [a, None, None]
                },
                // Terminal state
                [Some(_), None, None] => s,
                // Dead end state
                [None, None, None] => s,
                // Some states are unreachable in our state machine
                _ => unreachable!(),

            };

        }
        let curr_cursor = self.cursor;
        self.cursor += 1;

        // Calculate return state
        (
            curr_cursor,
            self.state,
        )
    }

}

#[derive(Debug, Copy, Clone)]
enum ResourceUseKind {
    Attachment,
    /// Required by the spec to be fully unique, even if only an input
    /// attachment. Does however enable reordering.
    AttachmentInput,
    /// Descriptor with read/write access. Prevents concurrent access
    /// and reordering.
    DescriptorWrite,
    //DescriptorRead(DescriptorRead),
}

/// Stores entity indices as u16.
/// I guess if this ever becomes a problem (who would use more than 2^16
/// entities?!), we can change it.
#[derive(Debug, Copy, Clone)]
enum ResourceUseState<A> {
    /// Indicates that the resource is an alias. This will cause a panic if a
    /// query is performed on an alias instead of what it references.
    Alias,
    Current {
        //kind: ResourceUseKind,
        aux: A,
        prev: Option<RawRow>,
        next: Option<RawRow>,
    },
    Between {
        prev: Option<RawRow>,
        next: Option<RawRow>,
    },
}
impl<A> ResourceUseState<A> {

    pub fn get(&self, dir: Direction) -> Option<RawRow> {
        match (dir, self) {
            (Direction::Reverse, ResourceUseState::Current { prev, .. }) => *prev,
            (Direction::Forward, ResourceUseState::Current { next, .. }) => *next,
            (Direction::Reverse, ResourceUseState::Between { prev, .. }) => *prev,
            (Direction::Forward, ResourceUseState::Between { next, .. }) => *next,
            (_, ResourceUseState::Alias) => panic!(),
        }
    }

    pub fn is_current(&self) -> bool {
        match self {
            ResourceUseState::Current { .. } => true,
            _ => false,
        }
    }

    pub fn aux(&self) -> &A {
        match self {
            ResourceUseState::Current { aux, .. } => aux,
            _ => panic!(),
        }
    }

}

/// Row number in matrix
#[derive(Debug, Copy, Clone)]
struct RawRow(usize);

/// Column number in matrix
#[derive(Debug, Copy, Clone)]
struct RawCol(usize);
impl From<ResourceId> for RawCol {
    fn from(f: ResourceId) -> Self {
        RawCol(f.0.try_into().unwrap())
    }
}

/// Calculated matrix index
#[derive(Debug, Copy, Clone)]
struct RawIdx(usize);

pub(crate) trait IndexMapping {
    type Input: Copy + Display;
    fn to_raw(&self, val: Self::Input) -> usize;
    fn from_raw(&self, raw: usize) -> Self::Input;
}

#[derive(Debug)]
pub struct NaturalIndexMapping<T>(PhantomData<T>);
impl<T> NaturalIndexMapping<T> {
    pub fn new() -> Self {
        NaturalIndexMapping(PhantomData)
    }
}
impl<T> IndexMapping for NaturalIndexMapping<T> where T: EntityRef + Display {
    type Input = T;
    fn to_raw(&self, val: Self::Input) -> usize {
        val.index()
    }
    fn from_raw(&self, raw: usize) -> Self::Input {
        T::new(raw)
    }
}

pub struct DefinedIndexMapping<T> {
    pub forward: Vec<usize>,
    pub back: Vec<usize>,
    pub marker: PhantomData<T>,
}
impl<T> IndexMapping for DefinedIndexMapping<T> where T: EntityRef + Display {
    type Input = T;
    fn to_raw(&self, val: Self::Input) -> usize {
        self.forward[val.index()] - 1
    }
    fn from_raw(&self, raw: usize) -> Self::Input {
        T::new(self.back[raw] - 1)
    }
}

pub(crate) type NaturalScheduleMatrix<R, C, A> = ScheduleMatrix<NaturalIndexMapping<R>, NaturalIndexMapping<C>, A>;

/// Topologically sorted entity schedule with additional data for jumping
/// between resource uses.
///
/// Implements a matrix with one row for each entity, and one column for each
/// resource. The cells then contain the state of the resource in the entity,
/// including the entities it connects to before and after.
#[derive(Debug)]
pub(crate) struct ScheduleMatrix<R, C, A> {
    pub row: R,
    pub col: C,

    row_size: usize,
    col_size: usize,

    matrix: Vec<ResourceUseState<A>>,
}

impl<R, C, A> ScheduleMatrix<R, C, A>
where
    A: Copy,
    R: IndexMapping,
    C: IndexMapping,
{

    pub fn new(row: R, col: C) -> Self {
        ScheduleMatrix {
            row,
            col,

            row_size: 0,
            col_size: 0,

            matrix: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.row_size = 0;
        self.col_size = 0;

        self.matrix.clear();
    }

    pub fn set_dims(&mut self, rows: usize, cols: usize) {
        self.row_size = rows;
        self.col_size = cols;

        self.matrix.clear();
        self.matrix.resize(rows * cols, ResourceUseState::Between { prev: None, next: None });
    }

    //pub fn gen_natural_entity_order(&mut self)  {
    //    assert!(self.lookup.len() == self.lookup_back.len());
    //    for (i, n) in self.lookup.iter_mut().enumerate() {
    //        *n = i + 1;
    //    }
    //    for (i, n) in self.lookup_back.iter_mut().enumerate() {
    //        *n = EntityId(i);
    //    }
    //}

    fn row_to_raw(&self, row: R::Input) -> RawRow {
        RawRow(self.row.to_raw(row))
    }
    fn row_from_raw(&self, row: RawRow) -> R::Input {
        self.row.from_raw(row.0)
    }
    fn col_to_raw(&self, col: C::Input) -> RawCol {
        RawCol(self.col.to_raw(col))
    }
    fn col_from_raw(&self, col: RawCol) -> C::Input {
        self.col.from_raw(col.0)
    }

    fn ridx_calc(&self, row: RawRow, col: RawCol) -> RawIdx {
        assert!(col.0 < self.col_size);
        assert!(row.0 < self.row_size);
        RawIdx((col.0 * self.row_size) + row.0)
    }

    fn idx(&self, row: R::Input, col: C::Input) -> ResourceUseState<A> {
        self.ridx(self.row_to_raw(row), self.col_to_raw(col))
    }

    fn ridx(&self, row: RawRow, col: RawCol) -> ResourceUseState<A> {
        let idx = self.ridx_calc(row, col);
        self.matrix[idx.0]
    }
    fn ridx_mut(&mut self, row: RawRow, col: RawCol) -> &mut ResourceUseState<A> {
        let idx = self.ridx_calc(row, col);
        &mut self.matrix[idx.0]
    }

    /// Fills a correctly sized and premapped schedule with data from the
    /// supplied `query` lambda. The query lambda should, given a resource id,
    /// return an iterator that iterates over the uses of the resource in order
    /// from first to last.
    ///
    /// Panics if an entity which is not in the lookup map is encountered.
    pub(crate) fn populate<F, I>(&mut self, query: F)
    where
        F: Fn(C::Input) -> Option<I>,
        I: Iterator<Item = (R::Input, A)>,
    {

        for col_idx in 0..self.col_size {
            let col_idx = RawCol(col_idx);
            let res_id = self.col_from_raw(col_idx);

            if let Some(mut res_iter) = query(res_id) {

                let mut walk_fsm = WalkFSM::new();
                loop {
                    let (cursor, state) = walk_fsm.advance(|| {
                        res_iter.next().map(|(e, a)| (self.row.to_raw(e), a))
                    });
                    if cursor == self.row_size { break; }
                    let cursor = RawRow(cursor);

                    *self.ridx_mut(cursor, col_idx) = match state {
                        [a, None, b] => ResourceUseState::Between {
                            prev: a.map(|(v, _)| RawRow(v.try_into().unwrap())),
                            next: b.map(|(v, _)| RawRow(v.try_into().unwrap())),
                        },
                        [a, Some((_, aux)), b] => ResourceUseState::Current {
                            aux,
                            prev: a.map(|(v, _)| RawRow(v.try_into().unwrap())),
                            next: b.map(|(v, _)| RawRow(v.try_into().unwrap())),
                        },
                    };
                }

            } else {
                for n in 0..self.row_size {
                    let n = RawRow(n);
                    *self.ridx_mut(n, col_idx) = ResourceUseState::Alias;
                }
            }


        }

    }

}

/// Traversal direction.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

impl Direction {

    pub fn cmp<T>(&self, lhs: &T, rhs: &T) -> Ordering
    where
        T: Ord,
    {
        match self {
            Direction::Forward => lhs.cmp(rhs),
            Direction::Reverse => rhs.cmp(lhs),
        }
    }

}

impl Mul<Self> for Direction {
    type Output = Direction;
    fn mul(self, rhs: Direction) -> Direction {
        match (self, rhs) {
            (Direction::Forward, Direction::Forward) => Direction::Forward,
            (Direction::Reverse, Direction::Forward) => Direction::Reverse,
            (Direction::Forward, Direction::Reverse) => Direction::Reverse,
            (Direction::Reverse, Direction::Reverse) => Direction::Forward,
        }
    }
}

pub trait MemberTest<'a, T: 'a>: IntoIterator<Item = &'a T> {
    fn size(self) -> usize;
    fn is_member(self, val: &T) -> bool;
}
impl<'a, T> MemberTest<'a, T> for &'a BTreeSet<T> where T: Eq + Ord {
    fn size(self) -> usize {
        self.size()
    }
    fn is_member(self, val: &T) -> bool {
        self.contains(val)
    }
}

impl<R, C, A> ScheduleMatrix<R, C, A>
where
    A: Copy,
    R: IndexMapping,
    C: IndexMapping,
{

    /// Given N entities, returns the one that is scheduled first in the current
    /// order.
    pub fn first_entity(&self, mut entities: impl Iterator<Item = R::Input>, dir: Direction) -> Option<R::Input> {
        let first = entities.next();
        if first.is_none() {
            return None;
        }

        let mut first = first.unwrap();
        let mut first_idx = self.row_to_raw(first);

        for ent in entities {
            let ent_idx = self.row_to_raw(ent);
            if dir.cmp(&ent_idx.0, &first_idx.0) == Ordering::Less {
                first = ent;
                first_idx = ent_idx;
            }
        }

        Some(first)
    }

    fn walk_usage_multi_raw(&self, entity: RawRow, dir: Direction, mut resources: impl Iterator<Item = RawCol>) -> Option<RawRow> {
        let mut next = None;

        for res in resources {
            let rus = self.ridx(entity, res);

            next = match (next, rus.get(dir)) {
                (None, None) => None,
                (None, Some(i)) => Some(i),
                (Some(i), None) => Some(i),
                (Some(i1), Some(i2)) => {
                    if dir.cmp(&i1.0, &i2.0) == Ordering::Greater {
                        Some(i2)
                    } else {
                        Some(i1)
                    }
                },
            };
        }

        next
    }

    /// Given an entity and N resources, walk to the next usage of any of those
    /// resources.
    pub(crate) fn walk_usage_multi(&self, entity: R::Input, dir: Direction, mut resources: impl Iterator<Item = C::Input>) -> Option<R::Input> {
        self.walk_usage_multi_raw(
            self.row_to_raw(entity),
            dir,
            resources.map(|v| self.col_to_raw(v))
        ).map(|idx| self.row_from_raw(idx))
    }

    fn walk_usage_raw(&self, entity: Option<RawRow>, dir: Direction, res: RawCol) -> Option<(RawRow, A)> {
        if let Some(entity) = entity {
            let rus = self.ridx(entity, res);
            let next = rus.get(dir);
            if let Some(next) = next {
                let aux = *self.ridx(next, res).aux();
                Some((next, aux))
            } else {
                None
            }
        } else {
            let first = self.ridx(RawRow(0), res);
            if first.is_current() {
                Some((RawRow(0), *first.aux()))
            } else {
                if let Some(next) = first.get(dir) {
                    let aux = *self.ridx(next, res).aux();
                    Some((next, aux))
                } else {
                    None
                }
            }
        }
    }

    pub fn walk_usage(&self, entity: Option<R::Input>, dir: Direction, res: C::Input) -> Option<(R::Input, A)> {
        self.walk_usage_raw(
            entity.map(|e| self.row_to_raw(e)),
            dir,
            self.col_to_raw(res),
        ).map(|(idx, aux)| (self.row_from_raw(idx), aux))
    }

    pub fn usages_by<'a>(&'a self, entity: R::Input) -> impl Iterator<Item = (C::Input, A)> + 'a {
        self.resources()
            .filter_map(move |res| {
                let s = self.idx(entity, res);
                if s.is_current() {
                    Some((res, *s.aux()))
                } else {
                    None
                }
            })
    }

    pub fn aux(&self, entity: R::Input, resource: C::Input) -> A {
        *self.idx(entity, resource).aux()
    }

    /// Iterates all usages of a resource between two entities (inclusive).
    pub(crate) fn usages_between<'a>(&'a self, from: R::Input, to: R::Input, dir: Direction, resource: C::Input) -> impl Iterator<Item = (R::Input, A)> + 'a {
        let from_i = self.row_to_raw(from);
        let to_i = self.row_to_raw(to);
        assert!(dir.cmp(&from_i.0, &to_i.0) != Ordering::Greater);

        let resource = self.col_to_raw(resource);

        let current;
        if let ResourceUseState::Current { aux, .. } = self.ridx(from_i, resource) {
            current = Some((from_i, aux));
        } else {
            current = self.walk_usage_raw(Some(from_i), dir, resource);
        }

        UsagesBetweenIterator {
            sched: self,
            dir,
            resource,
            current,
            end: to_i,
        }
    }

    /// Iterates all entities
    pub(crate) fn entities<'a>(&'a self, dir: Direction) -> impl Iterator<Item = R::Input> + DoubleEndedIterator + 'a {
        let rev = dir == Direction::Reverse;
        (0..self.row_size).map(move |n| {
            if rev {
                self.row.from_raw(self.row_size - 1 - n)
            } else {
                self.row.from_raw(n)
            }
        })
    }

    pub(crate) fn entities_between<'a>(&'a self, from: R::Input, to: R::Input, dir: Direction) -> impl Iterator<Item = R::Input> + 'a {
        let from_i = self.row_to_raw(from);
        let to_i = self.row_to_raw(to);
        assert!(dir.cmp(&from_i.0, &to_i.0) == Ordering::Less);

        let rev = from_i.0 > to_i.0;

        let from_r = std::cmp::min(from_i.0, to_i.0);
        let to_r = std::cmp::max(from_i.0, to_i.0);
        let range = to_r - from_r;

        (0..=range).map(move |idx| {
            let midx = if rev {
                range - idx
            } else {
                idx
            };
            self.row_from_raw(RawRow(from_r + midx))
        })
    }

    /// Iterates all resources
    pub(crate) fn resources<'a>(&'a self) -> impl Iterator<Item = C::Input> + DoubleEndedIterator + 'a {
        (0..self.col_size).map(move |n| self.col.from_raw(n))
    }

    pub (crate) fn iter_in_order_btreeset<'a>(&'a self, items: &'a BTreeSet<R::Input>) -> impl Iterator<Item = R::Input> + 'a
    where
        R::Input: Ord,
    {
        let first = self.first_entity(items.iter().cloned(), Direction::Forward).unwrap();
        let last = self.first_entity(items.iter().cloned(), Direction::Reverse).unwrap();
        self.entities_between(first, last, Direction::Forward).filter(move |v| items.contains(v))
    }

    pub (crate) fn iter_in_order_boundset<'a>(&'a self, items: &'a BoundEntitySet<'a, R::Input>) -> impl Iterator<Item = R::Input> + 'a
    where
        R::Input: EntityRef + Copy,
    {
        let first = self.first_entity(items.iter(), Direction::Forward).unwrap();
        let last = self.first_entity(items.iter(), Direction::Reverse).unwrap();
        self.entities_between(first, last, Direction::Forward).filter(move |v| items.contains(*v))
    }

}

struct UsagesBetweenIterator<'a, R, C, A> {
    sched: &'a ScheduleMatrix<R, C, A>,
    dir: Direction,
    resource: RawCol,
    current: Option<(RawRow, A)>,
    end: RawRow,
}
impl<'a, R, C, A> Iterator for UsagesBetweenIterator<'a, R, C, A>
where
    R: IndexMapping,
    C: IndexMapping,
    A: Copy,
{
    type Item = (R::Input, A);
    fn next(&mut self) -> Option<(R::Input, A)> {
        if let Some((curr, _)) = self.current {
            if self.dir.cmp(&curr.0, &self.end.0) == Ordering::Greater {
                self.current = None;
            }
        }
        let ret = self.current;

        if let Some((curr, _)) = self.current {
            self.current = self.sched.walk_usage_raw(Some(curr), self.dir, self.resource);
        }

        ret.map(|(v, a)| (self.sched.row_from_raw(v), a))
    }
}


#[cfg(test)]
mod test {
    use cranelift_entity::EntityRef;

    use crate::interface::EntityId;
    use super::*;

    fn make_schedule() -> ScheduleMatrix<NaturalIndexMapping<EntityId>, NaturalIndexMapping<ResourceId>, ()> {
        let mut schedule: ScheduleMatrix<NaturalIndexMapping<EntityId>, NaturalIndexMapping<ResourceId>, ()> =
            ScheduleMatrix::new(NaturalIndexMapping::new(), NaturalIndexMapping::new());

        schedule.set_dims(10, 3);

        let s_ents = [
            [
                Some((EntityId(2), ())),
                Some((EntityId(6), ())),
                Some((EntityId(7), ())),
                None,
            ],
            [
                Some((EntityId(1), ())),
                Some((EntityId(2), ())),
                Some((EntityId(7), ())),
                Some((EntityId(9), ())),
            ],
            [
                Some((EntityId(0), ())),
                Some((EntityId(9), ())),
                None,
                None,
            ]
        ];

        schedule.populate(|col| Some(s_ents[col.index()].iter().filter_map(|v| *v)));

        schedule
    }

    #[test]
    fn first_usage() {
        let sched = make_schedule();

        let first = sched.first_entity([EntityId(0), EntityId(1), EntityId(2)].iter().cloned(), Direction::Forward);
        assert!(first == Some(EntityId(0)));

        let first = sched.first_entity([EntityId(0), EntityId(1), EntityId(2)].iter().cloned(), Direction::Reverse);
        assert!(first == Some(EntityId(2)));

        let first = sched.first_entity([EntityId(0), EntityId(1)].iter().cloned(), Direction::Reverse);
        assert!(first == Some(EntityId(1)));

    }

    #[test]
    fn walk_usage_multi() {
        let sched = make_schedule();

        let n = sched.walk_usage_multi(EntityId(2), Direction::Forward, [ResourceId(0)].iter().cloned());
        assert!(n == Some(EntityId(6)));

        let n = sched.walk_usage_multi(EntityId(3), Direction::Forward, [ResourceId(0)].iter().cloned());
        assert!(n == Some(EntityId(6)));

        let n = sched.walk_usage_multi(EntityId(2), Direction::Reverse, [ResourceId(0)].iter().cloned());
        assert!(n == None);

        let n = sched.walk_usage_multi(EntityId(3), Direction::Reverse, [ResourceId(0)].iter().cloned());
        assert!(n == Some(EntityId(2)));

        let n = sched.walk_usage_multi(EntityId(0), Direction::Forward, [ResourceId(0), ResourceId(1)].iter().cloned());
        println!("{:?}", n);
        assert!(n == Some(EntityId(1)));

        let n = sched.walk_usage_multi(EntityId(1), Direction::Forward, [ResourceId(0), ResourceId(1)].iter().cloned());
        assert!(n == Some(EntityId(2)));

        let n = sched.walk_usage_multi(EntityId(2), Direction::Forward, [ResourceId(0), ResourceId(1)].iter().cloned());
        assert!(n == Some(EntityId(6)));
    }

    #[test]
    fn walk_usage() {
        let sched = make_schedule();
        println!("{:#?}", sched);

        let n = sched.walk_usage(None, Direction::Forward, ResourceId(2));
        println!("{:?}", n);
        assert!(n == Some((EntityId(0), ())));
    }

    #[test]
    fn usages_by() {
        let sched = make_schedule();

        let n: Vec<_> = sched.usages_by(EntityId(1)).map(|v| v.0).collect();
        assert!(n == [ResourceId(1)]);

        let n: Vec<_> = sched.usages_by(EntityId(2)).map(|v| v.0).collect();
        assert!(n == [ResourceId(0), ResourceId(1)]);

        let n: Vec<_> = sched.usages_by(EntityId(9)).map(|v| v.0).collect();
        assert!(n == [ResourceId(1), ResourceId(2)]);

    }

    #[test]
    fn usages_between() {
        let sched = make_schedule();

        let mut u = sched.usages_between(EntityId(1), EntityId(8), Direction::Forward, ResourceId(1));
        assert!(u.next() == Some((EntityId(1), ())));
        assert!(u.next() == Some((EntityId(2), ())));
        assert!(u.next() == Some((EntityId(7), ())));
        assert!(u.next() == None);

        let mut u = sched.usages_between(EntityId(3), EntityId(9), Direction::Forward, ResourceId(1));
        assert!(u.next() == Some((EntityId(7), ())));
        assert!(u.next() == Some((EntityId(9), ())));
        assert!(u.next() == None);

        let mut u = sched.usages_between(EntityId(8), EntityId(1), Direction::Reverse, ResourceId(1));
        assert!(u.next() == Some((EntityId(7), ())));
        assert!(u.next() == Some((EntityId(2), ())));
        assert!(u.next() == Some((EntityId(1), ())));
        assert!(u.next() == None);

    }

    #[test]
    fn entities_iter() {
        let sched = make_schedule();

        let mut i = sched.entities(Direction::Forward);
        assert!(i.next() == Some(EntityId(0)));
        assert!(i.next() == Some(EntityId(1)));
        assert!(i.next() == Some(EntityId(2)));
        assert!(i.next() == Some(EntityId(3)));
        assert!(i.next() == Some(EntityId(4)));
        assert!(i.next() == Some(EntityId(5)));
        assert!(i.next() == Some(EntityId(6)));
        assert!(i.next() == Some(EntityId(7)));
        assert!(i.next() == Some(EntityId(8)));
        assert!(i.next() == Some(EntityId(9)));
        assert!(i.next() == None);

        let mut i = sched.entities(Direction::Reverse);
        assert!(i.next() == Some(EntityId(9)));
        assert!(i.next() == Some(EntityId(8)));
        assert!(i.next() == Some(EntityId(7)));
        assert!(i.next() == Some(EntityId(6)));
        assert!(i.next() == Some(EntityId(5)));
        assert!(i.next() == Some(EntityId(4)));
        assert!(i.next() == Some(EntityId(3)));
        assert!(i.next() == Some(EntityId(2)));
        assert!(i.next() == Some(EntityId(1)));
        assert!(i.next() == Some(EntityId(0)));
        assert!(i.next() == None);

    }

    #[test]
    fn entities_between_iter() {
        let sched = make_schedule();

        let mut i = sched.entities_between(EntityId(2), EntityId(6), Direction::Forward);
        assert!(i.next() == Some(EntityId(2)));
        assert!(i.next() == Some(EntityId(3)));
        assert!(i.next() == Some(EntityId(4)));
        assert!(i.next() == Some(EntityId(5)));
        assert!(i.next() == Some(EntityId(6)));
        assert!(i.next() == None);

        let mut i = sched.entities_between(EntityId(8), EntityId(5), Direction::Reverse);
        assert!(i.next() == Some(EntityId(8)));
        assert!(i.next() == Some(EntityId(7)));
        assert!(i.next() == Some(EntityId(6)));
        assert!(i.next() == Some(EntityId(5)));
        assert!(i.next() == None);

    }

    #[test]
    fn resources_iter() {
        let sched = make_schedule();

        let mut i = sched.resources();
        assert!(i.next() == Some(ResourceId(0)));
        assert!(i.next() == Some(ResourceId(1)));
        assert!(i.next() == Some(ResourceId(2)));
        assert!(i.next() == None);
    }

}
