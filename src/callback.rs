use std::{cell::RefCell, rc::Rc};

type Handler = Rc<RefCell<Box<dyn FnMut()>>>;

/// A replaceable zero-argument callback shared with generated DOM listeners.
#[derive(Clone, Default)]
pub struct Callback {
    handler: Rc<RefCell<Option<Handler>>>,
}

impl Callback {
    pub fn set(&self, handler: impl FnMut() + 'static) {
        *self.handler.borrow_mut() = Some(Rc::new(RefCell::new(Box::new(handler))));
    }
    pub fn clear(&self) {
        self.handler.borrow_mut().take();
    }
    pub fn invoke(&self) {
        let handler = self.handler.borrow().clone();
        if let Some(handler) = handler {
            (handler.borrow_mut())();
        }
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
