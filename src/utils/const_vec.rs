#[derive(Debug, Clone, Copy)]
pub struct ConstVec<const S: usize, T> {
    data: [T; S],
    len: usize,
}

impl<const S: usize, T> ConstVec<S, T> {
    pub fn push(&mut self, item: T) {
        if self.len == S {
            panic!("the const vector size limit is exceeded")
        }
        let _ = std::mem::replace(&mut self.data[self.len], item);
        self.len += 1;
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[0..self.len]
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data[0..self.len].iter()
    }

    pub fn map<B>(self, mut f: impl FnMut(T) -> B) -> ConstVec<S, B> {
        let mut v = ConstVec::default();
        for (i, item) in self.data.into_iter().enumerate() {
            if self.len == i {
                break;
            }
            v.push(f(item));
        }
        v
    }
}

impl<const S: usize, T> Default for ConstVec<S, T> {
    fn default() -> Self {
        ConstVec {
            #[allow(clippy::uninit_assumed_init)]
            data: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }
}

impl<const S: usize, T> FromIterator<T> for ConstVec<S, T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut v = ConstVec::default();
        for item in iter.into_iter() {
            v.push(item);
        }
        v
    }
}

#[macro_export]
macro_rules! const_vec {
    () => (ConstVec::default());
    ($elem:expr; $n:expr) => (
        {
            let mut v = ConstVec::default();
            for _ in 0..$n {
                v.push($elem);
            }
            v
        }
    );
    ($($x:expr),+ $(,)?) => (
        {
            let mut v = ConstVec::default();
            $( v.push($x) ; )+
            v
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_vec_default() {
        _ = ConstVec::<10, u32>::default();
    }

    #[test]
    fn const_vec_from_vec() {
        let test_vec: [u32; 3] = [1, 2, 3];
        let _ = ConstVec::<5, u32>::from_iter(test_vec);
    }

    #[test]
    #[should_panic]
    fn const_vec_from_vec_panic() {
        let test_vec: [u32; 3] = [1, 2, 3];
        let _ = ConstVec::<2, u32>::from_iter(test_vec);
    }

    #[test]
    fn const_vec_push() {
        let mut cv = ConstVec::<1, u32>::default();
        cv.push(1);
    }

    #[test]
    #[should_panic]
    fn const_vec_push_panic() {
        let mut cv = ConstVec::<1, u32>::default();
        cv.push(1);
        cv.push(2);
    }

    #[test]
    fn const_vec_as_slice() {
        let mut cv = ConstVec::<1, u32>::default();
        cv.push(1);
        assert_eq!(&[1], cv.as_slice());
    }

    #[test]
    fn const_vec_iter() {
        let mut cv = ConstVec::<1, u32>::default();
        cv.push(1);

        let from_iter = cv.iter().copied().collect::<Vec<u32>>();
        assert_eq!(&from_iter, &[1]);
    }
}
