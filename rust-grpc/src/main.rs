use std::ptr;

struct Node<T> {
    data: T,
    next: *mut Node<T>,
    prev: *mut Node<T>,
}

pub struct DoublyLinkedList<T> {
    head: *mut Node<T>,
    tail: *mut Node<T>,
    len: usize,
}

impl<T> DoublyLinkedList<T> {
    pub fn new() -> Self {
        DoublyLinkedList {
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            len: 0,
        }
    }

    pub fn push_back(&mut self, data: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data,
            next: ptr::null_mut(),
            prev: self.tail,
        }));

        unsafe {
            if !self.tail.is_null() {
                (*self.tail).next = new_node;
            } else {
                self.head = new_node;
            }
        }

        self.tail = new_node;
        self.len += 1;
    }

    pub fn push_front(&mut self, data: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data,
            next: self.head,
            prev: ptr::null_mut(),
        }));

        unsafe {
            if !self.head.is_null() {
                (*self.head).prev = new_node;
            } else {
                self.tail = new_node;
            }
        }

        self.head = new_node;
        self.len += 1;
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.tail.is_null() {
            return None;
        }

        unsafe {
            let tail_box = Box::from_raw(self.tail);
            self.tail = tail_box.prev;

            if !self.tail.is_null() {
                (*self.tail).next = ptr::null_mut();
            } else {
                self.head = ptr::null_mut();
            }

            self.len -= 1;
            Some(tail_box.data)
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> Drop for DoublyLinkedList<T> {
    fn drop(&mut self) {
        while self.pop_back().is_some() {}
    }
}

fn main() {
    let mut list = DoublyLinkedList::new();
    list.push_back(10);
    list.push_back(20);
    list.push_front(5);

    println!("Tamanho da lista: {}", list.len()); // Saída: 3

    while let Some(val) = list.pop_back() {
        println!("Removido: {}", val);
    }
    println!("Worker gRPC em Rust inicializado.");
}
