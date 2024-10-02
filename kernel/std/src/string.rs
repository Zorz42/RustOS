use core::fmt::{Display, Formatter, Write};
use crate as std;
use crate::vector::{VecIntoIterator, VecIterator};
use crate::Vec;
use core::ops::{Index, IndexMut};

#[derive(derive::Serial, Default, PartialEq, Clone)]
pub struct String {
    vec: Vec::<char>,
}

impl String {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn from(s: &str) -> Self {
        let mut res = Self::new();
        for c in s.chars() {
            res.push(c);
        }
        res
    }

    pub unsafe fn get_unchecked(&self, i: usize) -> char {
        *self.vec.get_unchecked(i)
    }

    pub unsafe fn get_mut_unchecked(&mut self, i: usize) -> &mut char {
        self.vec.get_mut_unchecked(i)
    }

    pub fn get(&self, i: usize) -> Option<char> {
        self.vec.get(i).map(|x| *x)
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut char> {
        self.vec.get_mut(i)
    }

    pub fn reserve(&mut self, size: usize) {
        self.vec.reserve(size);
    }

    pub fn push(&mut self, element: char) {
        self.vec.push(element);
    }

    pub fn size(&self) -> usize {
        self.vec.size()
    }

    pub fn pop(&mut self) {
        self.vec.pop();
    }

    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    pub fn split(&self, c: char) -> Vec<String> {
        let mut curr = String::new();
        let mut res = Vec::new();
        for i in self {
            if *i == c {
                res.push(curr.clone());
                curr = String::new();
            } else {
                curr.push(*i);
            }
        }
        res.push(curr);
        res
    }
    
    pub fn as_str(&self) -> &str {
        unsafe {
            //core::str::from_utf8(core::slice::from_raw_parts((self.vec.get_unchecked(0) as *const char) as *const u8, self.vec.size())).unwrap()
            let mut data = Vec::new();
            for c in self {
                for i in c.encode_utf8(&mut [0; 4]).bytes() {
                    data.push(i);
                }
            }
            core::str::from_utf8(core::slice::from_raw_parts(data.get_unchecked(0) as *const u8, self.vec.size())).unwrap()
        }
    }
}

impl Index<usize> for String {
    type Output = char;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vec[index]
    }
}

impl IndexMut<usize> for String {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.vec[index]
    }
}

impl IntoIterator for String {
    type Item = char;
    type IntoIter = VecIntoIterator<char>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}

impl<'a> IntoIterator for &'a String {
    type Item = &'a char;
    type IntoIter = VecIterator<'a, char>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.vec).into_iter()
    }
}

impl Display for String {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        for c in self {
            f.write_char(*c)?;
        }
        Ok(())
    }
}
