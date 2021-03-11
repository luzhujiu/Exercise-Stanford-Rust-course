use std::fmt;
use std::option::Option;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {value: value, next: next}
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {head: None, size: 0}
    }
    
    pub fn get_size(&self) -> usize {
        self.size
    }
    
    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }
    
    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}

impl<T: fmt::Display> fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                },
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut values = vec![];
        loop {
            match current {
                Some(node) => {
                    let value = values.push(node.value.clone());
                    current = &node.next;
                },
                None => break,
            }
        }
        values.reverse();
        let mut new_list = LinkedList::new();
        for value in values {
            new_list.push_front(value);
        }
        return new_list;
    }
}

impl<T: PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size == other.size {
            let mut current: &Option<Box<Node<T>>> = &self.head;
            let mut current2: &Option<Box<Node<T>>> = &other.head;
            for _ in 0..self.size {
               let node = current.as_ref().unwrap();
               let node2 = current2.as_ref().unwrap();
               if node.value != node2.value {
                   return false;
               } else {
                   current = &node.next;
                   current2 = &node2.next;
               }
            }
            return true;
        } else {
            return false;
        }
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

impl<T: Clone> Iterator for LinkedList<T> {
    type Item = T;
    
    fn next(&mut self) -> Option<T> {
        self.pop_front()
    }
}




