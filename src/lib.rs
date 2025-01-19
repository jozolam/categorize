mod container_enum_dispatch;

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
    use std::str::FromStr;
    use uuid::Uuid;
    use super::*;


    trait ServiceATrait: Send+Sync {
        fn get_uuid(&self) -> Uuid;
    }

    struct ServiceA {
        pub uuid: Uuid,
    }

    impl ServiceATrait for ServiceA {
        fn get_uuid(&self) -> Uuid {
            self.uuid
        }
    }

    struct ServiceAMock {}
    impl ServiceATrait for ServiceAMock {
        fn get_uuid(&self) -> Uuid {
            Uuid::from_str("dccfce5b-726e-43a1-8433-b7c1911b5af4").unwrap()
        }
    }

    struct ServiceWithDirectDependencyOnA {
        pub service_a: Arc<ServiceA>,
    }

    struct ServiceWithTraitDependencyOnA {
        pub service_a: Arc<Box<dyn ServiceATrait>>,
    }

    enum ServiceAEnum {
        ServiceA(ServiceA),
        ServiceAMock(Box<dyn ServiceATrait>),
    }

    impl ServiceAEnum {
        fn get_uuid(&self) -> Uuid {
            match self {
                ServiceAEnum::ServiceA(a) => a.uuid,
                ServiceAEnum::ServiceAMock(a) => a.get_uuid(),
            }
        }
    }

    struct ServiceWithEnumDependencyOnA {
        pub service_a: Arc<ServiceAEnum>,
    }

    fn service_a(c: &mut Container) -> Arc<ServiceA> {
        c.build("service_a", |_container: &mut Container| {
            Arc::from(ServiceA{uuid: Uuid::new_v4()})
        })
    }

    fn service_a_with_trait(c: &mut Container) -> Arc<Box<dyn ServiceATrait>> {
        c.build("service_a_trait", |_container: &mut Container| {
            Arc::from(Box::new(ServiceA{uuid: Uuid::new_v4()}) as Box<dyn ServiceATrait>)
        })
    }

    fn service_a_with_enum(c: &mut Container) -> Arc<ServiceAEnum> {
        c.build("service_a_enum", |_container: &mut Container| {
            Arc::from(ServiceAEnum::ServiceA(ServiceA{uuid: Uuid::new_v4()}))
        })
    }

    fn service_with_direct_dependency_on_a(c: &mut Container) -> Arc<ServiceWithDirectDependencyOnA> {
        c.build("service_with_direct_dependency_on_a", |container: &mut Container| {
            Arc::from(ServiceWithDirectDependencyOnA{service_a: service_a(container)})
        })
    }

    fn service_with_trait_dependency_on_a(c: &mut Container) -> Arc<ServiceWithTraitDependencyOnA> {
        c.build("service_with_trait_dependency_on_a", |container: &mut Container| {
            Arc::from(ServiceWithTraitDependencyOnA{service_a: service_a_with_trait(container)})
        })
    }

    fn service_with_enum_dependency_on_a(c: &mut Container) -> Arc<ServiceWithEnumDependencyOnA> {
        c.build("service_with_trait_dependency_on_a", |container: &mut Container| {
            Arc::from(ServiceWithEnumDependencyOnA{service_a: service_a_with_enum(container)})
        })
    }

    #[test]
    fn fetch_simple_service_from_bottom() {
        let c = &mut Container::new();
        let service_a_instance = service_a(c);
        let service_with_direct_dependency_on_a_instance = service_with_direct_dependency_on_a(c);
        assert_eq!(service_with_direct_dependency_on_a_instance.service_a.uuid, service_a_instance.uuid);
    }

    #[test]
    fn fetch_simple_service_from_top() {
        let c = &mut Container::new();
        let service_with_direct_dependency_on_a_instance = service_with_direct_dependency_on_a(c);
        let service_a_instance = service_a(c);
        assert_eq!(service_with_direct_dependency_on_a_instance.service_a.uuid, service_a_instance.uuid);
    }

    #[test]
    fn set_and_fetch_simple_service() {
        let c = &mut Container::new();
        let service_a_instance = service_a(c);
        c.set("service_a", Arc::new(ServiceA{uuid: Uuid::new_v4()}));
        let service_with_direct_dependency_on_a_instance = service_with_direct_dependency_on_a(c);
        assert_ne!(service_with_direct_dependency_on_a_instance.service_a.uuid, service_a_instance.uuid);
    }

    #[test]
    fn fetch_service_with_trait_dependency_on_a_trait() {
        let c = &mut Container::new();
        let service_a_with_trait = service_a_with_trait(c);
        let service_with_trait_dependency_on_a_instance = service_with_trait_dependency_on_a(c);
        assert_eq!(service_with_trait_dependency_on_a_instance.service_a.get_uuid(), service_a_with_trait.get_uuid());
    }

    #[test]
    fn mock_service_a_with_trait() {
        let c = &mut Container::new();
        let service_a_with_trait = service_a_with_trait(c);
        let service_a_with_trait_mock = Arc::new(Box::new(ServiceAMock{}) as Box<dyn ServiceATrait>);
        c.set("service_a_trait", service_a_with_trait_mock.clone());
        let service_with_trait_dependency_on_a_instance = service_with_trait_dependency_on_a(c);
        assert_eq!(service_with_trait_dependency_on_a_instance.service_a.get_uuid(), service_a_with_trait_mock.get_uuid());
        assert_ne!(service_a_with_trait.get_uuid(), service_a_with_trait_mock.get_uuid());
    }


    #[test]
    fn fetch_service_with_enum_dependency_on_a_enum() {
        let c = &mut Container::new();
        let service_a_with_enum_instance = service_a_with_enum(c);
        let service_with_enum_dependency_on_a_instance = service_with_enum_dependency_on_a(c);
        assert_eq!(service_with_enum_dependency_on_a_instance.service_a.get_uuid(), service_a_with_enum_instance.get_uuid());
    }

    #[test]
    fn mock_service_a_with_enum() {
        let c = &mut Container::new();
        let service_a_with_trait = service_a_with_trait(c);
        let service_a_with_trait_mock = Arc::new(ServiceAEnum::ServiceAMock(Box::from(ServiceAMock{}) as Box<dyn ServiceATrait>));
        c.set("service_a_enum", service_a_with_trait_mock.clone());
        let service_with_trait_dependency_on_a_instance = service_with_enum_dependency_on_a(c);
        assert_eq!(service_with_trait_dependency_on_a_instance.service_a.get_uuid(), service_a_with_trait_mock.get_uuid());
        assert_ne!(service_a_with_trait.get_uuid(), service_a_with_trait_mock.get_uuid());
    }
}
