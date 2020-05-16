pub trait Consume {
    fn consume(self) -> ();
}

impl<T> Consume for T {
    fn consume(self) {
        let _ = self;
    }
}