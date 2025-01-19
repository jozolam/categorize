// use std::any::Any;
// use std::collections::HashMap;
// use std::sync::Arc;
//
// struct Container {
//     element_type type,
//     storage: HashMap<String, Option<Arc<dyn Any + Send + Sync>>>,
// }
//
// impl crate::Container {
//     fn new() -> crate::Container {
//         crate::Container {
//             storage: HashMap::new(),
//         }
//     }
//
//     fn set(&mut self, name: &str, instance: Arc<dyn Any + Send + Sync>) {
//         self.storage.insert(name.to_string(), Some(instance));
//     }
//
//     fn build<T: 'static + Send + Sync>(
//         &mut self,
//         name: &str,
//         builder: fn(container: &mut crate::Container) -> Arc<T>,
//     ) -> Arc<T> {
//         match self.storage.get(name) {
//             Some(a) => match a {
//                 Some(i) => i.clone().downcast::<T>().unwrap(),
//                 None => panic!("circular reference"),
//             },
//             None => {
//                 self.storage.insert(name.to_string(), None);
//                 let v = Arc::from(builder(self));
//                 self.storage
//                     .insert(name.to_string(), Some(Arc::clone(&v) as Arc<dyn Any + Send + Sync>));
//                 v
//             }
//         }
//     }
// }