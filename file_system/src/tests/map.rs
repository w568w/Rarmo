use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use parking_lot::RwLock;
// pub struct LockedMap<K: Eq + Hash, V>(RwLock<HashMap<K, V>>);

// impl<K: Eq + Hash, V> LockedMap<K, V> {
//     pub fn new() -> Self {
//         Self(RwLock::new(HashMap::new()))
//     }
//
//     pub fn add(&self, key: K, value: V) {
//         let mut lock = self.0.write();
//         if let Err(_) = lock.try_insert(key, value){
//             panic!("Key already exists");
//         }
//     }
//
//     pub fn try_add(&self, key: K, value: V) {
//         let mut lock = self.0.write();
//         lock.insert(key, value);
//     }
//
//     pub fn contains(&self, key: &K) -> bool {
//         let lock = self.0.read();
//         lock.contains_key(key)
//     }
// }
//
// impl<K: Eq + Hash, V> Index<K> for LockedMap<K, V> {
//     type Output = V;
//
//     fn index(&self, key: K) -> &Self::Output {
//         let lock = self.0.read();
//         lock.index(&key)
//     }
// }
//
// impl<K: Eq + Hash, V> IndexMut<K> for LockedMap<K, V> {
//     fn index_mut(&mut self, key: K) -> &mut Self::Output {
//         let mut lock = self.0.write();
//         lock.get_mut(&key).unwrap()
//     }
// }