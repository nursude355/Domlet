use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

type Observer<T> = Rc<dyn Fn(&T)>;
type WeakObserver<T> = Weak<dyn Fn(&T)>;

/// Keeps an observer registered. Dropping it releases the observer and its captures.
#[must_use = "keep the subscription alive to receive updates"]
pub struct Subscription {
    _observer: Box<dyn std::any::Any>,
}

/// An observer attempted to update a property already being notified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateCycle;

impl std::fmt::Display for UpdateCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cyclic property update")
    }
}
impl std::error::Error for UpdateCycle {}

struct NotifyGuard(Rc<Cell<bool>>);
impl Drop for NotifyGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A cloneable reactive value used by generated component properties.
#[derive(Clone)]
pub struct Property<T> {
    inner: Rc<RefCell<Inner<T>>>,
    notifying: Rc<Cell<bool>>,
}

struct Inner<T> {
    value: T,
    observers: Vec<WeakObserver<T>>,
}

impl<T: Clone + 'static> Property<T> {
    pub fn new(value: T) -> Self {
        Self {
            notifying: Rc::new(Cell::new(false)),
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
        self.try_set(value)
            .expect("cyclic property update; use try_set to handle feedback explicitly");
    }

    /// Updates the value only when it differs from the current value.
    ///
    /// Returns `true` when observers were notified.
    pub fn set_if_changed(&self, value: T) -> bool
    where
        T: PartialEq,
    {
        self.try_set_if_changed(value)
            .expect("cyclic property update; use try_set_if_changed to handle feedback explicitly")
    }

    /// Rejects recursive feedback before modifying the value.
    pub fn try_set(&self, value: T) -> Result<(), UpdateCycle> {
        if self.notifying.replace(true) {
            return Err(UpdateCycle);
        }
        let _guard = NotifyGuard(self.notifying.clone());
        let observers = {
            let mut inner = self.inner.borrow_mut();
            inner.value = value.clone();
            inner
                .observers
                .retain(|observer| observer.strong_count() > 0);
            inner.observers.clone()
        };
        for observer in observers {
            if let Some(observer) = observer.upgrade() {
                observer(&value);
            }
        }
        Ok(())
    }

    /// Fallible variant of [`Self::set_if_changed`].
    ///
    /// A no-op succeeds even while the property is notifying. A changed value
    /// still returns [`UpdateCycle`] in that situation.
    pub fn try_set_if_changed(&self, value: T) -> Result<bool, UpdateCycle>
    where
        T: PartialEq,
    {
        if self.inner.borrow().value == value {
            return Ok(false);
        }
        self.try_set(value)?;
        Ok(true)
    }

    pub fn observe(&self, observer: impl Fn(&T) + 'static) -> Subscription {
        let observer: Observer<T> = Rc::new(observer);
        // Initial delivery precedes registration so it cannot call itself.
        observer(&self.get());
        let mut inner = self.inner.borrow_mut();
        inner
            .observers
            .retain(|observer| observer.strong_count() > 0);
        inner.observers.push(Rc::downgrade(&observer));
        Subscription {
            _observer: Box::new(observer),
        }
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
    fn dropping_subscription_releases_captures() {
        let property = Property::new(0);
        let captured = Rc::new(());
        let weak = Rc::downgrade(&captured);
        let subscription = property.observe(move |_| {
            let _ = &captured;
        });
        assert!(weak.upgrade().is_some());
        drop(subscription);
        assert!(weak.upgrade().is_none());
        property.set(1);
    }

    #[test]
    fn recursive_update_is_rejected_and_later_updates_work() {
        let property = Property::new(0);
        let other = property.clone();
        let subscription = property.observe(move |value| {
            if *value > 0 {
                assert_eq!(other.try_set(99), Err(UpdateCycle));
            }
        });
        property.set(1);
        assert_eq!(property.get(), 1);
        drop(subscription);
        assert_eq!(property.try_set(2), Ok(()));
    }
    #[test]
    fn observers_receive_initial_and_changed_values() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let property = Property::new(1);
        let output = seen.clone();
        let _subscription = property.observe(move |value| output.borrow_mut().push(*value));
        property.set(2);
        assert_eq!(*seen.borrow(), vec![1, 2]);
    }

    #[test]
    fn set_if_changed_skips_equal_values() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let property = Property::new(1);
        let output = seen.clone();
        let _subscription = property.observe(move |value| output.borrow_mut().push(*value));

        assert!(!property.set_if_changed(1));
        assert!(property.set_if_changed(2));
        assert_eq!(*seen.borrow(), vec![1, 2]);
    }

    #[test]
    fn try_set_if_changed_distinguishes_noop_while_notifying() {
        let property = Property::new(0);
        let recursive = property.clone();
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let output = outcomes.clone();
        let _subscription = property.observe(move |value| {
            if *value == 1 {
                output.borrow_mut().push(recursive.try_set_if_changed(1));
                output.borrow_mut().push(recursive.try_set_if_changed(2));
            }
        });

        property.set(1);

        assert_eq!(*outcomes.borrow(), vec![Ok(false), Err(UpdateCycle)]);
        assert_eq!(property.get(), 1);
    }

    #[test]
    fn observer_may_update_another_property() {
        let source = Property::new(1);
        let destination = Property::new(0);
        let output = destination.clone();
        let _subscription = source.observe(move |value| output.set(value * 2));
        source.set(3);
        assert_eq!(destination.get(), 6);
    }
}
