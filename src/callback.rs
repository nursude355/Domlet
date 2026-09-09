use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type Handler = Rc<RefCell<Box<dyn FnMut()>>>;

/// A replaceable zero-argument callback shared with generated DOM listeners.
#[derive(Clone, Default)]
pub struct Callback {
    handler: Rc<RefCell<Option<Handler>>>,
    invoking: Rc<Cell<bool>>,
}

impl Callback {
    pub fn set(&self, handler: impl FnMut() + 'static) {
        *self.handler.borrow_mut() = Some(Rc::new(RefCell::new(Box::new(handler))));
    }
    pub fn clear(&self) {
        self.handler.borrow_mut().take();
    }
    pub fn invoke(&self) {
        self.try_invoke()
            .expect("recursive callback invocation; use try_invoke to handle recursion explicitly");
    }

    /// Returns an error if this callback is already running, including after replacement.
    pub fn try_invoke(&self) -> Result<(), crate::UpdateCycle> {
        if self.invoking.replace(true) {
            return Err(crate::UpdateCycle);
        }
        struct Guard(Rc<Cell<bool>>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(self.invoking.clone());
        let handler = self.handler.borrow().clone();
        if let Some(handler) = handler {
            (handler.borrow_mut())();
        }
        Ok(())
    }
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Callback")
            .field("connected", &self.handler.borrow().is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recursive_invocation_is_rejected() {
        let callback = Callback::default();
        let nested = callback.clone();
        callback.set(move || assert_eq!(nested.try_invoke(), Err(crate::UpdateCycle)));
        assert_eq!(callback.try_invoke(), Ok(()));
        callback.clear();
    }
    #[test]
    fn handler_can_be_replaced_and_cleared() {
        let count = Rc::new(RefCell::new(0));
        let callback = Callback::default();
        let output = count.clone();
        callback.set(move || *output.borrow_mut() += 1);
        callback.invoke();
        callback.clear();
        callback.invoke();
        assert_eq!(*count.borrow(), 1);
    }

    #[test]
    fn handler_may_replace_itself() {
        let callback = Callback::default();
        let replacement_target = callback.clone();
        callback.set(move || replacement_target.set(|| {}));
        callback.invoke();
        callback.invoke();
    }
}
