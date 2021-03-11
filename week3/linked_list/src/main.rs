use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<u32> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i);
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string()); // ToString impl for anything impl Display
    
    let mut clone: LinkedList<u32> = list.clone();
    println!("orginal = {}", list);
    println!("clone = {}", clone);
    
    // If you implement iterator trait:
    //for val in &list {
    //    println!("{}", val);
    //}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_eq() {
        let mut list: LinkedList<u32> = LinkedList::new();
        for i in 1..12 {
            list.push_front(i);
        }

        let clone = list.clone();

        assert!(list == clone);

        list.pop_front();

        assert!(list != clone);
    }

    #[test]
    fn test_iterator() {
        let mut list: LinkedList<u32> = LinkedList::new();
        assert_eq!(list.get_size(), 0);
        for i in 1..12 {
            list.push_front(i);
        }
        let mut ans = vec![];
        for val in list {
            ans.push(val);
        }
        ans.reverse();
        assert!((1..12).collect::<Vec<u32>>() == ans);
    }
}

