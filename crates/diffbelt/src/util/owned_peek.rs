pub struct OwnedPeek<T, I: Iterator<Item = T>> {
    first_value: Option<T>,
    iterator: I,
}

impl<T, I: Iterator<Item = T>> OwnedPeek<T, I> {
    pub fn new(iterator: I) -> Self {
        OwnedPeek {
            first_value: None,
            iterator,
        }
    }

    pub fn peek<R>(&mut self, fun: impl FnOnce(Option<T>) -> (R, Option<T>)) -> R {
        let next_item = match self.first_value.take() {
            None => self.iterator.next(),
            some => some,
        };

        let (result, value) = fun(next_item);

        match value {
            Some(value) => {
                self.first_value.replace(value);
            }
            None => {}
        }

        result
    }

    pub fn is_empty(&mut self) -> bool {
        match &self.first_value {
            Some(_) => false,
            None => {
                let value = self.iterator.next();

                match value {
                    Some(value) => {
                        self.first_value.replace(value);
                        false
                    }
                    None => true,
                }
            }
        }
    }
}

impl<T, I: Iterator<Item = T>> Iterator for OwnedPeek<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.first_value.take() {
            None => self.iterator.next(),
            some => some,
        }
    }
}
