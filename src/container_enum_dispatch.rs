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

trait ContainerTrait {
    type Service;

    fn insert(&mut self, key: String, value: Option<Arc<Self::Service>>);
    fn get(&self, key: &str) -> Option<&Option<Arc<Self::Service>>>;

    fn build(
        &mut self,
        name: &str,
        builder: fn(container: &mut Self) -> Self::Service,
    ) -> Arc<Self::Service> {
        match self.get(name) {
            Some(a) => match a {
                Some(i) => i.clone(),
                None => panic!("circular reference"),
            },
            None => {
                self.insert(name.to_string(), None);
                let v = Arc::from(builder(self));
                self.insert(name.to_string(), Some(v.clone()));
                v
            }
        }
    }

}

impl ContainerWithEnumDispatch {
    fn new() -> ContainerWithEnumDispatch {
        ContainerWithEnumDispatch {
            storage: HashMap::new(),
        }
    }
}

impl ContainerTrait for ContainerWithEnumDispatch {
    type Service = ServiceEnum;

    fn insert(&mut self, name: String, instance: Option<Arc<ServiceEnum>>) {
        self.storage.insert(name.to_string(), instance);
    }

    fn get(&self, key: &str) -> Option<&Option<Arc<ServiceEnum>>> {
        self.storage.get(key)
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

    // fn tratsaf (container: &mut impl ContainerTrait) -> ServiceEnum {
    //     ServiceEnum::ServiceB(Arc::from(ServiceB{service_a: service_a(container)}))
    // }

    fn service_b(c: &mut ContainerWithEnumDispatch) -> Arc<ServiceB> {
        match c.build("service_b", |container: &mut ContainerWithEnumDispatch| -> ServiceEnum {
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
