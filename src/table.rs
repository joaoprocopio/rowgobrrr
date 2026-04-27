use std::hash::{BuildHasher, Hash};

// pub enum Entry {
//     Vacant,
//     Occupied,
// }

// pub struct VacantEntry<'a, K, V, H, const S: usize>
// where
//     K: Hash,
//     H: BuildHasher,
// {
//     hash: u64,
//     key: K,
//     table: &'a mut Table<K, V, H, S>,
// }

// pub struct OccupiedEntry<'a, K, V, H, const S: usize>
// where
//     K: Hash,
//     H: BuildHasher,
// {
//     hash: u64,
//     elem: (K, V),
//     table: &'a mut Table<K, V, H, S>,
// }

#[derive(Debug)]
pub struct Table<K, V, H, const S: usize> {
    hash_builder: H,
    mask: u64,
    table: [Option<(K, V)>; S],
}

fn make_hash<Q, H>(hash_builder: &H, val: &Q) -> u64
where
    Q: Hash,
    H: BuildHasher,
{
    hash_builder.hash_one(val)
}

impl<K, V, H, const S: usize> Table<K, V, H, S>
where
    K: Hash,
    H: BuildHasher,
{
    pub fn with_hasher(hasher: H) -> Self {
        Self {
            hash_builder: hasher,
            mask: S as u64 - 1,
            table: [const { None }; S],
        }
    }

    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let hash = make_hash(&self.hash_builder, &key);
        let index = hash & self.mask;

        dbg!(index);

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::RandomState;

    #[test]
    fn insert() {
        let table =
            Table::<u8, &'static str, RandomState, 10_000>::with_hasher(RandomState::default());

        // dbg!(&table);
        assert_eq!(table.insert(0, "u8::MIN"), None);
        // dbg!(&table);

        assert_eq!(table.insert(255, "u8::MAX_OLD"), None);
        assert_eq!(table.insert(255, "u8::MAX_NEW"), Some("u8::MAX_OLD"));
    }
}
