use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

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

struct ServiceB {
    service_a: Arc<ServiceA>,
}

enum ServiceEnum {
    ServiceA(Arc<ServiceA>),
    ServiceB(Arc<ServiceB>),
}

struct ContainerWithEnumDispatch {
    storage: HashMap<String, Option<Arc<ServiceEnum>>>,
}

impl ContainerWithEnumDispatch {
    fn new() -> ContainerWithEnumDispatch {
        ContainerWithEnumDispatch {
            storage: HashMap::new(),
        }
    }

    fn set(&mut self, name: &str, instance: Arc<ServiceEnum>) {
        self.storage.insert(name.to_string(), Some(instance));
    }

    fn build(
        &mut self,
        name: &str,
        builder: fn(container: &mut ContainerWithEnumDispatch) -> ServiceEnum,
    ) -> Arc<ServiceEnum> {
        match self.storage.get(name) {
            Some(a) => match a {
                Some(i) => i.clone(),
                None => panic!("circular reference"),
            },
            None => {
                self.storage.insert(name.to_string(), None);
                let v = Arc::from(builder(self));
                self.storage.insert(name.to_string(), Some(v.clone()));
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;
    use uuid::Uuid;
    use super::*;

    fn service_a(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceA> {
        match c.build("service_a", |_container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceA(Arc::from(ServiceA{uuid: Uuid::new_v4()}))
        }).deref() {
            ServiceEnum::ServiceA(a)=> Arc::clone(&a),
            _ => panic!("Not a ServiceA"),
        }
    }

    fn service_b(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceB> {
        match c.build("service_b", |container: &mut ContainerWithEnumDispatch| {
            ServiceEnum::ServiceB(Arc::from(ServiceB{service_a: service_a(container)}))
        }).deref() {
            ServiceEnum::ServiceB(a) => Arc::clone(&a),
            _ => panic!("Not a ServiceB"),
        }
    }


    #[test]
    fn fetch_simple_service_from_bottom() {
        let c = &mut ContainerWithEnumDispatch::new();
        let service_a_instance = service_a(c);
        let service_b_instance = service_b(c);
        assert_eq!(service_b_instance.service_a.uuid, service_a_instance.uuid);
    }
}
