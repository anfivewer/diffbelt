use std::collections::VecDeque;

pub trait IndexedContainerPointer {
    fn index(&self) -> usize;
    fn counter(&self) -> u64;
}

pub trait IndexedContainerItem {
    type Item: IndexedContainerPointer;
    type Id: IndexedContainerPointer + Copy;

    fn new_id(index: usize, counter: u64) -> Self::Id;
}

pub struct IndexedContainer<T: IndexedContainerItem> {
    array: Vec<Option<T::Item>>,
    free_slots: VecDeque<usize>,
    counter: u64,
}

impl<T: IndexedContainerItem> IndexedContainer<T> {
    pub fn new() -> Self {
        Self {
            array: Vec::new(),
            free_slots: VecDeque::new(),
            counter: 0,
        }
    }

    pub fn insert<F: FnOnce(T::Id) -> T::Item>(&mut self, create: F) -> T::Id {
        self.counter += 1;
        let counter = self.counter;

        if let Some(index) = self.free_slots.pop_front() {
            let id = T::new_id(index, counter);
            let value = create(id);
            self.array[index] = Some(value);

            return id;
        }

        let index = self.array.len();
        let id = T::new_id(index, counter);
        let value = create(id);
        self.array.push(Some(value));

        id
    }

    pub fn get(&self, id: &T::Id) -> Option<&T::Item> {
        let Some(entry) = self.array.get(id.index()) else {
            return None;
        };

        let Some(item) = entry else {
            return None;
        };

        if item.counter() != id.counter() {
            return None;
        }

        Some(item)
    }

    pub fn get_mut(&mut self, id: &T::Id) -> Option<&mut T::Item> {
        let Some(entry) = self.array.get_mut(id.index()) else {
            return None;
        };

        let Some(item) = entry else {
            return None;
        };

        if item.counter() != id.counter() {
            return None;
        }

        Some(item)
    }

    pub fn delete(&mut self, id: &T::Id) {
        let Some(entry) = self.array.get_mut(id.index()) else {
            return;
        };

        let Some(item) = entry else {
            return;
        };

        if item.counter() != id.counter() {
            return;
        }

        entry.take();
    }
}
