use crate::errors::Error;

pub trait ReentrancyGuard {
    fn is_locked(&self) -> bool;
    fn set_locked(&mut self, locked: bool);

    fn lock(&mut self) -> Result<(), Error> {
        if self.is_locked() {
            return Err(Error::ReentrancyDetected);
        }
        self.set_locked(true);
        Ok(())
    }

    fn unlock(&mut self) {
        self.set_locked(false);
    }
}
