use std::collections::HashMap;
use std::sync::Arc;

trait ContainerTrait {
    type Service;
    fn insert(&mut self, key: &str, value: Option<Arc<Self::Service>>);
    fn get(&self, key: &str) -> Option<Option<Arc<Self::Service>>>;
    fn build(
        &mut self,
        name: &str,
        builder: fn(container: &mut Self) -> Self::Service,
    ) -> Arc<Self::Service> {
        match self.get(name) {
            Some(a) => match a {
                Some(i) => i.clone(),
                None => panic!("circular dependency detected for {}", name),
            },
            None => {
                self.insert(name, None);
                let v = Arc::new(builder(self));
                self.insert(name, Some(v.clone()));
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::ops::Deref;
    use std::panic::{catch_unwind};
    use std::str::FromStr;
    use std::sync::RwLock;
    use uuid::Uuid;
    use super::*;


    struct ContainerWithEnumDispatch {
        storage: RwLock<HashMap<String, Option<Arc<ServiceEnum>>>>,
    }

    impl ContainerWithEnumDispatch {
        fn new() -> ContainerWithEnumDispatch {
            ContainerWithEnumDispatch {
                storage: RwLock::new(HashMap::new()),
            }
        }
    }

    impl ContainerTrait for ContainerWithEnumDispatch {
        type Service = ServiceEnum;

        fn insert(&mut self, name: &str, instance: Option<Arc<ServiceEnum>>) {
            self.storage.write().unwrap().insert(name.to_string(), instance);
        }

        fn get(&self, key: &str) -> Option<Option<Arc<ServiceEnum>>> {
            self.storage.read().unwrap().get(key).map(
                |x| x.as_ref().map(|y| y.clone())
            )
        }
    }


    struct ServiceA {
        pub uuid: Uuid,
    }

    trait ServiceATrait: Send+Sync {
        fn get_uuid(&self) -> Uuid;
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

    fn service_a_with_trait(c: &mut ContainerWithEnumDispatch) -> Arc<Box<dyn ServiceATrait>> {
        match c.build("service_a_trait", |_container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceAWithTrait(Arc::new(Box::new(ServiceA{uuid: Uuid::new_v4()}) as Box<dyn ServiceATrait>))
        }).deref() {
            ServiceEnum::ServiceAWithTrait(a) => a.clone(),
            _ => panic!("not a ServiceAEnum"),
        }
    }

    fn service_a_with_enum(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceAEnum> {
        match c.build("service_a_trait", |_container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceAWithEnum(Arc::new(ServiceAEnum::ServiceA(ServiceA{uuid: Uuid::new_v4()})))
        }).deref() {
            ServiceEnum::ServiceAWithEnum(a) => a.clone(),
            _ => panic!("not a ServiceAEnum"),
        }
    }

    fn service_with_direct_dependency_on_a(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceWithDirectDependencyOnA> {
        match c.build("service_with_direct_dependency_on_a", |container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceWithDirectDependencyOnA(Arc::new(ServiceWithDirectDependencyOnA{service_a: service_a(container)}))
        }).deref() {
            ServiceEnum::ServiceWithDirectDependencyOnA(a) => a.clone(),
            _ => panic!("not a ServiceWithDirectDependencyOnA"),
        }
    }

    fn service_with_trait_dependency_on_a(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceWithTraitDependencyOnA> {
        match c.build("service_with_trait_dependency_on_a", |container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceWithTraitDependencyOnA(Arc::new(ServiceWithTraitDependencyOnA{service_a: service_a_with_trait(container)}))
        }).deref() {
            ServiceEnum::ServiceWithTraitDependencyOnA(a) => a.clone(),
            _ => panic!("not a ServiceWithTraitDependencyOnA"),
        }
    }

    fn service_with_enum_dependency_on_a(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceWithEnumDependencyOnA> {
        match c.build("service_with_trait_dependency_on_a", |container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceWithEnumDependencyOnA(Arc::new(ServiceWithEnumDependencyOnA{service_a: service_a_with_enum(container)}))
        }).deref() {
            ServiceEnum::ServiceWithEnumDependencyOnA(a) => a.clone(),
            _ => panic!("not a ServiceWithEnumDependencyOnA"),
        }
    }

    struct ServiceB {
        service_a: Arc<ServiceA>,
    }


    struct CircularA {
        // circular_b: Arc<CircularB>
    }

    #[derive(Debug)]
    struct CircularB {
        // circular_a: Arc<CircularA>
    }

    enum ServiceEnum {
        ServiceA(Arc<ServiceA>),
        ServiceAWithTrait(Arc<Box<dyn ServiceATrait>>),
        ServiceAWithEnum(Arc<ServiceAEnum>),
        ServiceB(Arc<ServiceB>),
        ServiceWithDirectDependencyOnA(Arc<ServiceWithDirectDependencyOnA>),
        ServiceWithTraitDependencyOnA(Arc<ServiceWithTraitDependencyOnA>),
        ServiceWithEnumDependencyOnA(Arc<ServiceWithEnumDependencyOnA>),
        CircularA(Arc<CircularA>),
        CircularB(Arc<CircularB>),
    }

    fn service_a(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceA> {
        match c.build("service_a", |_container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceA(Arc::new(ServiceA{uuid: Uuid::new_v4()}))
        }).deref() {
            ServiceEnum::ServiceA(a)=> Arc::clone(&a),
            _ => panic!("Not a ServiceA"),
        }
    }

    fn service_b(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceB> {
        match c.build("service_b", |container: &mut ContainerWithEnumDispatch| -> ServiceEnum {
            ServiceEnum::ServiceB(Arc::new(ServiceB{service_a: service_a(container)}))
    }).deref() {
            ServiceEnum::ServiceB(a) => Arc::clone(&a),
            _ => panic!("Not a ServiceB"),
        }
    }

    fn circular_a(c: &mut ContainerWithEnumDispatch) -> Arc<CircularA> {
        match c.build("circular_a", |container: &mut ContainerWithEnumDispatch| -> ServiceEnum {
            circular_b(container);
            ServiceEnum::CircularA(Arc::new(CircularA{}))
        }).deref() {
            ServiceEnum::CircularA(a) => Arc::clone(&a),
            _ => panic!("Not a CircularA"),
        }
    }

    fn circular_b(c: &mut ContainerWithEnumDispatch) -> Arc<CircularB> {
        match c.build("circular_b", |container: &mut ContainerWithEnumDispatch| -> ServiceEnum {
            circular_a(container);
            ServiceEnum::CircularB(Arc::new(CircularB{}))
        }).deref() {
            ServiceEnum::CircularB(a) => Arc::clone(&a),
            _ => panic!("Not a CircularB"),
        }
    }

    #[test]
    fn fetch_simple_service_from_bottom() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_instance = service_a(c);
        let service_b_instance = service_b(c);
        assert_eq!(service_b_instance.service_a.uuid, service_a_instance.uuid);
    }

    #[test]
    fn fetch_simple_service_from_top() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_with_direct_dependency_on_a_instance = service_with_direct_dependency_on_a(c);
        let service_a_instance = service_a(c);
        assert_eq!(service_with_direct_dependency_on_a_instance.service_a.uuid, service_a_instance.uuid);
    }

    #[test]
    fn set_and_fetch_simple_service() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_instance = service_a(c);
        c.insert("service_a", Some(Arc::new(ServiceEnum::ServiceA(Arc::new(ServiceA{uuid: Uuid::new_v4()})))));
        let service_with_direct_dependency_on_a_instance = service_with_direct_dependency_on_a(c);
        assert_ne!(service_with_direct_dependency_on_a_instance.service_a.uuid, service_a_instance.uuid);
    }

    #[test]
    fn fetch_service_with_trait_dependency_on_a_trait() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_with_trait = service_a_with_trait(c);
        let service_with_trait_dependency_on_a_instance = service_with_trait_dependency_on_a(c);
        assert_eq!(service_with_trait_dependency_on_a_instance.service_a.get_uuid(), service_a_with_trait.get_uuid());
    }

