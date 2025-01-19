use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

struct Container {
    storage: HashMap<String, Option<Arc<dyn Any + Send + Sync>>>,
}

impl Container {
    fn new() -> Container {
        Container {
            storage: HashMap::new(),
        }
    }

    fn set(&mut self, name: &str, instance: Arc<dyn Any + Send + Sync>) {
        self.storage.insert(name.to_string(), Some(instance));
    }

    fn build<T: 'static + Send + Sync>(
        &mut self,
        name: &str,
        builder: fn(container: &mut Container) -> Arc<T>,
    ) -> Arc<T> {
        match self.storage.get(name) {
            Some(a) => match a {
                Some(i) => i.clone().downcast::<T>().unwrap(),
                None => panic!("circular reference"),
            },
            None => {
                self.storage.insert(name.to_string(), None);
                let v = Arc::from(builder(self));
                self.storage
                    .insert(name.to_string(), Some(Arc::clone(&v) as Arc<dyn Any + Send + Sync>));
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Example1Impl {
        pub test: String,
    }

    struct Example11Impl {}

    trait Example1ImpTrait:Send+Sync {
        fn hello(&self) -> String;
    }

    impl Example1ImpTrait for Example1Impl {
        fn hello(&self) -> String {
            self.test.clone()
        }
    }

    impl Example1ImpTrait for Example11Impl {
        fn hello(&self) -> String {
            "2".to_string()
        }
    }

    struct Example2Impl {
        pub example1impl: Arc<dyn Example1ImpTrait>, // will be wired with trait and this can be used for mocking
    }

    impl Example1Impl {
        pub fn new(text: String) -> Example1Impl {
            Example1Impl { test: text }
        }
    }

    impl Example2Impl {
        fn new(example1impl: Arc<dyn Example1ImpTrait>) -> Example2Impl {
            Example2Impl { example1impl }
        }
    }

    fn get_example1(c: &mut Container) -> Arc<Example1Impl> {
        c.build("example1", |_container: &mut Container| {
            Arc::from(Example1Impl::new("default".to_string()))
        })
    }

    fn get_example2(c: &mut Container) -> Arc<Example2Impl> {
        c.build("example2", |container: &mut Container| {
            Arc::from(Example2Impl::new(get_example1(container)))
        })
    }

    #[test]
    fn hello() {
        let mut c = Container::new();
        let s1 = get_example1(&mut c);
        assert_eq!("default".to_string(), s1.test);
        let mut c = Container::new();
        c.set("example1", Arc::from(Example1Impl::new("ahoj".to_string())));
        let s1 = get_example1(&mut c);
        let s2 = get_example2(&mut c);
        assert_eq!("ahoj".to_string(), s1.test);
        assert_eq!("ahoj".to_string(), s2.example1impl.hello());

        let mut c = Container::new();
        c.set(
            "example2",
            Arc::from(Example2Impl::new(Arc::from(Example11Impl {}))),
        );
        let s2 = get_example2(&mut c);
        assert_eq!("2".to_string(), s2.example1impl.hello());
    }
}
