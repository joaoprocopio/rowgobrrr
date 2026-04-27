use std::hash::{BuildHasher, Hash};

fn make_hash<Q, H>(hash_builder: &H, val: &Q) -> u64
where
    Q: Hash,
    H: BuildHasher,
{
    hash_builder.hash_one(val)
}

#[derive(Debug)]
pub struct Table<K, V, H, const S: usize> {
    hash_builder: H,
    mask: usize,
    table: [Option<(K, V)>; S],
}

impl<K, V, H, const S: usize> Table<K, V, H, S>
where
    K: Hash,
    H: BuildHasher,
{
    pub fn with_hasher(hasher: H) -> Self {
        Self {
            hash_builder: hasher,
            mask: S - 1,
            table: [const { None }; S],
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let hash = make_hash(&self.hash_builder, &key) as usize;
        let index = hash & self.mask;
        let elem = self.table[index].as_ref();

        elem.and_then(|el| Some(&el.1))
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let hash = make_hash(&self.hash_builder, &key) as usize;
        let index = hash & self.mask;
        let elem = self.table[index].as_mut();

        elem.and_then(|el| Some(&mut el.1))
    }

    pub fn insert(&mut self, key: K, value: V) {
        let hash = make_hash(&self.hash_builder, &key) as usize;
        let index = hash & self.mask;

        self.table[index] = Some((key, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Aggregate;
    use std::hash::RandomState;

    #[test]
    fn insert() {
        let mut table =
            Table::<&[u8], Aggregate, RandomState, 10_000>::with_hasher(RandomState::default());

        dbg!(&table);
        table.insert(b"jac", Aggregate::new(1));
        dbg!(&table);
    }
}