    #[test]
    fn mock_service_a_with_trait() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_with_trait = service_a_with_trait(c);
        let service_a_with_trait_mock = Arc::new(Box::new(ServiceAMock {}) as Box<dyn ServiceATrait>);
        c.insert("service_a_trait", Some(Arc::new(ServiceEnum::ServiceAWithTrait(service_a_with_trait_mock.clone()))));
        let service_with_trait_dependency_on_a_instance = service_with_trait_dependency_on_a(c);
        assert_eq!(service_with_trait_dependency_on_a_instance.service_a.get_uuid(), service_a_with_trait_mock.get_uuid());
        assert_ne!(service_a_with_trait.get_uuid(), service_a_with_trait_mock.get_uuid());
    }


    #[test]
    fn fetch_service_with_enum_dependency_on_a_enum() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_with_enum_instance = service_a_with_enum(c);
        let service_with_enum_dependency_on_a_instance = service_with_enum_dependency_on_a(c);
        assert_eq!(service_with_enum_dependency_on_a_instance.service_a.get_uuid(), service_a_with_enum_instance.get_uuid());
    }

    #[test]
    fn mock_service_a_with_enum() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_with_trait = service_a_with_trait(c);
        c.insert("service_a_trait", Some(Arc::new(ServiceEnum::ServiceAWithEnum(Arc::new(ServiceAEnum::ServiceAMock(Box::new(ServiceAMock {}) as Box<dyn ServiceATrait>))))));
        let service_with_trait_dependency_on_a_instance = service_with_enum_dependency_on_a(c);
        assert_eq!(service_with_trait_dependency_on_a_instance.service_a.get_uuid(), ServiceAMock{}.get_uuid());
        assert_ne!(service_a_with_trait.get_uuid(), ServiceAMock{}.get_uuid());
    }

    #[test]
    fn circular_dependency_panics() {
         fn get_panic_message(payload: &(dyn Any + Send)) -> Option<&str> {
                // taken from: https://github.com/rust-lang/rust/blob/4b9f4b221b92193c7e95b1beb502c6eb32c3b613/library/std/src/panicking.rs#L194-L200
                match payload.downcast_ref::<&'static str>() {
                    Some(msg) => Some(*msg),
                    None => match payload.downcast_ref::<String>() {
                        Some(msg) => Some(msg.as_str()),
                        None => None,
                    },
                }
         }


        let payload = catch_unwind(|| {
            let c = &mut ContainerWithEnumDispatch::new();
            circular_b(c)
        }).unwrap_err();

        assert_eq!(get_panic_message(payload.as_ref()).unwrap(), "circular dependency detected for circular_b");
    }
}
