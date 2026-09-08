use std::{cell::RefCell, rc::Rc};

type Observer<T> = Rc<dyn Fn(&T)>;

/// A cloneable reactive value used by generated component properties.
#[derive(Clone)]
pub struct Property<T> {
    inner: Rc<RefCell<Inner<T>>>,
}

struct Inner<T> {
    value: T,
    observers: Vec<Observer<T>>,
}

impl<T: Clone + 'static> Property<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                value,
                observers: Vec::new(),
            })),
        }
    }

    pub fn get(&self) -> T {
        self.inner.borrow().value.clone()
    }

    pub fn set(&self, value: T) {
        let observers = {
            let mut inner = self.inner.borrow_mut();
            inner.value = value.clone();
            inner.observers.clone()
        };
        for observer in observers {
            observer(&value);
        }
    }

    pub fn observe(&self, observer: impl Fn(&T) + 'static) {
        let observer: Observer<T> = Rc::new(observer);
        self.inner.borrow_mut().observers.push(observer.clone());
        observer(&self.get());
    }
}

impl<T: Default + Clone + 'static> Default for Property<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: std::fmt::Debug + Clone + 'static> std::fmt::Debug for Property<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("Property")
            .field(&self.get())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observers_receive_initial_and_changed_values() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let property = Property::new(1);
        let output = seen.clone();
        property.observe(move |value| output.borrow_mut().push(*value));
        property.set(2);
        assert_eq!(*seen.borrow(), vec![1, 2]);
    }

    #[test]
    fn observer_may_update_another_property() {
        let source = Property::new(1);
        let destination = Property::new(0);
        let output = destination.clone();
        source.observe(move |value| output.set(value * 2));
        source.set(3);
        assert_eq!(destination.get(), 6);
    }
}
